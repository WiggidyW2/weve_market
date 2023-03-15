use crate::{
    {LocationId, RegionId, TypeId},
    json::*,
    time,
};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::stream::{TryStreamExt, futures_unordered::FuturesUnordered};
use reqwest::{self, header::{self, HeaderValue, HeaderMap}};
use chrono::DateTime;
use base64;

const ADJUSTED_PRICE_URL: &str = "https://esi.evetech.net/latest/markets/prices/";
const SYSTEM_INDEX_URL: &str = "https://esi.evetech.net/latest/industry/systems/";
const AUTH_URL: &str = "https://login.eveonline.com/v2/oauth/token";
const HOST_URL: &str = "login.eveonline.com";
const ORDERS_PER_PAGE: usize = 1000;

fn station_order_url(region_id: &RegionId) -> String {
    format!(
        "https://esi.evetech.net/latest/markets/{}/orders/",
        region_id,
    )
}

fn structure_order_url(location_id: &LocationId) -> String {
    format!(
        "https://esi.evetech.net/latest/markets/structures/{}/",
        location_id,
    )
}

#[derive(Debug)]
pub enum Error {
    AuthenticationStatusCode(reqwest::StatusCode),
    JsonParseError(reqwest::Error),
    ReqwestClientError(reqwest::Error),
}

pub struct Client {
    client: reqwest::Client,
    blocking_client: reqwest::blocking::Client,
    auth_headers: HeaderMap,
    auth_tokens: HashMap<String, Arc<Mutex<AuthToken>>>,
}

impl Client {
    pub fn new(
        user_agent: &str,
        client_id: &str,
        client_secret: &str,
        refresh_tokens: &Vec<&str>,
        timeout: Option<std::time::Duration>,
    ) -> Client {
        let client: reqwest::Client = match timeout {
            Some(t) => reqwest::ClientBuilder::new().timeout(t),
            None => reqwest::ClientBuilder::new(),
        }
            .default_headers({
                let mut header_map = HeaderMap::new();
                header_map.insert(
                    header::USER_AGENT,
                    HeaderValue::from_str(user_agent).unwrap(),
                );
                header_map.insert(
                    header::ACCEPT,
                    HeaderValue::from_static("application/json"),
                );
                header_map
            })
            .build()
            .unwrap();

        let blocking_client: reqwest::blocking::Client = match timeout {
            Some(t) => reqwest::blocking::ClientBuilder::new().timeout(t),
            None => reqwest::blocking::ClientBuilder::new(),
        }
            .default_headers({
                let mut header_map = HeaderMap::new();
                header_map.insert(
                    header::USER_AGENT,
                    HeaderValue::from_str(user_agent).unwrap(),
                );
                header_map.insert(
                    header::ACCEPT,
                    HeaderValue::from_static("application/json"),
                );
                header_map
            })
            .build()
            .unwrap();

        let mut auth_headers: HeaderMap = HeaderMap::new();
        auth_headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", base64::encode(format!(
                "{}:{}",
                client_id,
                client_secret,
            ))))
                .unwrap(),
        );
        auth_headers.insert(
            header::HOST,
            HeaderValue::from_static(HOST_URL),
        );

        let mut auth_tokens: HashMap<String, Arc<Mutex<AuthToken>>> =
            HashMap::new();
        for refresh_token in refresh_tokens {
            auth_tokens.insert(
                refresh_token.to_string(),
                Arc::new(Mutex::new(AuthToken::new())),
            );
        }

        Client {
            client: client,
            blocking_client: blocking_client,
            auth_headers: auth_headers,
            auth_tokens: auth_tokens,
        }
    }

    fn add_auth_header(
        &self,
        t: &str,
        query: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        query.header(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", t)).unwrap()
        )
    }

    fn try_authenticate(
        &self,
        refresh_token: Option<&str>,
        query: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, Error> {
        let refresh_token: &str = match refresh_token {
            Some(s) => s,
            None => return Ok(query),
        };

        let auth_token_ref = self.auth_tokens[refresh_token].clone();
        let auth_token = auth_token_ref.lock().unwrap(); // Read Only
        if !auth_token.expired() {
            return Ok(self.add_auth_header(&auth_token.access_token, query))
        }

        let now: u64 = time::now();
        let rep: reqwest::blocking::Response = self
            .blocking_client
            .post(AUTH_URL)
            .headers(self.auth_headers.clone())
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .send()
            .map_err(|e| Error::ReqwestClientError(e))?;
        if rep.status() != 200 {
            return Err(Error::AuthenticationStatusCode(rep.status()))
        }

        let data: AuthenticationResponse = rep.json()
            .map_err(|e| Error::JsonParseError(e))?;

        let mut auth_token = auth_token; // Mutable
        auth_token.access_token = data.access_token;
        auth_token.expiry = now + data.expires_in;

        let auth_token = auth_token; // Read Only
        Ok(self.add_auth_header(&auth_token.access_token, query))
    }

    pub async fn get_structure_orders(
        &self,
        location_id: &LocationId,
        refresh_token: Option<&str>,
    ) -> Result<Expirable<Vec<StructureOrder>>, Error> {
        let page_count: usize = self
            .try_authenticate(
                refresh_token,
                self.client
                    .head(structure_order_url(location_id))
                    .query(&[
                        ("datasource", "tranquility"),
                        ("page", "1"),
                    ]),
            )?
            .send()
            .await
            .map_err(|e| Error::ReqwestClientError(e))?
            .headers()
            .get("x-pages")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();

        let mut req_futures = FuturesUnordered::new();
        for i in 1..page_count + 1 {
            req_futures.push(
                self.try_authenticate(
                    refresh_token,
                    self.client
                        .get(structure_order_url(location_id))
                        .query(&[
                            ("datasource", "tranquility"),
                            ("page", &format!("{:?}", i)),
                        ]),
                )?
                .send()
            );
        }

        let mut parse_futures = FuturesUnordered::new();
        let mut greatest_expires_in: u64 = 0;
        while let Some(rep) = req_futures
            .try_next()
            .await
            .map_err(|e| Error::ReqwestClientError(e))?
        {
            let expires_in = expires_in(&rep);
            if expires_in > greatest_expires_in {
                greatest_expires_in = expires_in;
            }
            parse_futures.push(rep.json::<Vec<StructureOrder>>());
        }

        let mut structure_orders: Vec<StructureOrder> = {
            Vec::with_capacity(page_count * ORDERS_PER_PAGE)
        };
        while let Some(orders) = parse_futures
            .try_next()
            .await
            .map_err(|e| Error::JsonParseError(e))?
        {
            for order in orders.into_iter() {
                structure_orders.push(order);
            }
        }

        Ok(Expirable::new(structure_orders, greatest_expires_in))
    }

    pub async fn get_station_orders(
        &self,
        region_id: &RegionId,
        order_type: &str,
        type_id: &TypeId,
    ) -> Result<Expirable<Vec<StationOrder>>, Error> {
        let rep: reqwest::Response = self.client
            .get(station_order_url(region_id))
            .query(&[
                ("datasource", "tranquility"),
                ("page", "1"),
                ("order_type", order_type),
                ("type_id", &type_id.to_string()),
            ])
            .send()
            .await
            .map_err(|e| Error::ReqwestClientError(e))?;
        let expires_in: u64 = expires_in(&rep);
        rep.json::<Vec<StationOrder>>()
            .await
            .map_err(|e| Error::JsonParseError(e))
            .map(|v| { Expirable::new(v, expires_in) })
    }

    pub async fn get_adjusted_price(
        &self,
    ) -> Result<Expirable<Vec<AdjustedPrice>>, Error> {
        let rep: reqwest::Response = self.client
            .get(ADJUSTED_PRICE_URL)
            .query(&[("datasource", "tranquility")])
            .send()
            .await
            .map_err(|e| Error::ReqwestClientError(e))?;
        let expires_in: u64 = expires_in(&rep);
        rep.json::<Vec<AdjustedPrice>>()
            .await
            .map_err(|e| Error::JsonParseError(e))
            .map(|v| { Expirable::new(v, expires_in) })
    }

    pub async fn get_system_index(
        &self,
    ) -> Result<Expirable<Vec<SystemIndex>>, Error> {
        let rep: reqwest::Response = self.client
            .get(SYSTEM_INDEX_URL)
            .query(&[("datasource", "tranquility")])
            .send()
            .await
            .map_err(|e| Error::ReqwestClientError(e))?;
        let expires_in: u64 = expires_in(&rep);
        rep.json::<Vec<SystemIndex>>()
            .await
            .map_err(|e| Error::JsonParseError(e))
            .map(|v| { Expirable::new(v, expires_in) })
    }
}

fn expires_in(response: &reqwest::Response) -> u64 {
    u64::try_from(DateTime::parse_from_rfc2822(response
        .headers()
        .get("expires")
        .unwrap()
        .to_str()
        .unwrap()
    )
        .unwrap()
        .timestamp()
    )
        .unwrap()
}

struct AuthToken {
    access_token: String,
    expiry: u64,
}

impl AuthToken {
    fn new() -> AuthToken {
        AuthToken {
            access_token: "".to_string(),
            expiry: 0,
        }
    }

    fn expired(&self) -> bool {
        time::now() >= self.expiry
    }
}

use crate::{
    {LocationId, RegionId, MarketName, RefreshToken},
    config::{Markets, MinCacheDuration},
    esi_client::Client,
    service::Service,
    error::Error,
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    env::var,
};

use serde::Deserialize;
use either::Either;
use serde_json;

pub fn service_from_env() -> Result<Service, Error> {
    EnvData::from_env_var()?
        .into_service()
}

#[derive(Deserialize, Debug, Clone)]
struct EnvData {
    service_address: String,
    user_agent: String,
    client_id: String,
    client_secret: String,
    client_timeout: Option<String>,
    station_mo_timeout: String,
    structure_mo_timeout: String,
    adjusted_price_timeout: String,
    system_index_timeout: String,
    station_markets: String,
    structure_markets: String,
}

#[derive(Deserialize, Debug, Clone)]
struct StationMarket {
    location_id: LocationId,
    region_id: RegionId,
}

#[derive(Deserialize, Debug, Clone)]
struct StructureMarket {
    location_id: LocationId,
    refresh_token: Option<RefreshToken>,
}

impl EnvData {
    fn from_env_var() -> Result<EnvData, Error> {
        Ok(EnvData {
            service_address: var("WM_SERVICE_ADDRESS")?,
            user_agent: var("WM_USER_AGENT")?,
            client_id: var("WM_CLIENT_ID")?,
            client_secret: var("WM_CLIENT_SECRET")?,
            client_timeout: match var("WM_CLIENT_TIMEOUT") {
                Ok(timeout) => Some(timeout),
                Err(std::env::VarError::NotPresent) => None,
                Err(e) => return Err(Error::from(e)),
            },
            station_mo_timeout: var("WM_STATION_MARKET_ORDERS_TIMEOUT")?,
            structure_mo_timeout: var("WM_STRUCTURE_MARKET_ORDERS_TIMEOUT")?,
            adjusted_price_timeout: var("WM_ADJUSTED_PRICE_TIMEOUT")?,
            system_index_timeout: var("WM_SYSTEM_INDEX_TIMEOUT")?,
            station_markets: var("WM_STATION_MARKETS")?,
            structure_markets: var("WM_STRUCTURE_MARKETS")?,
        })
    }

    fn into_service(self) -> Result<Service, Error> {
        let service_address: SocketAddr = self.service_address.parse()?;

        let min_cache_duration: MinCacheDuration = MinCacheDuration {
            station_market_orders: self.station_mo_timeout.parse()?,
            structure_market_orders: self.structure_mo_timeout.parse()?,
            adjusted_price: self.adjusted_price_timeout.parse()?,
            system_index: self.system_index_timeout.parse()?,
        };

        let station_markets: HashMap<MarketName, StationMarket> =
            serde_json::from_str(&self.station_markets)
                .map_err(|e| Error::EnvJsonParseError(e))?;
        let structure_markets: HashMap<MarketName, StructureMarket> =
            serde_json::from_str(&self.structure_markets)
                .map_err(|e| Error::EnvJsonParseError(e))?;
        let mut markets: Markets = Markets::with_capacity(
            station_markets.len() + structure_markets.len()
        );
        for (k, v) in station_markets {
            markets.insert(k, (v.location_id, Either::Left(v.region_id)));
        }
        for (k, v) in structure_markets {
            markets.insert(k, (v.location_id, Either::Right(v.refresh_token)));
        }
        
        let refresh_tokens: Vec<&str> = markets.refresh_tokens();
        let client: Client = Client::new(
            &self.user_agent,
            &self.client_id,
            &self.client_secret,
            &refresh_tokens,
            match self.client_timeout {
                Some(s) => Some(std::time::Duration::from_secs(s.parse()?)),
                None => None,
            },
        );

        Ok(Service::new(client, markets, min_cache_duration, service_address))
    }
}

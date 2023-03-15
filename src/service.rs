use crate::{
    {LocationId, RegionId, TypeId},
    proto::weve_market_server::*,
    esi_client::*,
    cache::Cache,
    error::Error,
    proto::*,
    json::*,
    config,
};

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
    net::SocketAddr,
    cmp::max,
};

use tonic::{Request, Response, Status, transport::Server};
use tokio::sync::Mutex;
use either::Either;

type AdjustedPriceCache = Arc<Mutex<Cache<AdjustedPriceReq, AdjustedPriceRep>>>;
type SystemIndexCache = Arc<Mutex<Cache<SystemIndexReq, SystemIndexRep>>>;
type StationMarketOrderCache = HashMap<
    RegionId,
    Arc<RwLock<HashMap<
        (TypeId, bool),
        Arc<Mutex<Cache<MarketOrdersReq, MarketOrdersRep>>>,
    >>>,
>;
type StructureMarketOrderCache = HashMap<
LocationId,
    Arc<Mutex<Cache<MarketOrdersReq, MarketOrdersRep>>>,
>;

pub struct Service {
    esi_client: Client,
    structure_cache: StructureMarketOrderCache,
    station_cache: StationMarketOrderCache,
    adjusted_price_cache: AdjustedPriceCache,
    system_index_cache: SystemIndexCache,
    markets: config::Markets,
    stations: HashSet<(RegionId, LocationId)>,
    station_markets: HashMap<LocationId, String>,
    min_cache_time: config::MinCacheDuration,
    address: Option<SocketAddr>,
}

impl Service {
    pub fn new(
        esi_client: Client,
        markets: config::Markets,
        min_cache_time: config::MinCacheDuration,
        address: SocketAddr,
    ) -> Service {
        let stations = markets.stations();
        let station_markets = markets.station_markets();
        let system_index_cache = Arc::new(Mutex::new(Cache::new()));
        let adjusted_price_cache = Arc::new(Mutex::new(Cache::new()));

        let mut station_cache = HashMap::new();
        let mut structure_cache = HashMap::new();

        for item in markets.values() {
            match item {
                (_, Either::Left(region_id)) => station_cache
                    .insert(
                        *region_id,
                        Arc::new(RwLock::new(HashMap::new())),
                    )
                    .map(|_| ()),
                (location_id, _) => structure_cache
                    .insert(
                        *location_id,
                        Arc::new(Mutex::new(Cache::new())),
                    )
                    .map(|_| ()),
            };
        }

        Service {
            esi_client: esi_client,
            structure_cache: structure_cache,
            station_cache: station_cache,
            adjusted_price_cache: adjusted_price_cache,
            system_index_cache: system_index_cache,
            markets: markets,
            stations: stations,
            station_markets: station_markets,
            min_cache_time: min_cache_time,
            address: Some(address),
        }
    }

    pub async fn serve(mut self) -> Result<(), Error> {
        let address: SocketAddr = self
            .address
            .take()
            .unwrap();
        Server::builder()
            .add_service(WeveMarketServer::new(self))
            .serve(address)
            .await
            .map_err(|e| Error::ServiceServeError(e))
    }

    async fn station_orders(
        &self,
        req: MarketOrdersReq,
        location_id: &LocationId,
        region_id: &RegionId,
    ) -> Result<Response<MarketOrdersRep>, Status> {
        let region_map_ref = self.station_cache[region_id].clone();
        let k = &(req.type_id, req.buy);

        let cache_ref = region_map_ref
            .read()
            .unwrap()
            .get(k)
            .map(|c| c.clone());
        let cache_ref = match cache_ref {
            Some(c) => c,
            None => {
                let mut region_map = region_map_ref.write().unwrap();
                region_map.insert(*k, Arc::new(Mutex::new(Cache::new())));
                region_map[k].clone()
            }
        };
        let mut cache = cache_ref.lock().await;

        if !cache.expired() {
            match cache.get(&req) {
                Some(rep) => return Ok(Response::new(rep.clone())),
                None => return Ok(Response::new(MarketOrdersRep {
                    market_orders: Vec::new(),
                })),
            };
        }

        let raws: Expirable<Vec<StationOrder>> = self
            .esi_client
            .get_station_orders(
                &region_id,
                match req.buy {
                    true => "buy",
                    false => "sell",
                },
                &req.type_id,
            )
            .await
            .unwrap();

        cache.clear_and_update_expiry(max(
            raws.expires_in,
            self.min_cache_time.station_market_orders(),
        ));

        let mut rep: MarketOrdersRep = MarketOrdersRep {
            market_orders: Vec::with_capacity(raws.inner.len()),
        };
        let mut reps: HashMap<LocationId, MarketOrdersRep> = HashMap::new();
        for raw in raws.into_inner().into_iter() {
            let k = &raw.location_id;
            if k == location_id {
                rep.market_orders.push(raw.into_proto());
            }
            else if self.stations.contains(&(*region_id, *k)) {
                match reps.get_mut(k) {
                    Some(r) => r
                        .market_orders
                        .push(raw.into_proto()),
                    None => reps
                        .insert(*k, MarketOrdersRep {
                            market_orders: vec![raw.into_proto()],
                        })
                        .map_or_else(|| (), |_| ()),
                };
            }
        }

        cache.insert(req.clone(), rep);
        for (location_id, rep) in reps.into_iter() {
            cache.insert(
                MarketOrdersReq {
                    type_id: req.type_id,
                    market: self.station_markets[&location_id].clone(),
                    buy: req.buy,
                },
                rep,
            )
        }

        match cache.get_forced(&req) {
            Some(rep) => Ok(Response::new(rep.clone())),
            None => Ok(Response::new(MarketOrdersRep {
                market_orders: Vec::new(),
            }))
        }
    }

    async fn structure_orders(
        &self,
        req: MarketOrdersReq,
        location_id: &i64,
        refresh_token: Option<&str>,
    ) -> Result<Response<MarketOrdersRep>, Status> {
        let cache_ref = self.structure_cache[location_id].clone();
        let mut cache = cache_ref.lock().await;

        if !cache.expired() {
            match cache.get(&req) {
                Some(rep) => return Ok(Response::new(rep.clone())),
                None => return Ok(Response::new(MarketOrdersRep {
                    market_orders: Vec::new(),
                })),
            };
        }

        let raws: Expirable<Vec<StructureOrder>> = self
            .esi_client
            .get_structure_orders(
                location_id,
                refresh_token,
            )
            .await
            .unwrap();

        cache.clear_and_update_expiry(max(
            raws.expires_in,
            self.min_cache_time.structure_market_orders(),
        ));

        let mut reps: HashMap<(TypeId, bool), MarketOrdersRep> = HashMap::new();
        for raw in raws.into_inner().into_iter() {
            let k = (raw.type_id, raw.is_buy_order);
            match reps.get_mut(&k) {
                Some(r) => r
                    .market_orders
                    .push(raw.into_proto()),
                None => reps
                    .insert(k, MarketOrdersRep {
                        market_orders: vec![raw.into_proto()]
                    })
                    .map_or_else(|| (), |_| ()),
            };
        }

        for ((type_id, is_buy_order), rep) in reps.into_iter() {
            cache.insert(
                MarketOrdersReq {
                    type_id: type_id,
                    market: req.market.clone(),
                    buy: is_buy_order,
                },
                rep,
            );
        }

        match cache.get_forced(&req) {
            Some(rep) => Ok(Response::new(rep.clone())),
            None => Ok(Response::new(MarketOrdersRep {
                market_orders: Vec::new(),
            }))
        }
    }
}

#[tonic::async_trait]
impl WeveMarket for Service {
    async fn market_orders(
        &self,
        request: Request<MarketOrdersReq>,
    ) -> Result<Response<MarketOrdersRep>, Status> {
        let req = request.into_inner();
        println!(
            "Received MarketOrdersReq: [{}] [{}] [{}]",
            req.type_id,
            req.market,
            req.buy,
        );
        match self.markets.get(&req.market) {
            Some((location_id, Either::Left(region_id))) => self
                .station_orders(req, location_id, region_id)
                .await,
            Some((location_id, Either::Right(refresh_token))) => self
                .structure_orders(req, location_id, refresh_token.as_deref())
                .await,
            None => Ok(Response::new(MarketOrdersRep{
                market_orders: Vec::new(),
            })),
        }
    }

    async fn adjusted_price(
        &self,
        request: Request<AdjustedPriceReq>,
    ) -> Result<Response<AdjustedPriceRep>, Status> {
        let req = request.into_inner();
        println!(
            "Received AdjustedPriceReq: [{}]",
            req.type_id,
        );
        let cache_ref = self.adjusted_price_cache.clone();
        let mut cache = cache_ref.lock().await;

        if let Some(rep) = cache.get(&req) {
            return Ok(Response::new(rep.clone()));
        }

        let raws: Expirable<Vec<AdjustedPrice>> = self
            .esi_client
            .get_adjusted_price()
            .await
            .unwrap();
        cache.clear_and_update_expiry(max(
            raws.expires_in,
            self.min_cache_time.adjusted_price(),
        ));

        for raw in raws.into_inner().into_iter() {
            cache.insert(
                raw.clone().into_proto_req(),
                raw.into_proto(),
            );
        }

        Ok(Response::new(cache.get_forced(&req).unwrap().clone()))
    }

    async fn system_index(
        &self,
        request: Request<SystemIndexReq>,
    ) -> Result<Response<SystemIndexRep>, Status> {
        let req = request.into_inner();
        println!(
            "Received SystemIndexReq: [{}]",
            req.system_id,
        );
        let cache_ref = self.system_index_cache.clone();
        let mut cache = cache_ref.lock().await;

        if let Some(rep) = cache.get(&req) {
            return Ok(Response::new(rep.clone()));
        }

        let raws: Expirable<Vec<SystemIndex>> = self
            .esi_client
            .get_system_index()
            .await
            .unwrap();
        cache.clear_and_update_expiry(max(
            raws.expires_in,
            self.min_cache_time.system_index(),
        ));
        for raw in raws.into_inner().into_iter() {
            cache.insert(
                raw.clone().into_proto_req(),
                raw.into_proto(),
            );
        }

        Ok(Response::new(cache.get_forced(&req).unwrap().clone()))
    }
}

impl Hash for MarketOrdersReq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.market.hash(state);
        self.buy.hash(state);
    }
}
impl Eq for MarketOrdersReq {}

impl Hash for AdjustedPriceReq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}
impl Eq for AdjustedPriceReq {}

impl Hash for SystemIndexReq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.system_id.hash(state);
    }
}
impl Eq for SystemIndexReq {}

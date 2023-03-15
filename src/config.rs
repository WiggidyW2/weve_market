use crate::{
    {LocationId, RegionId, MarketName, RefreshToken},
    time,
};

use std::collections::{HashMap, HashSet};

use either::Either;

#[derive(Debug, Default, Clone)]
pub struct Markets {
    inner: HashMap<
        MarketName,
        (LocationId, Either<RegionId, Option<RefreshToken>>),
    >
}

#[derive(Debug, Default, Clone)]
pub struct MinCacheDuration {
    pub station_market_orders: u64,
    pub structure_market_orders: u64,
    pub adjusted_price: u64,
    pub system_index: u64,
}

impl Markets {
    pub fn with_capacity(capacity: usize) -> Markets {
        Markets {
            inner: HashMap::with_capacity(capacity)
        }
    }

    pub fn insert(
        &mut self,
        k: MarketName,
        v: (LocationId, Either<RegionId, Option<RefreshToken>>),
    ) {
        self.inner.insert(k, v);
    }

    pub fn get(
        &self,
        k: &str,
    ) -> Option<&(LocationId, Either<RegionId, Option<RefreshToken>>)> {
        self.inner.get(k)
    }

    pub fn values(
        &self,
    ) -> impl Iterator<
        Item = &(LocationId, Either<RegionId, Option<RefreshToken>>)
    > {
        self.inner.values()
    }

    // Returns an iterator over all locationid which are structures
    // pub fn structure_ids(&self) -> impl Iterator<Item = LocationId> + '_ {
    //     self.inner
    //         .values()
    //         .filter_map(|(location_id, either)| 
    //             match either {
    //                 Either::Right(_) => Some(*location_id),
    //                 _ => None,
    //             }
    //         )
    // }

    // Returns a HashSet of (regionid, locationid) pairs
    pub fn stations(&self) -> HashSet<(RegionId, LocationId)> {
        let mut stations = HashSet::new();
        for v in self.values() {
            match v {
                (locationid, Either::Left(regionid)) => stations
                    .insert((*regionid, *locationid)),
                _ => false,
            };
        }
        stations
    }

    // Returns a HashMap indexing the MarketName for a locationid key
    pub fn station_markets(&self) -> HashMap<LocationId, MarketName> {
        let mut station_markets = HashMap::new();
        for (k, v) in self.inner.iter() {
            match v {
                (locationid, Either::Left(_)) => station_markets
                    .insert(*locationid, k.to_string()),
                _ => None,
            };
        }
        station_markets
    }

    // Returns a vector of all refresh tokens as ref
    pub fn refresh_tokens<'s>(&'s self) -> Vec<&'s str> {
        let mut tokens: Vec<&'s str> = Vec::new();
        for (_, v) in self.inner.iter() {
            if let Either::Right(Some(s)) = &v.1 {
                if !tokens.contains(&s.as_str()) {
                    tokens.push(s);
                }
            }
        }
        tokens
    }
}

impl MinCacheDuration {
    pub fn station_market_orders(&self) -> u64 {
        time::now() + self.station_market_orders
    }
    pub fn structure_market_orders(&self) -> u64 {
        time::now() + self.structure_market_orders
    }
    pub fn adjusted_price(&self) -> u64 {
        time::now() + self.adjusted_price
    }
    pub fn system_index(&self) -> u64 {
        time::now() + self.system_index
    }
}

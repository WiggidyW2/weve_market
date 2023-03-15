use crate::{
    {LocationId, TypeId},
    proto::*,
};

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Expirable<T> {
    pub inner: T,
    pub expires_in: u64,
}

impl<T> Expirable<T> {
    pub fn new(t: T, expires_in: u64) -> Expirable<T> {
        Expirable {
            inner: t,
            expires_in: expires_in,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct AuthenticationResponse {
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StructureOrder {
    pub is_buy_order: bool,
    pub price: f64,
    pub type_id: TypeId,
    pub volume_remain: i32,
}

impl StructureOrder {
    pub fn into_proto(self) -> MarketOrder {
        MarketOrder {
            quantity: self.volume_remain,
            price: self.price,
        }
    }

    // pub fn into_proto_req(self, market: String) -> MarketOrdersReq {
    //     MarketOrdersReq {
    //         type_id: self.type_id,
    //         market: market,
    //         buy: self.is_buy_order,
    //     }
    // }
}

#[derive(Deserialize, Debug, Clone)]
pub struct StationOrder {
    pub location_id: LocationId,
    pub price: f64,
    pub volume_remain: i32,
}

impl StationOrder {
    pub fn into_proto(self) -> MarketOrder {
        MarketOrder {
            quantity: self.volume_remain,
            price: self.price,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct AdjustedPrice {
    pub adjusted_price: f64,
    pub type_id: TypeId,
}

impl AdjustedPrice {
    pub fn into_proto(self) -> AdjustedPriceRep {
        AdjustedPriceRep {
            adjusted_price: self.adjusted_price,
        }
    }

    pub fn into_proto_req(self) -> AdjustedPriceReq {
        AdjustedPriceReq {
            type_id: self.type_id,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SystemIndex {
    pub cost_indices: [CostIndex; 6],
    pub solar_system_id: i32,
}

impl SystemIndex {
    pub fn into_proto(self) -> SystemIndexRep {
        let mut system_index_rep = SystemIndexRep {
            manufacturing: 0.0,
            research_te: 0.0,
            research_me: 0.0,
            copying: 0.0,
            invention: 0.0,
            reactions: 0.0,
        };
        for cost_indice in self.cost_indices.into_iter() {
            match cost_indice.activity {
                Activity::Manufacturing => system_index_rep.manufacturing = cost_indice.cost_index,
                Activity::ResearchTE => system_index_rep.research_te = cost_indice.cost_index,
                Activity::ResearchME => system_index_rep.research_me = cost_indice.cost_index,
                Activity::Copying => system_index_rep.copying = cost_indice.cost_index,
                Activity::Invention => system_index_rep.invention = cost_indice.cost_index,
                Activity::Reactions => system_index_rep.reactions = cost_indice.cost_index,
            }
        }
        system_index_rep
    }

    pub fn into_proto_req(self) -> SystemIndexReq {
        SystemIndexReq {
            system_id: self.solar_system_id,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CostIndex {
    pub activity: Activity,
    pub cost_index: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub enum Activity {
    #[serde(rename = "manufacturing")]
    Manufacturing,
    #[serde(rename = "researching_time_efficiency")]
    ResearchTE,
    #[serde(rename = "researching_material_efficiency")]
    ResearchME,
    #[serde(rename = "copying")]
    Copying,
    #[serde(rename = "invention")]
    Invention,
    #[serde(rename = "reaction")]
    Reactions,
}

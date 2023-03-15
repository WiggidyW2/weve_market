use crate::time;

use std::collections::HashMap;

pub struct Cache<K, V> {
    inner: HashMap<K, V>,
    expiry: u64,
}

impl<K: Eq + std::hash::Hash, V> Cache<K, V> {
    pub fn new() -> Cache<K, V> {
        Cache {
            inner: HashMap::new(),
            expiry: 0,
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        match &time::now() < &self.expiry {
            true => self.inner.get(k),
            false => None,
        }
    }

    pub fn get_forced(&self, k: &K) -> Option<&V> {
        self.inner.get(k)
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.inner.insert(k, v);
    }

    pub fn clear_and_update_expiry(&mut self, expiry: u64) {
        self.inner.clear();
        self.expiry = expiry;
    }

    pub fn expired(&self) -> bool {
        time::now() > self.expiry
    }
}

// impl Cache<crate::proto::AdjustedPriceReq, crate::proto::AdjustedPriceRep>{
//     pub fn debug_print(&self) {
//         for v in self.inner.iter() {
//             if v.1.adjusted_price != 0.0 {
//                 println!("{:?}", v);
//             }
//         }
//     }
// }

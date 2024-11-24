use crate::db_models::Offer;
use fxhash::{FxHashMap, FxHashSet};
use gxhash::{HashMapExt, HashSetExt};

pub struct NumberOfDaysIndex {
    map: FxHashMap<u32, Vec<u32>>,
}

impl NumberOfDaysIndex {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::new(),
        }
    }

    pub fn filter_offers(&self, days: u32, offers: impl Iterator<Item=u32>) -> impl Iterator<Item=u32> {
        let set = if let Some(set) = self.map.get(&days) {
            FxHashSet::from_iter(set.iter().copied())
        } else {
            FxHashSet::new()
        };
        offers.filter(move |offer| set.contains(offer))
    }

    pub fn index_offer(&mut self, offer: &Offer) {
        let days = ((offer.end_date - offer.start_date) / (1000 * 60 * 60 * 24)) as u32;
        self.map.entry(days).or_default().push(offer.idx);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

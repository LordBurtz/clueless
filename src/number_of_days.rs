use crate::db_models::Offer;
use fxhash::{FxHashMap, FxHashSet};
use gxhash::{HashMapExt, HashSetExt};

pub struct NumberOfDaysIndex {
    map: FxHashMap<u32, FxHashSet<u32>>,
}

impl NumberOfDaysIndex {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::new(),
        }
    }

    pub fn filter_offers<'a>(&'a self, days: u32, offers: impl Iterator<Item=u32> + 'a) -> Box<dyn Iterator<Item=u32> + 'a> {
        let set = match self.map.get(&days) {
            Some(set) => set,
            None => return Box::new(std::iter::empty()),
        };
        Box::new(offers.filter(move |idx| set.contains(idx)))
    }

    pub fn index_offer(&mut self, offer: &Offer) {
        let days = ((offer.end_date - offer.start_date) / (1000 * 60 * 60 * 24)) as u32;
        self.map.entry(days).or_default().insert(offer.idx);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

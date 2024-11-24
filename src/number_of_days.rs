use crate::db_models::Offer;
use fxhash::FxHashMap;
use gxhash::HashMapExt;

pub struct NumberOfDaysIndex {
    map: FxHashMap<u32, Vec<u32>>,
}

impl NumberOfDaysIndex {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::new(),
        }
    }

    pub fn filter_offers(&self, days: u32, offers: impl Iterator<Item = u32>) -> impl Iterator<Item = u32> {
        if let Some(set) = self.map.get(&days) {
            let sorted_set: Vec<u32> = {
                let mut vec: Vec<u32> = set.iter().copied().collect();
                vec.sort_unstable();
                vec
            };
            offers.filter(move |offer| sorted_set.binary_search(offer).is_ok())
        } else {
            offers.filter(|_| false)
        }
    }

    pub fn index_offer(&mut self, offer: &Offer) {
        let days = ((offer.end_date - offer.start_date) / (1000 * 60 * 60 * 24)) as u32;
        self.map.entry(days).or_default().push(offer.idx);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

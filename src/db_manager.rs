use std::cmp::Reverse;
use crate::db_models::{ Offer};
use crate::index_tree::{IndexTree, ROOT_REGION};
use crate::json_models::{
    CarTypeCount, FreeKilometerRange, GetReponseBodyModel, PriceRange, RequestOffer, ResponseOffer,
    SeatCount, SortOrder, VollKaskoCount, CarType
};
use crate::GenericError;
use fxhash::{FxBuildHasher, FxHashMap};
use gxhash::HashMapExt;
use itertools::Itertools;
use std::collections::{BinaryHeap, HashMap};
use tokio::sync::RwLock;


pub struct DBManager {
    pub index_tree_lock: RwLock<IndexTree>,
    pub dense_store_lock: RwLock<DenseStore>,
}

// impl CarType {
//     fn eq_me(&self, other: &crate::json_models::CarType) -> bool {
//         match (self, other) {
//             (CarType::Small, crate::json_models::CarType::Small) => true,
//             (CarType::Sports, crate::json_models::CarType::Sports) => true,
//             (CarType::Luxury, crate::json_models::CarType::Luxury) => true,
//             (CarType::Family, crate::json_models::CarType::Family) => true,
//             _ => false,
//         }
//     }
// }

use std::cmp::Ordering;

struct HeapItem<'a> {
    sort_key: u32,
    offer: &'a Offer,
}

impl<'a> PartialEq for HeapItem<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.offer.idx == other.offer.idx
    }
}

impl<'a> Eq for HeapItem<'a> {}

impl<'a> PartialOrd for HeapItem<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // Delegate to `Ord` implementation
    }
}

impl<'a> Ord for HeapItem<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key.cmp(&other.sort_key)
            .then_with(|| self.offer.id.cmp(&other.offer.id)) // Tie-breaker
    }
}



impl DBManager {
    pub fn new() -> Self {
        Self {
            index_tree_lock: IndexTree::populate_with_regions(&ROOT_REGION).into(),
            dense_store_lock: DenseStore::new().into(),
        }
    }

    pub async fn query_for(
        &self,
        request_offer: RequestOffer,
    ) -> Result<GetReponseBodyModel, GenericError> {
        let dense_store = self.dense_store_lock.read().await;
        let index_tree = self.index_tree_lock.read().await;

        let mut page_offers_heap = BinaryHeap::new();
        let page_size = request_offer.page_size as usize;
        let page_start = (request_offer.page * request_offer.page_size) as usize;
        let page_end = page_start + page_size;

        let offers_iter = index_tree
            .get_available_offers(
                request_offer.region_id,
                request_offer.number_days,
                request_offer.time_range_start,
                request_offer.time_range_end,
            )
            .map(|offer_idx| &dense_store.all[offer_idx as usize]);

        let mut vollkasko_count = VollKaskoCount {
            true_count: 0,
            false_count: 0,
        };

        let mut car_type_count = CarTypeCount {
            small: 0,
            sports: 0,
            luxury: 0,
            family: 0,
        };

        let mut free_kilometers_interval_mapping = FxHashMap::new();
        let mut price_range_interval_mapping = FxHashMap::new();
        let mut seats_count_map = FxHashMap::new();

        for offer in offers_iter {
            let mut seats_incl = true;
            let mut car_type_incl = true;
            let mut only_vollkasko_ignored = true;
            let mut free_kilometers_incl = true;
            let mut price_range_incl = true;

            if let Some(minNumberOfSeats) = request_offer.min_number_seats {
                if offer.number_seats < minNumberOfSeats {
                    seats_incl = false;
                }
            }
            if let Some(carType) = request_offer.car_type {
                if ! (offer.car_type == carType) {
                    car_type_incl = false
                }
            }
            if let Some(vollkasko_required) = request_offer.only_vollkasko {
                if vollkasko_required && !offer.has_vollkasko {
                    only_vollkasko_ignored = false;
                }
            }
            if let Some(minFreeKilometers) = request_offer.min_free_kilometer {
                if offer.free_kilometers < minFreeKilometers {
                    free_kilometers_incl = false;
                }
            }
            if let Some(maxPrice) = request_offer.max_price {
                if maxPrice <= offer.price {
                    price_range_incl = false;
                }
            }
            if let Some(minPrice) = request_offer.min_price {
                if minPrice > offer.price {
                    price_range_incl = false;
                }
            }
            match (
                seats_incl,
                car_type_incl,
                only_vollkasko_ignored,
                free_kilometers_incl,
                price_range_incl,
            ) {
                (true, true, true, true, true) => {

                    let sort_key = match request_offer.sort_order {
                        SortOrder::PriceAsc => offer.price,
                        SortOrder::PriceDesc => u32::MAX - offer.price,
                    };

                    let heap_item = HeapItem {
                        sort_key,
                        offer,
                    };

                    if page_offers_heap.len() < page_end {
                        page_offers_heap.push(heap_item);
                    } else if let Some(top_item) = page_offers_heap.peek() {
                        if heap_item < *top_item {
                            page_offers_heap.pop();
                            page_offers_heap.push(heap_item);
                        }
                    }
                    Self::handle_vollkasko_count(&mut vollkasko_count, offer);
                    Self::handle_car_type_count(&mut car_type_count, offer);
                    Self::handle_free_kilometers_range(
                        &request_offer,
                        &mut free_kilometers_interval_mapping,
                        offer,
                    );
                    Self::handle_price_range(
                        &request_offer,
                        &mut price_range_interval_mapping,
                        offer,
                    );
                    Self::handle_seats_count(&mut seats_count_map, offer);
                }
                (true, true, true, true, false) => {
                    Self::handle_price_range(
                        &request_offer,
                        &mut price_range_interval_mapping,
                        offer,
                    );
                }
                (true, true, true, false, true) => {
                    Self::handle_free_kilometers_range(
                        &request_offer,
                        &mut free_kilometers_interval_mapping,
                        offer,
                    );
                }
                (true, true, false, true, true) => {
                    Self::handle_vollkasko_count(&mut vollkasko_count, offer);
                }
                (true, false, true, true, true) => {
                    Self::handle_car_type_count(&mut car_type_count, offer);
                }
                (false, true, true, true, true) => {
                    Self::handle_seats_count(&mut seats_count_map, offer);
                }
                _ => {}
            }
        }

        let mut price_ranges = Vec::with_capacity(price_range_interval_mapping.len());

        for key in price_range_interval_mapping.keys().sorted() {
            let count = price_range_interval_mapping[key];
            price_ranges.push(PriceRange {
                start: *key,
                end: *key + request_offer.price_range_width,
                count,
            });
        }

        let mut kilometer_ranges = Vec::with_capacity(free_kilometers_interval_mapping.len());
        for key in free_kilometers_interval_mapping.keys().sorted() {
            let count = free_kilometers_interval_mapping[key];
            kilometer_ranges.push(FreeKilometerRange {
                start: *key,
                end: *key + request_offer.min_free_kilometer_width,
                count,
            });
        }

        // Extract the offers for the current page from the heap
        let page_offers_vec: Vec<_> = page_offers_heap
            .into_sorted_vec()
            .into_iter()
            .collect();


        // Paginate
        let paged_offers = page_offers_vec
            .into_iter()
            .skip(page_start)
            .take(page_size)
            .map(|item| ResponseOffer {
                ID: item.offer.id.clone(),
                data: item.offer.data.clone(),
            })
            .collect();

        Ok(GetReponseBodyModel {
            offers: paged_offers,
            price_ranges,
            car_type_counts: car_type_count,
            seats_count: seats_count_map
                .into_iter()
                .map(|(number_seats, count)| SeatCount {
                    number_seats,
                    count,
                })
                .collect(),
            free_kilometer_range: kilometer_ranges,
            vollkasko_count,
        })
    }

    #[inline(always)]
    fn handle_seats_count(seats_count_map: &mut HashMap<u32, u32, FxBuildHasher>, offer: &Offer) {
        seats_count_map
            .entry(offer.number_seats)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    #[inline(always)]
    fn handle_price_range(
        request_offer: &RequestOffer,
        price_range_interval_mapping: &mut HashMap<u32, u32, FxBuildHasher>,
        offer: &Offer,
    ) {
        let lower_bound =
            (offer.price / request_offer.price_range_width) * request_offer.price_range_width;
        price_range_interval_mapping
            .entry(lower_bound)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    #[inline(always)]
    fn handle_free_kilometers_range(
        request_offer: &RequestOffer,
        free_kilometers_interval_mapping: &mut HashMap<u32, u32, FxBuildHasher>,
        offer: &Offer,
    ) {
        let lower_bound = (offer.free_kilometers / request_offer.min_free_kilometer_width)
            * request_offer.min_free_kilometer_width;
        free_kilometers_interval_mapping
            .entry(lower_bound)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    #[inline(always)]
    fn handle_car_type_count(car_type_count: &mut CarTypeCount, offer: &Offer) {
        match offer.car_type {
            CarType::Small => car_type_count.small += 1,
            CarType::Sports => car_type_count.sports += 1,
            CarType::Luxury => car_type_count.luxury += 1,
            CarType::Family => car_type_count.family += 1,
        }
    }

    #[inline(always)]
    fn handle_vollkasko_count(vollkasko_count: &mut VollKaskoCount, offer: &Offer) {
        if offer.has_vollkasko {
            vollkasko_count.true_count += 1;
        } else {
            vollkasko_count.false_count += 1;
        }
    }

    fn get_car_type_count(
        offers: &[Offer],
        excluded_offers: &[&Offer],
        request_offer: &RequestOffer,
    ) -> CarTypeCount {
        let filtered_offers_count = offers.len() as u32;
        match request_offer.car_type {
            None => {
                let mut small = 0;
                let mut sports = 0;
                let mut luxury = 0;
                let mut family = 0;
                for offer in offers {
                    match offer.car_type {
                        CarType::Small => small += 1,
                        CarType::Sports => sports += 1,
                        CarType::Luxury => luxury += 1,
                        CarType::Family => family += 1,
                    }
                }
                CarTypeCount {
                    small,
                    sports,
                    luxury,
                    family,
                }
            }
            Some(filtered_car_type) => {
                let mut small_excluded = 0;
                let mut family_excluded = 0;
                let mut luxury_excluded = 0;
                let mut sports_excluded = 0;
                for offer in excluded_offers {
                    match offer.car_type {
                        CarType::Small => small_excluded += 1,
                        CarType::Sports => sports_excluded += 1,
                        CarType::Luxury => luxury_excluded += 1,
                        CarType::Family => family_excluded += 1,
                    };
                }
                CarTypeCount {
                    small: if CarType::Small.eq(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        small_excluded
                    },
                    sports: if CarType::Sports.eq(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        sports_excluded
                    },
                    luxury: if CarType::Luxury.eq(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        luxury_excluded
                    },
                    family: if CarType::Family.eq(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        family_excluded
                    },
                }
            }
        }
    }

    fn sort_orders_and_paginate(
        offers: &mut Vec<&Offer>,
        request_offer: RequestOffer,
    ) -> Vec<ResponseOffer> {
        if offers.is_empty() {
            return vec![];
        }

        match request_offer.sort_order {
            SortOrder::PriceAsc => offers.sort_by(|a, b| {
                let comp = a.price.cmp(&b.price);
                if comp.is_eq() {
                    return a.id.cmp(&b.id);
                }
                return comp;
            }),
            SortOrder::PriceDesc => offers.sort_by(|a, b| {
                let comp = b.price.cmp(&a.price);
                if comp.is_eq() {
                    return a.id.cmp(&b.id);
                }
                return comp;
            }),
        }

        offers
            .into_iter()
            .skip(((request_offer.page) * request_offer.page_size) as usize) // pagination starts at 0
            .take(request_offer.page_size as usize)
            .map(|o| ResponseOffer {
                ID: o.id.clone(),
                data: o.data.clone(),
            })
            .collect()
    }

    fn to_free_kilometers_offers<'a>(
        offers: impl Iterator<Item = &'a Offer>,
        free_kilometer_width: u32,
    ) -> Vec<FreeKilometerRange> {
        let mut interval_mapping = FxHashMap::new();

        for offer in offers {
            let lower_bound = (offer.free_kilometers / free_kilometer_width) * free_kilometer_width;
            interval_mapping
                .entry(lower_bound)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        let mut kilometer_ranges = Vec::with_capacity(interval_mapping.len());
        for key in interval_mapping.keys().sorted() {
            let count = interval_mapping[key];
            kilometer_ranges.push(FreeKilometerRange {
                start: *key,
                end: *key + free_kilometer_width,
                count,
            });
        }
        kilometer_ranges
    }

    fn to_vollkasko_offers<'a>(offers: impl Iterator<Item = &'a Offer>) -> VollKaskoCount {
        // counts for vollkasko occurences
        let (mut true_count, mut false_count) = (0, 0);

        for offer in offers {
            if offer.has_vollkasko {
                true_count += 1
            } else {
                false_count += 1
            }
        }

        VollKaskoCount {
            true_count,
            false_count,
        }
    }

    fn to_car_type_count<'a>(offers: impl Iterator<Item = &'a Offer>) -> CarTypeCount {
        // counts for car types
        let (mut small, mut sports, mut luxury, mut family) = (0, 0, 0, 0);

        for offer in offers {
            match offer.car_type {
                CarType::Small => small += 1,
                CarType::Sports => sports += 1,
                CarType::Luxury => luxury += 1,
                CarType::Family => family += 1,
            }
        }

        CarTypeCount {
            small,
            sports,
            luxury,
            family,
        }
    }

    pub fn to_price_ranges_offers<'a>(
        offers: impl Iterator<Item = &'a Offer>,
        price_range_width: u32,
    ) -> Vec<PriceRange> {
        let mut interval_mapping = FxHashMap::new();

        for offer in offers {
            let lower_bound = (offer.price / price_range_width) * price_range_width;
            interval_mapping
                .entry(lower_bound)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        let mut price_ranges = Vec::with_capacity(interval_mapping.len());

        for key in interval_mapping.keys().sorted() {
            let count = interval_mapping[key];
            price_ranges.push(PriceRange {
                start: *key,
                end: *key + price_range_width,
                count,
            });
        }

        price_ranges
    }

    pub fn to_seat_number_offers<'a>(offers: impl Iterator<Item = &'a Offer>) -> Vec<SeatCount> {
        let mut count_map = FxHashMap::new();

        for offer in offers {
            count_map
                .entry(offer.number_seats)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        count_map
            .into_iter()
            .map(|(number_seats, count)| SeatCount {
                number_seats,
                count,
            })
            .collect()
    }

    pub async fn cleanup(&self) -> Result<(), GenericError> {
        {
            let mut region_tree_lock = self.index_tree_lock.write().await;
            region_tree_lock.clear_offers();
        }
        {
            let mut dense_store_lock = self.dense_store_lock.write().await;
            dense_store_lock.all.clear();
        }
        Ok(())
    }
}

pub struct DenseStore {
    pub all: Vec<Offer>,
}

impl DenseStore {
    pub fn new() -> Self {
        Self {
            all: Vec::with_capacity(1 << 25),
        }
    }

    pub fn insert(&mut self, offer: Offer) {
        self.all.push(offer);
    }
}

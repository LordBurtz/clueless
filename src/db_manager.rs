use crate::db_models::{CarType, Offer};
use crate::json_models::{
    CarTypeCount, FreeKilometerRange, GetReponseBodyModel, PriceRange, RequestOffer, ResponseOffer,
    SeatCount, SortOrder, VollKaskoCount,
};
use crate::number_of_days::NumberOfDaysIndex;
use crate::region_hierarchy::{RegionTree, ROOT_REGION};
use crate::GenericError;
use gxhash::{HashMap, HashMapExt};
use tokio::sync::RwLock;

pub struct DBManager {
    pub region_tree_lock: RwLock<RegionTree>,
    pub dense_store_lock: RwLock<DenseStore>,
    pub number_of_days_index_lock: RwLock<NumberOfDaysIndex>,
}

impl CarType {
    fn eqMe(&self, other: &crate::json_models::CarType) -> bool {
        match (self, other) {
            (CarType::Small, crate::json_models::CarType::Small) => true,
            (CarType::Sports, crate::json_models::CarType::Sports) => true,
            (CarType::Luxury, crate::json_models::CarType::Luxury) => true,
            (CarType::Family, crate::json_models::CarType::Family) => true,
            _ => false,
        }
    }
}

impl DBManager {
    pub fn new() -> Self {
        Self {
            region_tree_lock: RegionTree::populate_with_regions(&ROOT_REGION).into(),
            dense_store_lock: DenseStore::new().into(),
            number_of_days_index_lock: NumberOfDaysIndex::new().into(),
        }
    }

    pub async fn query_for(
        &self,
        request_offer: RequestOffer,
    ) -> Result<GetReponseBodyModel, GenericError> {
        let dense_store = self.dense_store_lock.read().await;
        let region_tree = self.region_tree_lock.read().await;
        let number_of_days_index = self.number_of_days_index_lock.read().await;

        let offers = number_of_days_index
            .filter_offers(
                request_offer.number_days,
                region_tree.get_available_offers(request_offer.region_id),
            )
            .map(|offer_idx| &dense_store.all[offer_idx as usize])
            .filter(|a| {
                request_offer.time_range_start <= a.start_date
                    && request_offer.time_range_end >= a.end_date
            })
            .collect::<Vec<_>>();

        if offers.is_empty() {
            return Ok(crate::json_models::GetReponseBodyModel {
                offers: vec![],
                price_ranges: vec![],
                car_type_counts: CarTypeCount {
                    small: 0,
                    sports: 0,
                    luxury: 0,
                    family: 0,
                },
                seats_count: vec![],
                free_kilometer_range: vec![],
                vollkasko_count: VollKaskoCount {
                    true_count: 0,
                    false_count: 0,
                },
            });
        }

        let mut filtered_offers = vec![];
        let mut seats_filter_excl = vec![];
        let mut car_type_filter_excl = vec![];
        let mut has_vollkasko_filter_excl = vec![];
        let mut free_kilometers_filter_excl = vec![];
        let mut price_range_filter_excl = vec![];

        for offer in offers.iter().copied() {
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
                if !offer.car_type.eqMe(&carType) {
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
                (true, true, true, true, true) => filtered_offers.push(offer),
                (true, true, true, true, false) => price_range_filter_excl.push(offer),
                (true, true, true, false, true) => free_kilometers_filter_excl.push(offer),
                (true, true, false, true, true) => has_vollkasko_filter_excl.push(offer),
                (true, false, true, true, true) => car_type_filter_excl.push(offer),
                (false, true, true, true, true) => seats_filter_excl.push(offer),
                _ => {}
            }
        }

        let price_range_bucket = Self::to_price_ranges_offers(
            filtered_offers
                .iter()
                .copied()
                .chain(price_range_filter_excl),
            request_offer.price_range_width,
        );

        let vollkasko_count2 = Self::to_vollkasko_offers(
            filtered_offers
                .iter()
                .copied()
                .chain(has_vollkasko_filter_excl),
        );

        let car_type_count2 =
            Self::to_car_type_count(filtered_offers.iter().copied().chain(car_type_filter_excl));

        let list_for_it = filtered_offers
            .iter()
            .copied()
            .chain(free_kilometers_filter_excl);
        let free_kilometer_bucket =
            Self::to_free_kilometers_offers(list_for_it, request_offer.min_free_kilometer_width);

        //
        // calculate seat count
        //

        let seat_count_vec =
            Self::to_seat_number_offers(filtered_offers.iter().copied().chain(seats_filter_excl));

        //
        // Apply all optional filters, then paginate and return
        //

        let paged_offers = Self::sort_orders_and_paginate(filtered_offers, request_offer);

        Ok(GetReponseBodyModel {
            offers: paged_offers,
            price_ranges: price_range_bucket,
            car_type_counts: car_type_count2,
            seats_count: seat_count_vec,
            free_kilometer_range: free_kilometer_bucket,
            vollkasko_count: vollkasko_count2,
        })
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
                    small: if CarType::Small.eqMe(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        small_excluded
                    },
                    sports: if CarType::Sports.eqMe(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        sports_excluded
                    },
                    luxury: if CarType::Luxury.eqMe(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        luxury_excluded
                    },
                    family: if CarType::Family.eqMe(&filtered_car_type) {
                        filtered_offers_count
                    } else {
                        family_excluded
                    },
                }
            }
        }
    }

    fn sort_orders_and_paginate(
        offers: Vec<&Offer>,
        request_offer: RequestOffer,
    ) -> Vec<ResponseOffer> {
        let mut local_offers = offers.into_iter().cloned().collect::<Vec<Offer>>();
        if local_offers.is_empty() {
            return vec![];
        }

        match request_offer.sort_order {
            SortOrder::PriceAsc => local_offers.sort_by(|a, b| {
                let comp = a.price.cmp(&b.price);
                if comp.is_eq() {
                    return a.id.cmp(&b.id);
                }
                return comp;
            }),
            SortOrder::PriceDesc => local_offers.sort_by(|a, b| {
                let comp = b.price.cmp(&a.price);
                if comp.is_eq() {
                    return a.id.cmp(&b.id);
                }
                return comp;
            }),
        }

        local_offers
            .into_iter()
            .skip(((request_offer.page) * request_offer.page_size) as usize) // pagination starts at 0
            .take(request_offer.page_size as usize)
            .map(|o| ResponseOffer {
                ID: o.id,
                data: o.data,
            })
            .collect()
    }

    fn to_free_kilometers_offers<'a>(
        offers: impl Iterator<Item = &'a Offer>,
        min_free_kilometer_width: u32,
    ) -> Vec<FreeKilometerRange> {
        let mut vec_offers_free_kilometers = offers.collect::<Vec<&Offer>>();

        if vec_offers_free_kilometers.is_empty() {
            return vec![];
        }

        vec_offers_free_kilometers.sort_by(|a, b| a.free_kilometers.cmp(&b.free_kilometers));
        let head = vec_offers_free_kilometers.first().unwrap();

        // magic number access,

        let mut lower_bound_free_km =
            (head.free_kilometers / min_free_kilometer_width) * min_free_kilometer_width;

        // let mut lower_bound_free_km =
        //     first_km.free_kilometers + min_free_kilometer_width;

        let mut km_vec_vec: Vec<FreeKilometerRange> = vec![]; // i literally do not care
        km_vec_vec.push(crate::json_models::FreeKilometerRange {
            start: lower_bound_free_km,
            end: lower_bound_free_km + min_free_kilometer_width,
            count: 0,
        });

        for offer in vec_offers_free_kilometers {
            let FreeKilometerRange { start, end, count } = km_vec_vec.last_mut().unwrap();
            if offer.free_kilometers < *end {
                *count += 1
            } else {
                // TODO: helper method
                lower_bound_free_km =
                    (offer.free_kilometers / min_free_kilometer_width) * min_free_kilometer_width;
                km_vec_vec.push(FreeKilometerRange {
                    start: lower_bound_free_km,
                    end: lower_bound_free_km + min_free_kilometer_width,
                    count: 1,
                });
            }
        }

        return km_vec_vec;
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
        let mut interval_mapping = HashMap::new();

        for offer in offers {
            let lower_bound = (offer.price / price_range_width) * price_range_width;
            interval_mapping
                .entry(lower_bound)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        interval_mapping
            .into_iter()
            .map(|(lower_bound, count)| PriceRange {
                start: lower_bound,
                end: lower_bound + price_range_width,
                count,
            })
            .collect()
    }

    pub fn to_seat_number_offers<'a>(offers: impl Iterator<Item = &'a Offer>) -> Vec<SeatCount> {
        // todo: exchange with better suited data structure
        let mut count_map = HashMap::new();

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
            let mut region_tree_lock = self.region_tree_lock.write().await;
            region_tree_lock.clear_offers();
        }
        {
            let mut dense_store_lock = self.dense_store_lock.write().await;
            dense_store_lock.all.clear();
        }
        {
            let mut number_of_days_index_lock = self.number_of_days_index_lock.write().await;
            number_of_days_index_lock.clear();
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

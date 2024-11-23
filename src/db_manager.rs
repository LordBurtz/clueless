use crate::db_models::{CarType, Offer, RegionHierarchy};
use crate::json_models::*;
use crate::json_models::{
    CarTypeCount, FreeKilometerRange, GetReponseBodyModel, PostRequestBodyModel, PriceRange,
    RequestOffer, ResponseOffer, SeatCount, SortOrder, VollKaskoCount,
};
use crate::GenericError;
use clickhouse::sql::Bind;

#[derive(Clone)]
pub struct DBManager {
    client: clickhouse::Client,
}

const INSERT_QUERY: &str = r#"
    INSERT INTO check24_db
        (ID, data, mostSpecificRegionID,startDate, endDate, numberSeats, price, carType, hasVollkasko, freeKilometers)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
"#;

const SEARCH_QUERY: &str = r#"
    SELECT *
    FROM check24_db
    WHERE
        mostSpecificRegionID = ?1 AND
        ?2 <= startDate AND
        ?3 >= endDate AND
        ?4 <= numberSeats AND
        carType = ?5 AND
        hasVollkasko = ?6 AND
        freeKilometers >= ?7 AND
        price BETWEEN ?9 AND ?0;
"#;

const DELETE_QUERY: &str = r#""DELETE FROM offers"#;

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
    pub fn new(client: clickhouse::Client) -> Self {
        Self { client }
    }

    pub async fn init(&self) -> Result<(), GenericError> {
        self.client
            .query(
                r#"
    CREATE TABLE IF NOT EXISTS offers (
        id VARCHAR(36) PRIMARY KEY,
        data VARCHAR(256) NOT NULL,
        most_specific_region_id UInt32 NOT NULL,
        start_date UInt64 NOT NULL,
        end_date UInt64 NOT NULL,
        number_seats UInt32 NOT NULL,
        price UInt32 NOT NULL,
        car_type UInt8, -- Car type is a string
        has_vollkasko BOOLEAN NOT NULL,
        free_kilometers UInt32 NOT NULL
    )
    ENGINE = MergeTree
    ORDER BY (id, has_vollkasko, car_type, number_seats, most_specific_region_id, free_kilometers, price, start_date, end_date);
"#,
            )
            .execute()
            .await?;

        self.client
            .query(
                r#"
        CREATE TABLE IF NOT EXISTS region_hierarchy (
        ancestor_id UInt32,
        descendant_id UInt32
    ) ENGINE = MergeTree()
    ORDER BY (ancestor_id, descendant_id);
        "#,
            )
            .execute()
            .await?;
        Ok(())
    }

    pub async fn delete_region_hierarchy(&self) -> Result<(), GenericError> {
        self.client
            .query("TRUNCATE TABLE region_hierarchy")
            .execute()
            .await?;
        Ok(())
    }

    pub async fn insert_region_hierarchy(
        &self,
        regions: Vec<RegionHierarchy>,
    ) -> Result<(), GenericError> {
        let mut insert = self.client.insert("region_hierarchy")?;

        for region in regions {
            insert.write(&region).await?;
        }

        insert.end().await?;

        Ok(())
    }

    pub async fn insert_offers(&self, offers: PostRequestBodyModel) -> Result<(), GenericError> {
        let mut insert = self.client.insert("offers")?;

        for offer in offers.offers {
            insert
                .write(&Offer {
                    id: offer.id,
                    data: offer.data,
                    most_specific_region_id: offer.most_specific_region_ID,
                    start_date: offer.start_date,
                    end_date: offer.end_date,
                    number_seats: offer.number_seats,
                    price: offer.price,
                    car_type: offer.car_type.into(),
                    has_vollkasko: offer.has_vollkasko,
                    free_kilometers: offer.free_kilometers,
                })
                .await?;
        }

        insert.end().await?;

        Ok(())
    }

    pub async fn query_for(
        &self,
        request_offer: RequestOffer,
    ) -> Result<GetReponseBodyModel, GenericError> {
        let query_string: String = "SELECT ?fields
        FROM offers
        JOIN region_hierarchy rh ON offers.most_specific_region_id = rh.descendant_id
        WHERE
            rh.ancestor_id = ? AND
            ? <= start_date AND
            ? >= end_date AND
            end_date - start_date >= ?"
            .to_string();

        let mut query = self
            .client
            .query(&query_string)
            .bind(request_offer.region_id)
            .bind(request_offer.time_range_start)
            .bind(request_offer.time_range_end)
            .bind(request_offer.number_days * 24 * 60 * 60 * 1000);


        //
        //Results of db query
        //
        let offers = query.fetch_all::<Offer>().await?;

        // todo passt eig so
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

        for offer in &offers {
            let mut seats_incl = true;
            let mut car_type_incl = true;
            let mut has_vollkasko_incl = true;
            let mut free_kilometers_incl = true;
            let mut price_range_incl = true;

            if let Some(minNumberOfSeats) = request_offer.min_number_seats {
                if offer.number_seats < minNumberOfSeats {
                    seats_incl = false;
                }
            }
            if let Some(carType) = request_offer.car_type {
                if offer.car_type.eqMe(&carType) {
                    car_type_incl = false
                }
            }
            if let Some(hasVollkasko) = request_offer.only_vollkasko {
                if offer.has_vollkasko != hasVollkasko {
                    has_vollkasko_incl = false;
                }
            }
            if let Some(minFreeKilometers) = request_offer.min_free_kilometer {
                if offer.free_kilometers < minFreeKilometers {
                    free_kilometers_incl = false;
                }
            }
            if let Some(maxPrice) = request_offer.max_price {
                if (maxPrice <= offer.price) {
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
                has_vollkasko_incl,
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

        let filtered_offers_count = filtered_offers.len() as u32;

        //
        // Count vollkasko and car type options
        //

        let vollkasko_count = match request_offer.only_vollkasko {
            None => VollKaskoCount {
                true_count: filtered_offers
                    .iter()
                    .filter(|offer| offer.has_vollkasko)
                    .count() as u32,
                false_count: filtered_offers
                    .iter()
                    .filter(|offer| !offer.has_vollkasko)
                    .count() as u32,
            },
            Some(only_vollkasko) => VollKaskoCount {
                true_count: filtered_offers_count
                    + if only_vollkasko {
                    0
                } else {
                    has_vollkasko_filter_excl.len() as u32
                },
                false_count: filtered_offers_count
                    + if !only_vollkasko {
                    0
                } else {
                    has_vollkasko_filter_excl.len() as u32
                },
            },
        };

        // let car_type_count =
            // Self::get_car_type_count(&offers, &car_type_filter_excl, &request_offer);

        //
        // price range slicing
        //

        let price_range_bucket = Self::toPriceRangesOffers(
            offers.iter().chain(price_range_filter_excl),
            request_offer.price_range_width,
        );

        //
        // free kilometers slicing
        //

        let vollkasko_count2 =
            Self::toVollkaskoOffers(offers.iter().chain(has_vollkasko_filter_excl));
        // let defsi = car_type_count;
        let sid = vollkasko_count;
        let car_type_count2 = Self::to_car_type_count(offers.iter().chain(car_type_filter_excl));


        let listForIt = (filtered_offers.iter().copied().chain(free_kilometers_filter_excl));
        let free_kilometer_bucket = Self::toFreeKilometersOffers(
            listForIt,
            request_offer.min_free_kilometer_width,
        );


        //
        // calculate seat count
        //

        let seatCountVec =
            Self::toSeatNumberOffers(filtered_offers.iter().copied().chain(seats_filter_excl));

        //
        // Apply all optional filters, then paginate and return
        //

        let paged_offers = Self::sortOrdersAndPaginate(filtered_offers, request_offer);

        return Ok(GetReponseBodyModel {
            offers: paged_offers,
            price_ranges: price_range_bucket,
            car_type_counts: car_type_count2,
            seats_count: seatCountVec,
            free_kilometer_range: free_kilometer_bucket,
            vollkasko_count: vollkasko_count2,
        });
    }

    // fn get_car_type_count(
    //     offers: &[Offer],
    //     excluded_offers: &[Offer],
    //     request_offer: &RequestOffer,
    // ) -> CarTypeCount {
    //     let filtered_offers_count = offers.len() as u32;
    //     match request_offer.car_type {
    //         None => {
    //             let mut small = 0;
    //             let mut sports = 0;
    //             let mut luxury = 0;
    //             let mut family = 0;
    //             for offer in offers {
    //                 match offer.car_type {
    //                     CarType::Small => small += 1,
    //                     CarType::Sports => sports += 1,
    //                     CarType::Luxury => luxury += 1,
    //                     CarType::Family => family += 1
    //                 }
    //             }
    //             CarTypeCount {
    //                 small,
    //                 sports,
    //                 luxury,
    //                 family,
    //             }
    //         }
    //         Some(filtered_car_type) => {
    //             let mut small_excluded = 0;
    //             let mut family_excluded = 0;
    //             let mut luxury_excluded = 0;
    //             let mut sports_excluded = 0;
    //             for offer in excluded_offers {
    //                 match offer.car_type {
    //                     CarType::Small => small_excluded += 1,
    //                     CarType::Sports => sports_excluded += 1,
    //                     CarType::Luxury => luxury_excluded += 1,
    //                     CarType::Family => family_excluded += 1,
    //                 };
    //             }
    //             CarTypeCount {
    //                 small: if (filtered_car_type.into() == CarType::Small) {
    //                     filtered_offers_count
    //                 } else {
    //                     small_excluded
    //                 },
    //                 sports: if (filtered_car_type.into() == CarType::Sports) {
    //                     filtered_offers_count
    //                 } else {
    //                     sports_excluded
    //                 },
    //                 luxury: if (filtered_car_type.into() == CarType::Luxury) {
    //                     filtered_offers_count
    //                 } else {
    //                     luxury_excluded
    //                 },
    //                 family: if (filtered_car_type.into() == CarType::Family) {
    //                     filtered_offers_count
    //                 } else {
    //                     family_excluded
    //                 },
    //             }
    //         }
    //     }
    // }

    fn sortOrdersAndPaginate(
        offers: Vec<&Offer>,
        request_offer: RequestOffer,
    ) -> Vec<ResponseOffer> {
        let mut local_offers = offers.into_iter().cloned().collect::<Vec<Offer>>();
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

    fn toFreeKilometersOffers<'a>(
        offers: impl Iterator<Item=&'a Offer>,
        min_free_kilometer_width: u32,
    ) -> Vec<FreeKilometerRange> {
        let mut vec_offers_free_kilometers = offers.collect::<Vec<&Offer>>();
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

    fn toVollkaskoOffers<'a>(offers: impl Iterator<Item=&'a Offer>) -> VollKaskoCount {
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

    fn to_car_type_count<'a>(offers: impl Iterator<Item=&'a Offer>) -> CarTypeCount {
        // counts for car types
        let (mut small, mut sports, mut luxury, mut family) = (0, 0, 0, 0);

        for offer in offers {
            match (offer.car_type) {
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

    pub fn toPriceRangesOffers<'a>(
        offers: impl Iterator<Item=&'a Offer>,
        price_range_width: u32,
    ) -> Vec<PriceRange> {
        let mut vec_offers_price_range = offers.collect::<Vec<&Offer>>();

        vec_offers_price_range.sort_by(|a, b| a.price.cmp(&b.price));
        let head = vec_offers_price_range.first().unwrap();

        // magic number access,

        // TODO: rename later
        let mut lower_bound_free_km = (head.price / price_range_width) * price_range_width;

        // let mut lower_bound_free_km =
        //     first_km.free_kilometers + min_free_kilometer_width;

        let mut km_vec_vec: Vec<PriceRange> = vec![]; // i literally do not care
        km_vec_vec.push(crate::json_models::PriceRange {
            start: lower_bound_free_km,
            end: lower_bound_free_km + price_range_width,
            count: 0,
        });

        for offer in vec_offers_price_range {
            let PriceRange { start, end, count } = km_vec_vec.last_mut().unwrap();
            if offer.price < *end {
                *count += 1
            } else {
                // TODO: helper method
                lower_bound_free_km = (offer.price / price_range_width) * price_range_width;
                km_vec_vec.push(PriceRange {
                    start: lower_bound_free_km,
                    end: lower_bound_free_km + price_range_width,
                    count: 1,
                });
            }
        }

        return km_vec_vec;
    }

    pub fn toSeatNumberOffers<'a>(offers: impl Iterator<Item=&'a Offer>) -> Vec<SeatCount> {
        let mut vec_number_seats = offers.collect::<Vec<&Offer>>();
        vec_number_seats.sort_by(|a, b| a.number_seats.cmp(&b.number_seats));

        return vec_number_seats
            .chunk_by(|a, b| a.number_seats == b.number_seats)
            .map(|chunk| {
                let number_seats = chunk.first().unwrap().number_seats;
                let count = chunk.len() as u32;
                SeatCount {
                    number_seats,
                    count,
                }
            })
            .collect::<Vec<SeatCount>>();
    }

    pub async fn cleanup(&self) -> Result<(), GenericError> {
        self.client.query("TRUNCATE TABLE offers").execute().await?;
        Ok(())
    }
}

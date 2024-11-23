use clickhouse::query::Query;
use crate::json_models::*;
use crate::json_models::{CarTypeCount, GetReponseBodyModel};
use crate::GenericError;
use clickhouse::sql::Bind;

#[derive(Clone)]
pub struct DBManager {
    client: clickhouse::Client,
}

const TABLE_INIT: &str = r#"
    CREATE TABLE IF NOT EXISTS check24_db (
        ID VARCHAR(36) PRIMARY KEY, -- UUID format
        data VARCHAR(256) NOT NULL,
        mostSpecificRegionID INT NOT NULL,
        startDate BIGINT NOT NULL,
        endDate BIGINT NOT NULL,
        numberSeats INT NOT NULL,
        price INT NOT NULL,
        carType INT, -- Car type is a string
        hasVollkasko BOOLEAN NOT NULL,
        freeKilometers INT NOT NULL
    );
"#;

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

impl DBManager {
    pub fn new(client: clickhouse::Client) -> Self {
        Self { client }
    }

    pub async fn insert_offers(&self, offers: PostRequestBodyModel) -> Result<(), GenericError> {
        let mut insert = self.client.insert("offers")?;

        for offer in offers.offers {
            insert
                .write(&Offer {
                    id: offer.id,
                    data: offer.data,
                    most_specific_region_ID: offer.most_specific_region_ID,
                    start_date: offer.start_date,
                    end_date: offer.end_date,
                    number_seats: offer.number_seats,
                    price: offer.price,
                    car_type: offer.car_type,
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
        let mut query_parameters: Vec<impl Bind> = vec![];
        let mut query_string: String = " SELECT ?fields
        FROM offers
        WHERE
        mostSpecificRegionID = ? AND
            ? <= startDate AND
            ? >= endDate".to_string();
        
        if let Some(numberOfSeats) = request_offer.min_number_seats {
            query_string.push_str("AND ? <= numberSeats");
            query_parameters.push(numberOfSeats)
        }
        if let Some(carType) = request_offer.car_type {
            query_string.push_str(" AND carType = ?");
            query_parameters.push(carType);
        }
        if let Some(hasVollkasko) = request_offer.only_vollkasko {
            query_string.push_str(" AND hasVollkasko = ?");
            query_parameters.push(hasVollkasko);
        }
        if let Some(freeKilometers) = request_offer.min_free_kilometer {
            query_string.push_str(" AND freeKilometers >= ?");
            query_parameters.push(freeKilometers);
        }
        if let Some(minPrice) = request_offer.min_price {
            query_parameters.push(minPrice);
            if let Some(maxPrice) = request_offer.max_price {
                query_string.push_str(" AND price BETWEEN ? AND ?");
                query_parameters.push(maxPrice);
            } else {
                query_string.push_str(" AND price >= ?");
            }
        }
        let mut query = self
            .client
            .query(query_parameters.as_slice());

        for param in query_parameters {
            query = query.bind(&param);
        }

        let mut offers = query
            .fetch_all::<Offer>()
            .await?;

        // counts for vollkasko occurences
        let (mut true_count, mut false_count) = (0, 0);

        // counts for car types
        let (mut small, mut sports, mut luxury, mut family) = (0, 0, 0, 0);

        for offer in &offers {
            if offer.has_vollkasko {
                true_count += 1
            } else {
                false_count += 1
            }

            match (offer.car_type) {
                CarType::Small => small += 1,
                CarType::Sports => sports += 1,
                CarType::Luxury => luxury += 1,
                CarType::Family => family += 1,
            }
        }

        //
        // price range slicing
        //

        // TODO: only once
        let mut vec_offers_price_range = offers.clone();

        vec_offers_price_range.sort_by(|a, b| a.price.cmp(&b.price));
        let (head_price_range, tail_price_range) = vec_offers_price_range.split_at(1);

        // magic number access,
        let first_price_offer = head_price_range.first().unwrap();

        let mut lower_bound_price_range = first_price_offer.price + request_offer.price_range_width;

        let mut price_vec_vec: Vec<Vec<&Offer>> = vec![]; // i literally do not care
        let mut vec_number_seats = offers.clone();
        let mut vec_offers_free_kilometers = offers.clone();
        price_vec_vec.push(vec![first_price_offer]);

        for offer in tail_price_range {
            if offer.price < lower_bound_price_range {
                price_vec_vec.last_mut().unwrap().push(offer);
            } else {
                lower_bound_price_range += request_offer.price_range_width;
                price_vec_vec.push(vec![offer]);
            }
        }

        let price_range_bucket = price_vec_vec
            .iter()
            .map(|a| {
                let start = a.first().unwrap().price;
                let end = a.last().unwrap().price;
                let count = a.len() as i32;
                PriceRange { start, end, count }
            })
            .collect();

        //
        // Sort orders and paginate
        //

        match request_offer.sort_order {
            SortOrder::PriceAsc => offers.sort_by(|a, b| a.number_seats.cmp(&b.number_seats)),
            SortOrder::PriceDesc => offers.sort_by(|a, b| b.number_seats.cmp(&a.number_seats)),
        }

        //
        // 190 |   ...       .into_iter()
        //     |              ----------- `Iterator::Item` remains `&[Offer]` here
        // note: required by a bound in `std::iter::Iterator::collect`
        // --> /rustc/f6e511eec7342f59a25f7c0534f1dbea00d01b14/library/core/src/iter/traits/iterator.rs:1996:5
        let paged_offers: Vec<ResponseOffer> = offers
            .into_iter()
            //TODO: double check if pagination starts at 1
            .skip(((request_offer.page - 1) * request_offer.page_size) as usize)
            .take(request_offer.page_size as usize)
            .map(|o| ResponseOffer {
                ID: o.id,
                data: o.data,
            })
            .collect();

        //
        // number seats slicing
        //

        // TODO: only once
        // setup a list of offers

        vec_number_seats.sort_by(|a, b| a.number_seats.cmp(&b.number_seats));

        let seatCountVec = vec_number_seats
            .chunk_by(|a, b| a.number_seats == b.number_seats)
            .map(|chunk| {
                let number_seats = chunk.first().unwrap().number_seats;
                let count = chunk.len() as i32;
                SeatCount {
                    number_seats,
                    count,
                }
            })
            .collect::<Vec<SeatCount>>();

        //
        // free kilometers slicing
        //

        // TODO: only once
        // setup a list of offers

        vec_offers_free_kilometers.sort_by(|a, b| a.free_kilometers.cmp(&b.free_kilometers));
        let (head_free_km, tail_free_km) = vec_offers_free_kilometers.split_at(1);

        // magic number access,
        let first_km = head_free_km.first().unwrap();

        let mut lower_bound_free_km =
            first_km.free_kilometers + request_offer.min_free_kilometer_width;

        let mut km_vec_vec: Vec<Vec<&Offer>> = vec![]; // i literally do not care
        km_vec_vec.push(vec![first_km]);

        for offer in tail_free_km {
            if offer.free_kilometers < lower_bound_free_km {
                km_vec_vec.last_mut().unwrap().push(offer);
            } else {
                lower_bound_free_km += request_offer.min_free_kilometer_width;
                km_vec_vec.push(vec![offer]);
            }
        }

        let free_kilometer_range_bucket = km_vec_vec
            .iter()
            .map(|a| {
                let start = a.first().unwrap().free_kilometers;
                let end = a.last().unwrap().free_kilometers;
                let count = a.len() as i32;
                FreeKilometerRange { start, end, count }
            })
            .collect();

        return Ok(GetReponseBodyModel {
            offers: paged_offers,
            price_ranges: price_range_bucket,
            car_type_counts: CarTypeCount {
                small,
                sports,
                luxury,
                family,
            },
            seats_count: seatCountVec,
            free_kilometer_range: free_kilometer_range_bucket,
            vollkasko_count: VollKaskoCount {
                true_count,
                false_count,
            },
        });
    }

    pub async fn cleanup(&self) -> Result<(), GenericError> {
        self.client.query("TRUNCATE TABLE offers").execute().await?;
        Ok(())
    }
}

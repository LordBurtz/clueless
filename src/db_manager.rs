use std::sync::Mutex;
use std::collections::HashMap;
use crate::GenericError;
use crate::json_models::*;
use rusqlite::*;
use crate::json_models::{GetReponseBodyModel, CarTypeCount};

pub struct DBManager {
    conn: Connection,
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
        regionID = ? AND
        ? <= startDate AND
        ? >= endDate AND
        ? <= numberSeats AND
        carType = ? AND
        hasVollkasko = ? AND
        freeKilometers >= ? AND
        price BETWEEN ? AND ?
"#;

const DELETE_QUERY: &str = r#""DELETE FROM check24_db"#;

pub fn open_connection() -> Result<Connection, Error> {
    let conn = Connection::open_in_memory()?;
    conn.execute(TABLE_INIT, [])?;
    Ok(conn)
}

impl DBManager {
    pub fn new_lock(conn: Connection) -> Mutex<Self> {
        Mutex::new(DBManager {conn})
    }

    pub fn insert_offers(&self, offers: PostRequestBodyModel) -> Result<(), GenericError> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(INSERT_QUERY)?;

        for offer in offers.offers {
            stmt.execute(params![
                offer.id,
                offer.data,
                offer.most_specific_region_ID,
                offer.start_date,
                offer.end_date,
                offer.number_seats,
                offer.price,
                offer.car_type.as_u8(),
                offer.has_vollkasko,
                offer.free_kilometers
            ])?;
        }

        Ok(())
    }

    pub fn query_mock(&self, request_offer: RequestOffer) -> Result<(), Error> {
        Ok(())
    }

    pub fn query_for(&self, request_offer: RequestOffer) -> Result<GetReponseBodyModel, Error> {


        let conn = &self.conn;
        let mut stmt = conn.prepare(SEARCH_QUERY)?;

        let mut results = stmt.query_map(params![request_offer.region_id, request_offer.time_range_start, request_offer.time_range_end,request_offer.min_number_seats, request_offer.car_type.unwrap().as_u8(), request_offer.only_vollkasko, request_offer.min_price, request_offer.max_price], |row| {
            Ok(Offer{
                id: row.get(0)?,
                data: row.get(1)?,
                most_specific_region_ID: row.get(2)?,
                start_date: row.get(3)?,
                end_date: row.get(4)?,
                number_seats: row.get(5)?,
                price: row.get(6)?,
                car_type: CarType::to_enum(row.get(7)?),
                has_vollkasko: row.get(8)?,
                free_kilometers: row.get(9)?
            })
        })?;

        // counts for vollkasko occurences
        let (mut true_count,mut false_count) = (0, 0);

        // counts for car types
        let (mut small, mut sports, mut luxury, mut family) = (0,0,0,0);



        for offer in results.by_ref() {
            let off = offer?;

            if off.has_vollkasko {
                true_count += 1
            } else { false_count += 1 }

            match (off.car_type) {
                CarType::Small => {small += 1}
                CarType::Sports => {sports += 1}
                CarType::Luxury => luxury += 1,
                CarType::Family => {family += 1}
            }

        }

        //
        // price range slicing
        //

        // TODO: only once
        // setup a list of offers
        let mut vec_offers_price_range = results.by_ref()
            .map(|a| a.ok())
            .filter_map(|a| a)
            .collect::<Vec<Offer>>()
            ;

        vec_offers_price_range
            .sort_by(|a, b| a.price.cmp(&b.price));
        let (head_price_range, tail_price_range) = vec_offers_price_range.split_at(0);

        // magic number access,
        let first_price_offer = head_price_range.first().unwrap();

        let mut lower_bound_price_range = first_price_offer.price + request_offer.price_range_width;

        let mut price_vec_vec: Vec<Vec<&Offer>> = vec![]; // i literally do not care
        price_vec_vec.push(vec![first_price_offer]);

        for offer in tail_price_range {
            if offer.price < lower_bound_price_range {
                price_vec_vec.last_mut().unwrap()
                    .push(offer);
            } else {
                lower_bound_price_range += request_offer.price_range_width;
                price_vec_vec.push(vec![offer]);
            }
        }

        let price_range_bucket = price_vec_vec.iter().map(|a| {
            let start = a.first().unwrap().price;
            let end = a.last().unwrap().price;
            let count = a.len() as i32;
            PriceRange{start, end, count}
        }).collect();


        //
        // number seats slicing
        //

        // TODO: only once
        // setup a list of offers
        let mut vec_number_seats = results.by_ref()
            .map(|a| a.ok())
            .filter_map(|a| a)
            .collect::<Vec<Offer>>()
            ;

        vec_number_seats
            .sort_by(|a, b| a.number_seats.cmp(&b.number_seats));

        let seatCountVec = vec_number_seats.chunk_by(|a, b| {
            a.number_seats == b.number_seats
        }).map(|chunk| {
            let number_seats = chunk.first().unwrap().number_seats;
            let count = chunk.len() as i32;
            SeatCount{number_seats, count}
        }).collect::<Vec<SeatCount>>();


        //
        // free kilometers slicing
        //

        // TODO: only once
        // setup a list of offers
        let mut vec_offers_free_kilometers = results.by_ref()
            .map(|a| a.ok())
            .filter_map(|a| a)
            .collect::<Vec<Offer>>()
            ;

        vec_offers_free_kilometers
            .sort_by(|a, b| a.free_kilometers.cmp(&b.free_kilometers));
        let (head_free_km, tail_free_km) = vec_offers_free_kilometers.split_at(0);

        // magic number access,
        let first_km = head_free_km.first().unwrap();

        let mut lower_bound_free_km = first_km.free_kilometers + request_offer.min_free_kilometer_width;

        let mut km_vec_vec: Vec<Vec<&Offer>> = vec![]; // i literally do not care
        km_vec_vec.push(vec![first_km]);

        for offer in tail_free_km {
            if offer.free_kilometers < lower_bound_free_km {
                km_vec_vec.last_mut().unwrap()
                    .push(offer);
            } else {
                lower_bound_free_km += request_offer.min_free_kilometer_width;
                km_vec_vec.push(vec![offer]);
            }
        }

        let free_kilometer_range_bucket = km_vec_vec.iter().map(|a| {
            let start = a.first().unwrap().free_kilometers;
            let end = a.last().unwrap().free_kilometers;
            let count = a.len() as i32;
            FreeKilometerRange{start, end, count}
        }).collect();

        return Ok(GetReponseBodyModel{
            offers: vec![],
            price_ranges: price_range_bucket,
            car_type_counts: CarTypeCount {
                small, sports, luxury, family
            },
            seats_count: seatCountVec,
            free_kilometer_range: free_kilometer_range_bucket,
            vollkasko_count: VollKaskoCount {
                true_count, false_count
            }
        });
    }

    pub fn cleanup(&self) -> Result<usize> {
        self.conn.execute(DELETE_QUERY, [])
    }
}
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
        let mut seat_amount: HashMap<i32, i32> = HashMap::new();
        let mut price_ranges: HashMap<i32,i32> = HashMap::new();

        let mut response_offers_list: Vec<ResponseOffer> = Vec::new();
        let mut price_range_list: Vec<i32> = vec![0];

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

            response_offers_list.push(ResponseOffer{ID: off.id, data: off.data});

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

        // free kilometers slicing
        for offer in results.by_ref()
            .filter(|a| {
                return if let Some(_) = a {
                    true
                } else {
                    false
                }
            })
            .is_sorted_by(|a, b| {
                if let Some(a_ok) = a {
                    if let Some(b_ok) = b {
                        &a.free_kilometers < &b.free_kilometers
                    }
                }
                let a_ok = a?;
                let b_ok = b?;
                &a.free_kilometers < &b.free_kilometers
            }) {

        }




        return Ok(GetReponseBodyModel{
            offers: response_offers_list,
            price_ranges: vec![],
            car_type_counts: CarTypeCount {
                small, sports, luxury, family
            },
            seats_count: vec![],
            free_kilometer_range: vec![],
            vollkasko_count: VollKaskoCount {
                true_count, false_count
            }
        });
    }

    pub fn cleanup(&self) -> Result<usize> {
        self.conn.execute(DELETE_QUERY, [])
    }
}
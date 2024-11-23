use rusqlite::ToSql;
use rusqlite::types::{FromSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequestOffer {
    #[serde(rename = "regionID")]
    pub region_id: u8,
    pub time_range_start: i64,
    pub time_range_end: i64,
    pub number_days: i32,
    pub sort_order: SortOrder,
    pub page: i32,
    pub page_size: i32,
    pub price_range_width: i32,
    pub min_free_kilometer_width: i32,
    pub min_number_seats: Option<i32>,
    pub min_price: Option<i32>,
    pub max_price: Option<i32>,
    pub car_type: Option<CarType>,
    pub only_vollkasko: Option<bool>,
    pub min_free_kilometer: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum CarType {
    Small = 0,
    Sports = 1,
    Luxury = 2,
    Family = 3,
}

impl CarType {
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn to_enum(num: u8) -> CarType {
        match num {
            0 => CarType::Small,
            1 => CarType::Sports,
            2 => CarType::Luxury,
            _ => CarType::Family,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum SortOrder {
    PriceAsc,
    PriceDesc,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetReponseBodyModel {
    pub offers: Vec<ResponseOffer>,
    pub price_ranges: Vec<PriceRange>,
    pub car_type_counts: CarTypeCount,
    pub seats_count: Vec<SeatCount>,
    pub free_kilometer_range: Vec<FreeKilometerRange>,
    pub vollkasko_count: VollKaskoCount,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseOffer {
    pub ID: String,
    pub data: String // encoded as base64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PriceRange {
    start: i32,
    end: i32,
    count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CarTypeCount {
    pub small: i32,
    pub sports: i32,
    pub luxury: i32,
    pub family: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SeatCount {
    count: i32,
    number_seats: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FreeKilometerRange {
    start: i32,
    end: i32,
    count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VollKaskoCount {
    pub true_count: i32,
    pub false_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostRequestBodyModel {
    pub offers: Vec<Offer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    #[serde(rename = "ID")]
    pub id: String,
    // TODO: optimize?
    pub data: String, // base64 encoded 256 Byte array
    pub most_specific_region_ID: i32,
    pub start_date: i64,
    pub end_date: i64,
    pub number_seats: i32,
    pub price: i32,
    pub car_type: CarType,
    pub has_vollkasko: bool,
    pub free_kilometers: i32,
}


pub const SAMPLE_GET_RESPONSE: &str = r#"
{
  "offers": [
    {
      "ID": "01934a57-7988-7879-bb9b-e03bd4e77b9d",
      "data": "string"
    }
  ],
  "priceRanges": [
    {
      "start": 10000,
      "end": 15000,
      "count": 4
    }
  ],
  "carTypeCounts": {
    "small": 1,
    "sports": 2,
    "luxury": 1,
    "family": 0
  },
  "seatsCount": [
    {
      "numberSeats": 5,
      "count": 4
    }
  ],
  "freeKilometerRange": [
    {
      "start": 100,
      "end": 150,
      "count": 4
    }
  ],
  "vollkaskoCount": {
    "trueCount": 3,
    "falseCount": 1
  }
}
"#;

pub const SAMPLE_POST_REQUEST: &str = r#"
{
  "offers": [
    {
      "ID": "01934a57-7988-7879-bb9b-e03bd4e77b9d",
      "data": "string",
      "mostSpecificRegionID": 5,
      "startDate": 1732104000000,
      "endDate": 1732449600000,
      "numberSeats": 5,
      "price": 10000,
      "carType": "luxury",
      "hasVollkasko": true,
      "freeKilometers": 120
    }
  ]
}
"#;

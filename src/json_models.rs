use crate::db_models;
use sonic_rs::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequestOffer {
    #[serde(rename = "regionID")]
    pub region_id: u8,
    pub time_range_start: u64,
    pub time_range_end: u64,
    pub number_days: u32,
    pub sort_order: SortOrder,
    pub page: u32,
    pub page_size: u32,
    pub price_range_width: u32,
    pub min_free_kilometer_width: u32,
    pub min_number_seats: Option<u32>,
    pub min_price: Option<u32>,
    pub max_price: Option<u32>,
    pub car_type: Option<CarType>,
    pub only_vollkasko: Option<bool>,
    pub min_free_kilometer: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CarType {
    Small,
    Sports,
    Luxury,
    Family,
}

// impl From<CarType> for db_models::CarType {
//     fn from(car_type: CarType) -> Self {
//         match car_type {
//             CarType::Small => db_models::CarType::Small,
//             CarType::Sports => db_models::CarType::Sports,
//             CarType::Luxury => db_models::CarType::Luxury,
//             CarType::Family => db_models::CarType::Family,
//         }
//     }
// }

// impl PartialEq<CarType> for CarType {
//     fn eq(&self, other: &CarType) -> bool {
//         match (self, other) {
//             (CarType::Small, CarType::Small) => true,
//             (CarType::Sports, CarType::Sports) => true,
//             (CarType::Luxury, CarType::Luxury) => true,
//             (CarType::Family, CarType::Family) => true,
//             (_, _) => false,
//         }
//     }
// }

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
    pub data: String, // encoded as base64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PriceRange {
    pub start: u32,
    pub end: u32,
    pub count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CarTypeCount {
    pub small: u32,
    pub sports: u32,
    pub luxury: u32,
    pub family: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SeatCount {
    pub count: u32,
    pub number_seats: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FreeKilometerRange {
    pub start: u32,
    pub end: u32,
    pub count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VollKaskoCount {
    pub true_count: u32,
    pub false_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Offer<'a> {
    #[serde(rename = "ID")]
    pub id: &'a str,
    // TODO: optimize?
    pub data: String, // base64 encoded 256 Byte array
    pub most_specific_region_ID: u32,
    pub start_date: u64,
    pub end_date: u64,
    pub number_seats: u32,
    pub price: u32,
    pub car_type: CarType,
    pub has_vollkasko: bool,
    pub free_kilometers: u32,
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

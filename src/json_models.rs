use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequestOffer {
    region_id: u8,
    time_range_start: i64,
    time_range_end: i64,
    number_days: i32,
    sort_order: SortOrder,
    page: i32,
    page_size: i32,
    price_range_width: i32,
    min_free_kilometer_width: i32,
    min_number_seats: Option<i32>,
    min_price: Option<i32>,
    max_price: Option<i32>,
    car_type: Option<CarType>,
    only_vollkasko: Option<bool>,
    min_free_kilometer: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum CarType {
    Small,
    Sports,
    Luxury,
    Family
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum SortOrder {
    PriceAsc,
    PriceDesc,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetReponseBodyModel {
    offers: Vec<ResponseOffer>,
    price_ranges: Vec<PriceRange>,
    car_type_counts: CarTypeCount,
    seats_count: Vec<SeatCount>,
    free_kilometer_range: Vec<FreeKilometerRange>,
    vollkasko_count: VollKaskoCount,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResponseOffer {
    ID: String,
    data: String // encoded as base64
}

#[derive(Serialize, Deserialize, Debug)]
struct PriceRange {
    start: i32,
    end: i32,
    count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct CarTypeCount {
    small: i32,
    sports: i32,
    luxury: i32,
    family: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SeatCount {
    count: i32,
    number_seats: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct FreeKilometerRange {
    start: i32,
    end: i32,
    count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VollKaskoCount {
    true_count: i32,
    false_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostRequestBodyModel {
    offers: Vec<Offer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Offer {
    #[serde(rename = "ID")]
    id: String,
    // TODO: optimize?
    data: String, // base64 encoded 256 Byte array
    most_specific_region_ID: i32,
    start_date: i64,
    end_date: i64,
    number_seats: i32,
    price: i32,
    car_type: CarType,
    has_vollkasko: bool,
    free_kilometers: i32,
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

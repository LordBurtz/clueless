use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{char, digit1},
    combinator::{map_res, opt},
    multi::separated_list1,
    sequence::{separated_pair, terminated},
    IResult,
};
use std::str::FromStr;
use crate::GenericError;
use crate::json_models::SortOrder;
use crate::json_models::CarType;
use crate::json_models::RequestOffer;


impl FromStr for SortOrder {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "price-asc" => Ok(SortOrder::PriceAsc),
            "price-desc" => Ok(SortOrder::PriceDesc),
            _ => Err(()),
        }
    }
}

impl FromStr for CarType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "small" => Ok(CarType::Small),
            "sports" => Ok(CarType::Sports),
            "luxury" => Ok(CarType::Luxury),
            "family" => Ok(CarType::Family),
            _ => Err(()),
        }
    }
}

// #[derive(Debug)]
// pub struct RequestOffer {
//     pub region_id: u8,
//     pub time_range_start: u64,
//     pub time_range_end: u64,
//     pub number_days: u32,
//     pub sort_order: SortOrder,
//     pub page: u32,
//     pub page_size: u32,
//     pub price_range_width: u32,
//     pub min_free_kilometer_width: u32,
//     pub min_number_seats: Option<u32>,
//     pub min_price: Option<u32>,
//     pub max_price: Option<u32>,
//     pub car_type: Option<CarType>,
//     pub only_vollkasko: Option<bool>,
//     pub min_free_kilometer: Option<u32>,
// }

fn parse_key_value(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(take_until("="), char('='), take_until("&"))(input)
}

fn parse_query_string(input: &str) -> IResult<&str, Vec<(&str, &str)>> {
    separated_list1(char('&'), terminated(parse_key_value, opt(char('&'))))(input)
}

pub fn parse_request_offer(query: &str) -> Option<RequestOffer> {
    let (_, pairs) = parse_query_string(query).ok()?;
    let mut region_id = None;
    let mut time_range_start = None;
    let mut time_range_end = None;
    let mut number_days = None;
    let mut sort_order = None;
    let mut page = None;
    let mut page_size = None;
    let mut price_range_width = None;
    let mut min_free_kilometer_width = None;

    for (key, value) in pairs {
        match key {
            "regionID" => region_id = value.parse::<u8>().ok(),
            "timeRangeStart" => time_range_start = value.parse::<u64>().ok(),
            "timeRangeEnd" => time_range_end = value.parse::<u64>().ok(),
            "numberDays" => number_days = value.parse::<u32>().ok(),
            "sortOrder" => sort_order = value.parse::<SortOrder>().ok(),
            "page" => page = value.parse::<u32>().ok(),
            "pageSize" => page_size = value.parse::<u32>().ok(),
            "priceRangeWidth" => price_range_width = value.parse::<u32>().ok(),
            "minFreeKilometerWidth" => min_free_kilometer_width = value.parse::<u32>().ok(),
            _ => {}
        }
    }

    Some(RequestOffer {
        region_id: region_id?,
        time_range_start: time_range_start?,
        time_range_end: time_range_end?,
        number_days: number_days?,
        sort_order: sort_order?,
        page: page?,
        page_size: page_size?,
        price_range_width: price_range_width?,
        min_free_kilometer_width: min_free_kilometer_width?,
        min_number_seats: None,
        min_price: None,
        max_price: None,
        car_type: None,
        only_vollkasko: None,
        min_free_kilometer: None,
    })
}
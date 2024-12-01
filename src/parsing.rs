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
use crate::json_models::{GetReponseBodyModel, SortOrder};
use crate::json_models::CarType;
use crate::json_models::RequestOffer;
use crate::json_models::SortOrder::PriceAsc;

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

impl SortOrder {
    #[inline(always)]
    fn fast_from_str(s: &str) -> Self {
        match s {
            "price-desc" => SortOrder::PriceDesc,
            _ => PriceAsc,
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

pub fn parse_request_offer(query: &str) -> RequestOffer {
    // let (_, pairs) = parse_query_string(query).ok()?;
    let mut region_id = 0u8;
    let mut time_range_start = 0u64;
    let mut time_range_end = 0u64;
    let mut number_days = 0u32;
    let mut sort_order = SortOrder::PriceAsc; // Default
    let mut page = 0u32;
    let mut page_size = 0u32;
    let mut price_range_width = 0u32;
    let mut min_free_kilometer_width = 0u32;
    let mut min_number_seats = None;
    let mut min_price = None;
    let mut max_price = None;
    let mut car_type = None;
    let mut only_vollkasko = None;
    let mut min_free_kilometers = None;

    query.split('&').for_each(|pair | {
        // oh no
        unsafe {
            //anyways
                let (key, value) = {
                    let mut split = pair.splitn(2, '=');
                    (split.next().unwrap_unchecked(), split.next().unwrap_unchecked())
                };

                match key {
                    "regionID" => region_id = value.parse::<u8>().unwrap_unchecked(),
                    "timeRangeStart" => time_range_start = value.parse::<u64>().unwrap_unchecked(),
                    "timeRangeEnd" => time_range_end = value.parse::<u64>().unwrap_unchecked(),
                    "numberDays" => number_days = value.parse::<u32>().unwrap_unchecked(),
                    "sortOrder" => sort_order = SortOrder::fast_from_str(value),
                    "page" => page = value.parse::<u32>().unwrap_unchecked(),
                    "pageSize" => page_size = value.parse::<u32>().unwrap_unchecked(),
                    "priceRangeWidth" => price_range_width = value.parse::<u32>().unwrap_unchecked(),
                    "minFreeKilometerWidth" => min_free_kilometer_width = value.parse::<u32>().unwrap_unchecked(),
                    "minNumberSeats" => min_number_seats = value.parse::<u32>().unwrap_unchecked().into(),
                    "minPrice" => min_price = value.parse::<u32>().unwrap_unchecked().into(),
                    "maxPrice" => max_price = value.parse::<u32>().unwrap_unchecked().into(),
                    "carType" => car_type = value.parse::<CarType>().unwrap_unchecked().into(),
                    "onlyVollkasko" => only_vollkasko = value.parse::<bool>().unwrap_unchecked().into(),
                    "minFreeKilometer" => min_free_kilometers = value.parse::<u32>().unwrap_unchecked().into(),
                    _ => {} // Skip unknown keys for simplicity
                }
        }
    });

    RequestOffer {
        region_id: region_id,
        time_range_start: time_range_start,
        time_range_end: time_range_end,
        number_days: number_days,
        sort_order: sort_order,
        page: page,
        page_size: page_size,
        price_range_width: price_range_width,
        min_free_kilometer_width: min_free_kilometer_width,
        min_number_seats: min_number_seats,
        min_price: min_price,
        max_price: max_price,
        car_type: car_type,
        only_vollkasko: only_vollkasko,
        min_free_kilometer: min_free_kilometers,
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::*;

    #[test]
    fn test_parse_request_offer() {
        env::set_var("RUST_BACKTRACE", "1");
        let res = parse_request_offer("minFreeKilometerWidth=50&numberDays=4&page=0&pageSize=100&priceRangeWidth=10&regionID=0&sortOrder=price-asc&timeRangeEnd=1716595200000&timeRangeStart=1716249600000");
        println!("{:#?}", res);
        assert_eq!(true, true);
    }
}


impl GetReponseBodyModel {
    pub unsafe fn to_json(&self) -> String {
        let mut json = String::with_capacity(1024); // Preallocate memory to reduce reallocations

        json.push('{');

        // Offers
        json.push_str("\"offers\":[");
        for (i, offer) in self.offers.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push('{');
            json.push_str("\"ID\":\"");
            json.push_str(&offer.ID);
            json.push_str("\",\"data\":\"");
            json.push_str(&offer.data);
            json.push_str("\"}");
        }
        json.push(']');

        // Price ranges
        json.push_str(",\"price_ranges\":[");
        for (i, range) in self.price_ranges.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "{{\"start\":{},\"end\":{},\"count\":{}}}",
                range.start, range.end, range.count
            ));
        }
        json.push(']');

        // Car type counts
        json.push_str(",\"car_type_counts\":{");
        json.push_str(&format!(
            "\"small\":{},\"sports\":{},\"luxury\":{},\"family\":{}",
            self.car_type_counts.small,
            self.car_type_counts.sports,
            self.car_type_counts.luxury,
            self.car_type_counts.family
        ));
        json.push('}');

        // Seat counts
        json.push_str(",\"seats_count\":[");
        for (i, seat) in self.seats_count.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "{{\"count\":{},\"number_seats\":{}}}",
                seat.count, seat.number_seats
            ));
        }
        json.push(']');

        // Free kilometer range
        json.push_str(",\"free_kilometer_range\":[");
        for (i, range) in self.free_kilometer_range.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "{{\"start\":{},\"end\":{},\"count\":{}}}",
                range.start, range.end, range.count
            ));
        }
        json.push(']');

        // Vollkasko count
        json.push_str(",\"vollkasko_count\":{");
        json.push_str(&format!(
            "\"true_count\":{},\"false_count\":{}",
            self.vollkasko_count.true_count, self.vollkasko_count.false_count
        ));
        json.push('}');

        json.push('}');

        json
    }
}


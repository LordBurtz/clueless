use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct RequestOffer {
    regionID: i8,
    timeRangeStart: i32,
    timeRangeEnd: i32,
    numberDays: i32,
    sortOrder: SortOrder,
    page: i32,
    pageSize: i32,
    priceRangeWidth: i32,
    minFreeKilometerWidth: i32,
    minNumberSeats: i32,
    minPrice: i32,
    maxPrice: i32,
    carType: CarType,
    onlyVollkasko: bool,
    minFreeKilometer: i32,
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


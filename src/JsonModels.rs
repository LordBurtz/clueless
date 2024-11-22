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

#[derive(Serialize, Deserialize, Debug)]
struct ResponseOffers {
    offers: Vec<Offer>,
    priceRange: Vec<PriceRange>,
    carTypeCounts: CarTypeCount,
    seatsCount: Vec<SeatCount>,
    freeKilometerRange: Vec<FreeKilometerRange>,
    vollkaskoCount: VollKaskoCount,
}

#[derive(Serialize, Deserialize, Debug)]
struct Offer {
    id: String,
    data: String
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
struct SeatCount {
    count: i32,
    numberSeats: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct FreeKilometerRange {
    start: i32,
    end: i32,
    count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct VollKaskoCount {
    trueCount: i32,
    falseCount: i32,
}


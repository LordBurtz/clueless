use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone)]
pub struct Offer {
    pub idx: u32,
    pub id: String,
    // TODO: optimize?
    pub data: String, // base64 encoded 256 Byte array
    pub most_specific_region_id: u32,
    pub start_date: u64,
    pub end_date: u64,
    pub number_seats: u32,
    pub price: u32,
    pub car_type: CarType,
    pub has_vollkasko: bool,
    pub free_kilometers: u32,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum CarType {
    Small = 0,
    Sports = 1,
    Luxury = 2,
    Family = 3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegionHierarchy {
    pub ancestor_id: u32,
    pub descendant_id: u32,
}

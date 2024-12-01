use crate::json_models::CarType;

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

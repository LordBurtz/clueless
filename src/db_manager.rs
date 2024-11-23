use std::sync::Mutex;
use crate::GenericError;
use crate::json_models::*;

pub struct DBManager {

}

impl DBManager {
    pub fn new_lock() -> Mutex<Self> {
        Mutex::new(DBManager {})
    }

    pub fn insert_offers(self, offers: PostRequestBodyModel) -> Result<(), GenericError> {
        return Ok(());
    }

    pub fn query_for(self, request_offer: RequestOffer) -> Option<GetReponseBodyModel> {
        return None;
    }
}
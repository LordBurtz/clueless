use crate::GenericError;
use crate::json_models::*;

struct DBManager {

}

impl DBManager {
    fn insert_offers(self, offers: PostRequestBodyModel) -> Result<(), GenericError> {
        return Ok(());
    }

    fn query_for(self, request_offer: RequestOffer) -> Option<GetReponseBodyModel> {
        return None;
    }
}
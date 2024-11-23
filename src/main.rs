// #![deny(warnings)]
mod db_manager;
mod db_models;
mod json_models;
mod region_hierarchy;
//mod tree_exp;

use json_models::*;

use crate::db_manager::DBManager;
use crate::region_hierarchy::populate_region_hierarchy;
use bytes::{Buf, Bytes};
use futures::{StreamExt, TryStreamExt};
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Incoming};
use hyper::server::conn::http1;
use hyper::service::{service_fn, Service};
use hyper::{body::Incoming as IncomingBody, header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use json_models::*;
use sonic_rs::{
    get_from_bytes_unchecked, to_array_iter_unchecked,
    JsonValueTrait,
};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static OFFER_CREATED: &[u8] = b"Offers were created";
static NOTFOUND: &[u8] = b"Not Found";
static OFFERS_CLEANED_UP: &[u8] = b"Offers were cleaned up";

async fn api_post_response(
    req: Request<Incoming>,
    manager: &DBManager,
) -> Result<Response<BoxBody>> {
    let body = req.collect().await?.to_bytes();

    let root_value = unsafe { get_from_bytes_unchecked(&body, &["offers"]).unwrap() };

    let iter = unsafe { to_array_iter_unchecked(root_value.as_raw_str()) };

    let mut insert = manager.client.insert("offers")?;

    println!("Inserting offers");

    for elem in iter {
        match elem {
            Ok(json_value) => {
                let id = json_value
                    .get("ID")
                    .ok_or("Missing or invalid field 'id'")?
                    .as_str()
                    .unwrap()
                    .to_string();
                let data = json_value
                    .get("data")
                    .ok_or("Missing or invalid field 'data'")?
                    .as_str()
                    .unwrap()
                    .to_string();
                let most_specific_region_id = json_value
                    .get("mostSpecificRegionID")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid field 'mostSpecificRegionID'")?
                    as u32;
                let start_date = json_value
                    .get("startDate")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid field 'start_date'")?;
                let end_date = json_value
                    .get("endDate")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid field 'endDate'")?;
                let number_seats = json_value
                    .get("numberSeats")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid field 'numberSeats'")?
                    as u32;
                let price = json_value
                    .get("price")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid field 'price'")? as u32;
                let car_type = json_value
                    .get("carType")
                    .ok_or("Missing or invalid field 'carType'")?
                    .as_str()
                    .unwrap()
                    .to_string();
                let has_vollkasko = json_value
                    .get("hasVollkasko")
                    .and_then(|v| v.as_bool())
                    .ok_or("Missing or invalid field 'hasVollkasko'")?;
                let free_kilometers = json_value
                    .get("freeKilometers")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid field 'freeKilometers'")?
                    as u32;

                let car_type = match car_type.as_str() {
                    "small" => db_models::CarType::Small,
                    "sports" => db_models::CarType::Sports,
                    "luxury" => db_models::CarType::Luxury,
                    "family" => db_models::CarType::Family,
                    _ => return Err("Invalid car type".into()),
                };
                let offer = db_models::Offer {
                    id,
                    data,
                    most_specific_region_id,
                    start_date,
                    end_date,
                    number_seats,
                    price,
                    car_type,
                    has_vollkasko,
                    free_kilometers,
                };

                // Write each offer to the database
                insert.write(&offer).await?;
            }
            Err(err) => {
                // Handle parsing errors
                eprintln!("Error parsing JSON array element: {:?}", err);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(full("Invalid JSON format"))?);
            }
        }
    }

    insert.end().await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full(OFFER_CREATED))?;
    Ok(response)
}

async fn handle_get_offers_request(
    req: Request<IncomingBody>,
    manager: &DBManager,
) -> Result<Response<BoxBody>> {
    // println!("GET request");
    // Aggregate the body...
    // println!("test");

    let query: RequestOffer = serde_urlencoded::from_str(req.uri().query().unwrap())?;

    // println!("{:?}", query);

    let (response, status_code) = match manager.query_for(query).await {
        Ok(res) => {
            // normally use res but now mock
            let json = serde_json::to_string(&res)?;

            (full(json), StatusCode::OK)
        }
        Err(err) => {
            println!("{:?}", err);
            (
                full(INTERNAL_SERVER_ERROR),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    };

    let response = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(response)?;
    Ok(response)
}

async fn delete_offer_request(manager: &DBManager) -> Result<Response<BoxBody>> {
    let (response, status_code) = match manager.cleanup().await {
        Ok(_) => (OFFERS_CLEANED_UP, StatusCode::OK),
        Err(_) => (INTERNAL_SERVER_ERROR, StatusCode::INTERNAL_SERVER_ERROR),
    };

    let response = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full(response))?;
    Ok(response)
}

async fn api_handler(
    req: Request<IncomingBody>,
    manager: Arc<DBManager>,
) -> Result<Response<BoxBody>> {
    // println!("{} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(full("clueless"))),
        (&Method::POST, "/api/offers") => api_post_response(req, &manager).await,
        (&Method::GET, "/api/offers") => handle_get_offers_request(req, &manager).await,
        (&Method::DELETE, "/api/offers") => delete_offer_request(&manager).await,
        _ => {
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(full(NOTFOUND))
                .unwrap())
        }
    }
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // pretty_env_logger::init();
    let db_client = clickhouse::Client::default()
        .with_url("http://localhost:8123")
        .with_user("default")
        .with_database("check24");

    let db_manager = Arc::new(DBManager::new(db_client));
    db_manager.init().await?;
    populate_region_hierarchy(&db_manager).await?;

    let addr: SocketAddr = "0.0.0.0:80".parse().unwrap();

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let db_manager = db_manager.clone();

        tokio::task::spawn(async move {
            let service = service_fn(|req| {
                let db_manager = db_manager.clone();
                api_handler(req, db_manager)
            });

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

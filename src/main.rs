// #![deny(warnings)]
mod json_models;
mod db_manager;

use json_models::*;

use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, header, Method, Request, Response, StatusCode};
use rusqlite::Connection;
use tokio::net::TcpListener;
use crate::db_manager::DBManager;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static OFFER_CREATED: &[u8] = b"Offers were created";
static NOTFOUND: &[u8] = b"Not Found";
static OFFERS_CLEANED_UP: &[u8] = b"Offers were cleaned up";

async fn api_post_response(req: Request<IncomingBody>, ref_db: Arc<Mutex<DBManager>>) -> Result<Response<BoxBody>> {
    // Aggregate the body...
    let whole_body = req.collect().await?.aggregate();

    // parse into model
    let request_body: PostRequestBodyModel = serde_json::from_reader(whole_body.reader())?;

    // try
    let mut manager = ref_db.lock().unwrap();

    let (response, status_code) = match manager.insert_offers(request_body) {
        Ok(_) => {
            (OFFER_CREATED, StatusCode::OK)
        },
        Err(err) => {
            println!("{:?}", err);
            (INTERNAL_SERVER_ERROR, StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    let response = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full(response))?;
    Ok(response)
}

async fn handle_get_offers_request(req: Request<IncomingBody>, ref_db: Arc<Mutex<DBManager>>) -> Result<Response<BoxBody>> {
    // Aggregate the body...
    let whole_body = req.collect().await?.aggregate();

    // parse into model
    let request_body: RequestOffer = serde_json::from_reader(whole_body.reader())?;
    // TODO: mock
    let mocked_query_result: GetReponseBodyModel = serde_json::from_str(SAMPLE_GET_RESPONSE)?;
        // serde_urlencoded::from_str(SAMPLE_GET_RESPONSE)?;

    let mut manager = ref_db.lock().unwrap();

    let (response, status_code) = match manager.query_mock(request_body) {
        Ok(res) => {
            // normally use res but now mock
            let json = serde_json::to_string(&mocked_query_result)?;

            (full(json), StatusCode::OK)
        },
        Err(_) => {
            (full(INTERNAL_SERVER_ERROR), StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    let response = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(response)?;
    Ok(response)
}

async fn delete_offer_request(ref_db: Arc<Mutex<DBManager>>) -> Result<Response<BoxBody>> {
    let mut manager = ref_db.lock().unwrap();

    let (response, status_code) = match manager.cleanup() {
        Ok(_) => {
            (OFFERS_CLEANED_UP, StatusCode::OK)
        },
        Err(_) => {
            (INTERNAL_SERVER_ERROR, StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    let response = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full(response))?;
    Ok(response)
}

async fn api_handler(req: Request<IncomingBody>, ref_db: Arc<Mutex<DBManager>>) -> Result<Response<BoxBody>> {

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(full("clueless"))),
        (&Method::POST, "/api/offers") => api_post_response(req, ref_db).await,
        (&Method::GET, "/api/offers") => handle_get_offers_request(req, ref_db).await,
        (&Method::DELETE, "/api/offers") => delete_offer_request(ref_db).await,
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

#[tokio::main]
async fn main() -> Result<()> {
    // pretty_env_logger::init();
    let conn = db_manager::open_connection()?;
    let db = Arc::new(DBManager::new_lock(conn));

    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let db_clone = db.clone();

        tokio::task::spawn(async move {
            let service = service_fn(|req| api_handler(req,  db_clone.clone()));

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

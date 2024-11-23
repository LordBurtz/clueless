// #![deny(warnings)]
mod db_manager;
mod json_models;
mod db_models;
mod region_hierarchy;

use json_models::*;

use crate::db_manager::DBManager;
use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::{service_fn, Service};
use hyper::{body::Incoming as IncomingBody, header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use crate::region_hierarchy::populate_region_hierarchy;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static OFFER_CREATED: &[u8] = b"Offers were created";
static NOTFOUND: &[u8] = b"Not Found";
static OFFERS_CLEANED_UP: &[u8] = b"Offers were cleaned up";

async fn api_post_response(
    req: Request<IncomingBody>,
    manager: &DBManager,
) -> Result<Response<BoxBody>> {
    // Aggregate the body...
    let whole_body = req.collect().await?.aggregate();

    // parse into model
    let request_body: PostRequestBodyModel = serde_json::from_reader(whole_body.reader())?;

    // try

    let (response, status_code) = match manager.insert_offers(request_body).await {
        Ok(_) => (OFFER_CREATED, StatusCode::OK),
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

async fn handle_get_offers_request(
    req: Request<IncomingBody>,
    manager: &DBManager,
) -> Result<Response<BoxBody>> {
    println!("GET request");
    // Aggregate the body...
    println!("test");

    let query: RequestOffer = serde_urlencoded::from_str(req.uri().query().unwrap())?;

    println!("{:?}", query);

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
    println!("{} {}", req.method(), req.uri().path());
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

#[tokio::main]
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

#![forbid(unsafe_code)]
#![cfg_attr(feature = "strict", deny(warnings))]
//! This is a library containing functionality for the journali
//! backend.
//!
//! This library exists for documentation purposes.

use actix_web::{get, web, HttpResponse, Responder};

/// The soul purpose of this module is to be
/// able to reference the current commit hash.
pub(crate) mod app_version {
    // We need to do this, orelse the environment
    // file will *NOT* be loaded during compilation.
    // This can't be used in an expression, due to
    // the way procedural macro's work.
    load_dotenv::try_load_dotenv!();
    pub const VERSION: &'static str = env!("RUST_APP_VERSION");
}

#[get("/hello/{name}")]
pub async fn hello(data: web::Path<String>) -> impl Responder {
    HttpResponse::Ok().body(format!("Hello sailor {}!", data.into_inner()))
}

#[get("/version")]
pub async fn version() -> impl Responder {
    HttpResponse::Ok().body(app_version::VERSION)
}

#[actix_rt::test]
async fn test_hello() {
    use actix_web::{body::Body, http::StatusCode, test, web::Bytes, App};

    let mut app = test::init_service(App::new().service(hello)).await;

    let req = test::TestRequest::with_uri("/hello/tester").to_request();

    // Call application
    let mut resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert_eq!(
        resp.take_body().as_ref(),
        Some(&Body::Bytes(Bytes::from("Hello sailor tester!")))
    );
}

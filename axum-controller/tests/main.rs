#![allow(unused)]
#![allow(clippy::extra_unused_type_parameters)]

use std::net::TcpListener;

use axum::{
    extract::{Path, State},
    routing::get,
    Form, Json,
};
use axum_test::TestServer;
use axum_controller::TypedRouter;
use axum_controller_macros::route;

/// This is a handler that is documented!
#[route(GET "/hello/:id?user_id&name")]
async fn generic_handler_with_complex_options<T: 'static>(
    mut id: u32,
    user_id: String,
    name: String,
    State(state): State<String>,
    hello: State<String>,
    Json(mut json): Json<u32>,
) -> String {
    format!("Hello, {id} - {user_id} - {name}!")
}

#[route(POST "/one")]
async fn one(state: State<String>) -> String {
    String::from("Hello!")
}

#[route(POST "/two")]
async fn two() -> String {
    String::from("Hello!")
}

#[route(GET "/three/:id")]
async fn three(id: u32) -> String {
    format!("Hello {id}!")
}

#[route(GET "/four?id")]
async fn four(id: u32) -> String {
    format!("Hello {id:?}!")
    // String::from("Hello 123!")
}

// Tests that hyphens are allowed in route names
#[route(GET "/foo-bar")]
async fn foo_bar() {}

#[tokio::test]
async fn test_normal() {
    let router: axum::Router = axum::Router::new()
        .typed_route(generic_handler_with_complex_options::<u32>)
        .typed_route(one)
        .with_state("state".to_string())
        .typed_route(two)
        .typed_route(three)
        .typed_route(four);

    let server = TestServer::new(router).unwrap();

    let response = server.post("/one").await;
    response.assert_status_ok();
    response.assert_text("Hello!");

    let response = server.post("/two").await;
    response.assert_status_ok();
    response.assert_text("Hello!");

    let response = server.get("/three/123").await;
    response.assert_status_ok();
    response.assert_text("Hello 123!");

    let response = server.get("/four").add_query_param("id", 123).await;
    response.assert_status_ok();
    response.assert_text("Hello 123!");

    let response = server
        .get("/hello/123")
        .add_query_param("user_id", 321.to_string())
        .add_query_param("name", "John".to_string())
        .json(&100)
        .await;
    response.assert_status_ok();
    response.assert_text("Hello, 123 - 321 - John!");

    let (path, method_router) = generic_handler_with_complex_options::<u32>();
    assert_eq!(path, "/hello/{id}");
}

#[route(GET "/*")]
async fn wildcard() {}

#[route(GET "/*capture")]
async fn wildcard_capture(capture: String) -> Json<String> {
    Json(capture)
}

#[route(GET "/")]
async fn root() {}

#[tokio::test]
async fn test_wildcard() {
    let router: axum::Router = axum::Router::new().typed_route(wildcard_capture);

    let server = TestServer::new(router).unwrap();

    let response = server.get("/foo/bar").await;
    response.assert_status_ok();
    assert_eq!(response.json::<String>(), "foo/bar");
}

#![allow(unused)]
use axum::extract::{Json, State};
use axum_controller::{controller, route, TypedRouter};

#[route(GET "/item/:id?amount&offset")]
async fn item_handler(
    id: u32,
    amount: Option<u32>,
    offset: Option<u32>,
    State(state): State<String>,
    Json(json): Json<u32>,
) -> String {
    todo!("handle request")
}

struct TestController;

#[controller(path = "/asd")]
impl TestController {}

fn main() {
    let router: axum::Router = axum::Router::new()
        .typed_route(item_handler)
        .with_state("state".to_string());
}

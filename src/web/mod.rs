mod error;
mod handlers;

use std::sync::Arc;

use axum::{Router, routing::get};

use crate::runtime::ContainerRuntime;

/// Builds the dashboard's routing table.
///
/// Takes the runtime as a trait object so `main` decides the implementation and
/// tests can pass a fake. Gains a `store` alongside it once the poller lands.
pub fn router(runtime: Arc<dyn ContainerRuntime>) -> Router {
    Router::new()
        .route("/", get(handlers::index))
        .with_state(runtime)
}

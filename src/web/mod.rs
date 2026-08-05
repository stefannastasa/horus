mod error;
mod handlers;

use std::sync::Arc;

use axum::{Router, routing::get};

use crate::{runtime::ContainerRuntime, store::Store};

/// Shared by every handler. Both fields are `Arc`, so cloning is a refcount
/// bump — which is what axum does per request.
#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<dyn ContainerRuntime>,
    pub store: Arc<dyn Store>,
}

/// Builds the dashboard's routing table.
///
/// Takes trait objects so `main` decides the implementations and tests can pass
/// fakes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::index))
        .with_state(state)
}

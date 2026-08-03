use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::Html};

use crate::{domain::Container, runtime::ContainerRuntime, web::error::AppError};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    containers: Vec<Container>,
}

pub async fn index(
    State(runtime): State<Arc<dyn ContainerRuntime>>,
) -> Result<Html<String>, AppError> {
    let containers = runtime.list().await?;
    Ok(Html(IndexTemplate { containers }.render()?))
}

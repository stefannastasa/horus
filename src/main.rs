use anyhow::Result;
use askama::Template;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use bollard::{Docker, query_parameters::ListContainersOptionsBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let docker = Docker::connect_with_defaults()?;
    let app = Router::new().route("/", get(index)).with_state(docker);

    let listening_addr = "0.0.0.0:5067";
    let listener = tokio::net::TcpListener::bind(listening_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    tracing::info!("server stopped");
    Ok(())
}

/// One row in the dashboard table.
struct ContainerView {
    name: String,
    image: String,
    state: String,
    status: String,
    running: bool,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    containers: Vec<ContainerView>,
}

async fn index(State(docker): State<Docker>) -> Result<Html<String>, AppError> {
    let options = ListContainersOptionsBuilder::default().all(true).build();

    let containers = docker
        .list_containers(Some(options))
        .await?
        .into_iter()
        .map(|c| {
            let state = c
                .state
                .map(|s| format!("{s:?}").to_lowercase())
                .unwrap_or_else(|| "unknown".to_string());

            ContainerView {
                // names come back prefixed with a forward slash
                name: c
                    .names
                    .and_then(|n| n.into_iter().next())
                    .map(|n| n.trim_start_matches('/').to_string())
                    .unwrap_or_else(|| "<unnamed>".to_string()),
                image: c.image.unwrap_or_default(),
                running: state == "running",
                state,
                status: c.status.unwrap_or_default(),
            }
        })
        .collect();

    Ok(Html(IndexTemplate { containers }.render()?))
}

/// Lets handlers use `?` on anything that converts into an `anyhow::Error`.
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{:#}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:#}", self.0)).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

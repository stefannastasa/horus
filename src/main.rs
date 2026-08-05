mod config;
mod domain;
mod poller;
mod runtime;
mod store;
mod web;

use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::Parser;

use crate::{
    config::Config,
    runtime::{ContainerRuntime, docker::DockerRuntime},
    store::{Store, memory::InMemoryStore},
    web::AppState,
};

#[derive(Parser)]
#[command(name = "horus", about = "Container dashboard for my homelab")]
struct Args {
    /// Path to the TOML config file. Missing is fine — defaults apply.
    #[arg(short, long, default_value = "horus.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // bollard logs every decoded API response at DEBUG, which buries our own
    // output. RUST_LOG overrides this wholesale when set.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("horus=debug,warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;
    tracing::info!(?config, "loaded config");

    let runtime: Arc<dyn ContainerRuntime> =
        Arc::new(DockerRuntime::connect(config.runtime.socket.as_deref())?);
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new(config.history_len));

    // Clone before handing them over: `tokio::spawn` needs a 'static future, so
    // the task has to own its handles.
    poller::spawn(runtime.clone(), store.clone(), config.poll_interval());
    tracing::info!("polling every {:?}", config.poll_interval());

    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, web::router(AppState { runtime, store })).await?;
    tracing::info!("server stopped");
    Ok(())
}

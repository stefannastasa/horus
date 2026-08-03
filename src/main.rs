mod config;
mod domain;
mod runtime;
mod web;

use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::Parser;

use crate::{config::Config, runtime::ContainerRuntime, runtime::docker::DockerRuntime};

#[derive(Parser)]
#[command(name = "horus", about = "Container dashboard for my homelab")]
struct Args {
    /// Path to the TOML config file. Missing is fine — defaults apply.
    #[arg(short, long, default_value = "horus.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;
    tracing::info!(?config, "loaded config");

    let runtime: Arc<dyn ContainerRuntime> =
        Arc::new(DockerRuntime::connect(config.runtime.socket.as_deref())?);

    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, web::router(runtime)).await?;
    tracing::info!("server stopped");
    Ok(())
}

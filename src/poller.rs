use std::{sync::Arc, time::Duration};

use time::OffsetDateTime;
use tokio::time::MissedTickBehavior;

use crate::{
    domain::Sample,
    runtime::{ContainerRuntime, RuntimeError},
    store::{Store, StoreError},
};

#[derive(Debug, thiserror::Error)]
pub enum PollError {
    #[error("runtime: {0}")]
    Runtime(#[from] RuntimeError),
    #[error("store: {0}")]
    Store(#[from] StoreError),
}

pub async fn poll_once(
    runtime: &dyn ContainerRuntime,
    store: &dyn Store,
) -> Result<usize, PollError> {
    let now = OffsetDateTime::now_utc();
    let samples: Vec<Sample> = runtime
        .list()
        .await?
        .iter()
        .map(|c| Sample::new(now, c))
        .collect();
    store.record(&samples).await.map_err(PollError::Store)?;
    Ok(samples.len())
}

/// Not `async`: the task starts as soon as this is called, rather than when a
/// returned future is awaited.
pub fn spawn(
    runtime: Arc<dyn ContainerRuntime>,
    store: Arc<dyn Store>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match poll_once(runtime.as_ref(), store.as_ref()).await {
                Ok(n) => tracing::debug!("recorded {n} samples"),
                Err(e) => tracing::warn!("poll failed: {e}"),
            }
        }
    })
}

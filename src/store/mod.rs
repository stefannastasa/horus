use async_trait::async_trait;

use crate::domain::Sample;

pub mod memory;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store lock poisoned: {0}")]
    Lock(String),
}

#[async_trait]
pub trait Store: Send + Sync {
    async fn record(&self, samples: &[Sample]) -> Result<(), StoreError>;
    async fn recent(&self, id: &str, limit: usize) -> Result<Vec<Sample>, StoreError>;
    async fn registered_containers(&self) -> Result<Vec<String>, StoreError>;
}

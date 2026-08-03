#[async_trait]
pub trait Store: Send + Sync {
    async fn record(&self, samples: &[Sample]) -> Result<()>;
    async fn recent(&self, id: &str, limit: usize) -> Result<Vec<Sample>>;
}

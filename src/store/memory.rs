use std::{
    collections::{HashMap, VecDeque},
    sync::RwLock,
};

use async_trait::async_trait;

use crate::{
    domain::Sample,
    store::{Store, StoreError},
};

pub struct InMemoryStore {
    max_samples: usize,
    data: RwLock<HashMap<String, VecDeque<Sample>>>,
}

impl InMemoryStore {
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples,
            data: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Store for InMemoryStore {
    async fn record(&self, samples: &[Sample]) -> Result<(), StoreError> {
        let mut store = self
            .data
            .write()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        for sample in samples {
            let container_store = store.entry(sample.container_id.clone()).or_default();
            if container_store.len() >= self.max_samples {
                container_store.pop_front();
            }
            container_store.push_back(sample.clone());
        }
        Ok(())
    }
    async fn recent(&self, id: &str, limit: usize) -> Result<Vec<Sample>, StoreError> {
        let store = self
            .data
            .read()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        let limited_samples = store
            .get(id)
            .map(|q| q.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default();
        Ok(limited_samples)
    }

    async fn registered_containers(&self) -> Result<Vec<String>, StoreError> {
        let store = self
            .data
            .read()
            .map_err(|e| StoreError::Lock(e.to_string()))?;

        Ok(store.keys().cloned().collect())
    }
}

#[cfg(test)]
mod test {
    use std::range::Range;

    use time::OffsetDateTime;

    use crate::{
        domain::{Sample, State},
        store::Store,
    };

    use super::InMemoryStore;

    fn sample_at(id: &String, secs: i64) -> Sample {
        Sample {
            at: OffsetDateTime::from_unix_timestamp(secs).unwrap(),
            container_id: id.to_string(),
            state: State::Running,
        }
    }

    #[tokio::test]
    async fn basics() {
        let store = InMemoryStore::new(5);
        assert!(store.recent("nope", 5).await.unwrap().is_empty());
        let mut samples = Vec::new();
        for _ in Range::from(0..20) {
            samples.push(sample_at(
                &String::from("nope"),
                OffsetDateTime::now_utc().unix_timestamp(),
            ));
        }
        store.record(&samples).await.unwrap();
        let samples = store.recent("nope", 10).await.unwrap();
        assert!(samples.len() == 5);
        assert!(samples.iter().map(|s| s.at).is_sorted()); // see if samples are returned in the order they were placed in (no sorting ds)
    }
}

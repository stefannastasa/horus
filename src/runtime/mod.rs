//! The seam between horus and whatever is actually running containers.

pub mod docker;

use async_trait::async_trait;

use crate::domain::Container;

/// Anything that can tell horus what containers exist.
///
/// `Send + Sync` because implementations are shared across tokio tasks behind
/// an `Arc`, and `async_trait` boxes the returned futures so the trait stays
/// dyn-compatible — see the note in `main` about `Arc<dyn ContainerRuntime>`.
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    async fn list(&self) -> Result<Vec<Container>, RuntimeError>;
}

/// Deliberately does not name bollard: callers shouldn't have to know which
/// runtime adapter produced the failure.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("container runtime unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("unexpected response from container runtime: {0}")]
    Malformed(String),
}

#[cfg(test)]
pub mod fake {
    use super::*;

    /// Lets the poller and handlers be tested without a container runtime.
    pub struct FakeRuntime {
        pub containers: Vec<Container>,
        pub fail: bool,
    }

    impl FakeRuntime {
        pub fn with(containers: Vec<Container>) -> Self {
            Self {
                containers,
                fail: false,
            }
        }

        pub fn failing() -> Self {
            Self {
                containers: Vec::new(),
                fail: true,
            }
        }
    }

    #[async_trait]
    impl ContainerRuntime for FakeRuntime {
        async fn list(&self) -> Result<Vec<Container>, RuntimeError> {
            if self.fail {
                return Err(RuntimeError::Malformed("fake failure".into()));
            }
            Ok(self.containers.clone())
        }
    }
}

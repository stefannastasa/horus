//! Adapter over the Docker-compatible API, which is what rootless Podman
//! exposes on its socket. All bollard types stop here.

use async_trait::async_trait;
use bollard::{Docker, models::ContainerSummary, query_parameters::ListContainersOptionsBuilder};

use crate::{
    domain::{Container, State},
    runtime::{ContainerRuntime, RuntimeError},
};

pub struct DockerRuntime {
    docker: Docker,
}

impl DockerRuntime {
    /// `socket` of `None` defers to bollard's defaults, which honour `DOCKER_HOST`.
    pub fn connect(socket: Option<&str>) -> Result<Self, RuntimeError> {
        let docker = match socket {
            Some(path) => Docker::connect_with_socket(path, 120, bollard::API_DEFAULT_VERSION),
            None => Docker::connect_with_defaults(),
        }
        .map_err(|e| RuntimeError::Unavailable(Box::new(e)))?;

        Ok(Self { docker })
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    async fn list(&self) -> Result<Vec<Container>, RuntimeError> {
        let options = ListContainersOptionsBuilder::default().all(true).build();

        let summaries = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| RuntimeError::Unavailable(Box::new(e)))?;

        summaries.into_iter().map(convert).collect()
    }
}

/// Every field on `ContainerSummary` is optional, so this is where the option
/// handling lives — once, rather than scattered through the codebase.
fn convert(summary: ContainerSummary) -> Result<Container, RuntimeError> {
    let id = summary
        .id
        .ok_or_else(|| RuntimeError::Malformed("container has no id".into()))?;

    // Names are prefixed with a forward slash for historical reasons, and a
    // container may have several when legacy links are in play.
    let name = summary
        .names
        .and_then(|names| names.into_iter().next())
        .map(|name| name.trim_start_matches('/').to_string())
        .unwrap_or_else(|| id.chars().take(12).collect());

    let state = summary
        .state
        .map(|s| State::from(format!("{s:?}").as_str()))
        .unwrap_or_else(|| State::Other("unknown".into()));

    Ok(Container {
        id,
        name,
        image: summary.image.unwrap_or_default(),
        state,
        status: summary.status.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::ContainerSummaryStateEnum;

    fn summary() -> ContainerSummary {
        ContainerSummary {
            id: Some("abc123def456789".into()),
            names: Some(vec!["/memos".into()]),
            image: Some("ghcr.io/usememos/memos:latest".into()),
            state: Some(ContainerSummaryStateEnum::RUNNING),
            status: Some("Up 3 hours".into()),
            ..Default::default()
        }
    }

    #[test]
    fn converts_a_typical_summary() {
        let c = convert(summary()).unwrap();
        assert_eq!(c.name, "memos");
        assert_eq!(c.state, State::Running);
        assert_eq!(c.status, "Up 3 hours");
    }

    #[test]
    fn falls_back_to_a_short_id_when_unnamed() {
        let c = convert(ContainerSummary {
            names: None,
            ..summary()
        })
        .unwrap();
        assert_eq!(c.name, "abc123def456");
    }

    #[test]
    fn rejects_a_summary_without_an_id() {
        let err = convert(ContainerSummary {
            id: None,
            ..summary()
        });
        assert!(matches!(err, Err(RuntimeError::Malformed(_))));
    }
}

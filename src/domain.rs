//! The types horus reasons about, independent of any container runtime.
//!
//! Nothing in here should mention bollard, axum or a database. Adapters convert
//! into these types at their boundary, so the rest of the codebase never deals
//! with a particular runtime's idea of what a container is.

use std::fmt;

use time::OffsetDateTime;

/// A container as horus cares about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: State,
    /// Human-readable detail from the runtime, e.g. "Up 3 hours".
    pub status: String,
}

/// Lifecycle state, normalised across runtimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Created,
    Running,
    Paused,
    Restarting,
    Exited,
    Dead,
    /// Anything a runtime reports that we don't model explicitly.
    Other(String),
}

impl State {
    pub fn is_running(&self) -> bool {
        matches!(self, State::Running)
    }

    pub fn as_str(&self) -> &str {
        match self {
            State::Created => "created",
            State::Running => "running",
            State::Paused => "paused",
            State::Restarting => "restarting",
            State::Exited => "exited",
            State::Dead => "dead",
            State::Other(s) => s,
        }
    }
}

impl From<&str> for State {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "created" => State::Created,
            "running" => State::Running,
            "paused" => State::Paused,
            "restarting" => State::Restarting,
            "exited" => State::Exited,
            "dead" => State::Dead,
            other => State::Other(other.to_string()),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One observation of one container at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    pub at: OffsetDateTime,
    pub container_id: String,
    pub state: State,
}

impl Sample {
    pub fn new(at: OffsetDateTime, container: &Container) -> Self {
        Self {
            at,
            container_id: container.id.clone(),
            state: container.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_states_parse_case_insensitively() {
        assert_eq!(State::from("Running"), State::Running);
        assert_eq!(State::from("EXITED"), State::Exited);
    }

    #[test]
    fn unknown_states_are_preserved_verbatim() {
        assert_eq!(State::from("stopping"), State::Other("stopping".into()));
        assert_eq!(State::from("stopping").to_string(), "stopping");
    }

    #[test]
    fn only_running_counts_as_running() {
        assert!(State::Running.is_running());
        assert!(!State::Restarting.is_running());
        assert!(!State::Other("running-ish".into()).is_running());
    }
}

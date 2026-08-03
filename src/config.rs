use std::{collections::BTreeMap, net::SocketAddr, path::Path, time::Duration};

use anyhow::{Context, Result};
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

/// Everything horus needs to start, assembled from defaults, a TOML file and the
/// environment — in that order of increasing precedence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Address the dashboard binds to. Must be 0.0.0.0 to be reachable off-host.
    pub listen: SocketAddr,

    /// How often the poller samples the container runtime.
    poll_interval_secs: u64,

    /// Samples kept per container by the in-memory store.
    pub history_len: usize,

    pub runtime: RuntimeConfig,

    /// Services to health-check, keyed by the name shown in the dashboard.
    /// Empty until HTTP checks land; the shape is here so the file doesn't
    /// need restructuring later.
    #[serde(default)]
    pub services: BTreeMap<String, ServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    /// Unix socket of the container runtime. `None` falls back to bollard's
    /// defaults, which honour `DOCKER_HOST`.
    #[serde(default)]
    pub socket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    /// URL to GET when checking whether the app behind the container responds.
    pub url: String,

    #[serde(default = "default_timeout_secs")]
    timeout_secs: u64,
}

impl Config {
    /// Load defaults, overlay `path` if it exists, then overlay `HORUS_*` env vars.
    pub fn load(path: &Path) -> Result<Self> {
        let config: Config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(path))
            .merge(Env::prefixed("HORUS_").split("__"))
            .extract()
            .with_context(|| format!("loading config from {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.poll_interval_secs > 0,
            "poll_interval_secs must be > 0"
        );
        anyhow::ensure!(self.history_len > 0, "history_len must be > 0");
        Ok(())
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.poll_interval_secs)
    }
}

impl ServiceConfig {
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([0, 0, 0, 0], 5067)),
            poll_interval_secs: 30,
            // 30s x 2880 = 24h of history
            history_len: 2880,
            runtime: RuntimeConfig { socket: None },
            services: BTreeMap::new(),
        }
    }
}

fn default_timeout_secs() -> u64 {
    5
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Figment::jail` gives each test an isolated cwd and environment.
    #[test]
    fn defaults_apply_with_no_file() {
        figment::Jail::expect_with(|_jail| {
            let config = Config::load(Path::new("horus.toml")).unwrap();
            assert_eq!(config.listen.port(), 5067);
            assert_eq!(config.poll_interval(), Duration::from_secs(30));
            assert!(config.services.is_empty());
            Ok(())
        });
    }

    #[test]
    fn file_overrides_defaults_and_env_overrides_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "horus.toml",
                r#"
                listen = "127.0.0.1:8080"
                poll_interval_secs = 10

                [runtime]
                socket = "/run/user/1000/podman/podman.sock"

                [services.memos]
                url = "http://localhost:5230/healthz"
                "#,
            )?;

            let config = Config::load(Path::new("horus.toml")).unwrap();
            assert_eq!(config.listen.port(), 8080);
            assert_eq!(config.poll_interval(), Duration::from_secs(10));
            assert_eq!(config.services["memos"].timeout(), Duration::from_secs(5));

            jail.set_env("HORUS_POLL_INTERVAL_SECS", "60");
            let config = Config::load(Path::new("horus.toml")).unwrap();
            assert_eq!(config.poll_interval(), Duration::from_secs(60));

            Ok(())
        });
    }

    #[test]
    fn rejects_zero_poll_interval() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("horus.toml", "poll_interval_secs = 0")?;
            assert!(Config::load(Path::new("horus.toml")).is_err());
            Ok(())
        });
    }
}

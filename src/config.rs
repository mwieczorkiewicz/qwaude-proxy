//! Runtime configuration, resolved from environment variables with documented defaults.

use std::env;
use std::time::Duration;

/// `LISTEN_ADDR` — address:port the proxy's HTTP server binds to.
pub const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:8080";
/// `VLLM_BASE_URL` — base URL of the upstream vLLM OpenAI-compatible server.
pub const DEFAULT_VLLM_BASE_URL: &str = "http://127.0.0.1:8000";
/// `NOTICE_PREFIX` — prepended to the content of every coerced system message.
pub const DEFAULT_NOTICE_PREFIX: &str = "[System Notification] ";
/// `LOG_LEVEL` — default `tracing_subscriber::EnvFilter` directive.
pub const DEFAULT_LOG_LEVEL: &str = "info";
/// `MAX_REQUEST_BODY_SIZE` — bytes; default 10 MiB.
pub const DEFAULT_MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024;
/// `UPSTREAM_CONNECT_TIMEOUT_SECS` — seconds to wait for the upstream TCP/TLS
/// connect to vLLM before failing with a 502.
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 5;
/// `UPSTREAM_TOTAL_TIMEOUT_SECS` — seconds bounding a non-streaming upstream
/// request end-to-end (a streaming response is bounded only until its first
/// byte arrives, not for the duration of the stream) before failing with a 504.
pub const DEFAULT_TOTAL_TIMEOUT_SECS: u64 = 30;
/// `VERBOSE_PAYLOAD_LOGGING` — off by default. When `true`, the handler
/// additionally logs request/response payload content at `debug`, for local
/// debugging only; never enable this against real traffic.
pub const DEFAULT_VERBOSE_PAYLOAD_LOGGING: bool = false;

/// Fully resolved runtime configuration for one proxy process.
#[derive(Debug, Clone, PartialEq)]
pub struct ProxyConfig {
    pub listen_addr: String,
    pub vllm_base_url: String,
    pub notice_prefix: String,
    pub log_level: String,
    pub max_request_body_size: usize,
    pub connect_timeout: Duration,
    pub total_timeout: Duration,
    pub verbose_payload_logging: bool,
}

/// A single invalid environment variable value.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid value for {var}: {value:?} ({reason})")]
pub struct ConfigError {
    var: &'static str,
    value: String,
    reason: &'static str,
}

/// Abstraction over "a source of named string values", so config resolution
/// is unit-testable without mutating real process environment variables —
/// `std::env::set_var` is process-global and racy under `cargo test`'s
/// default parallel execution, so tests inject a fake source instead.
trait EnvSource {
    fn get(&self, key: &str) -> Option<String>;
}

struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn get(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

impl ProxyConfig {
    /// Resolve configuration from the real process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::resolve(&ProcessEnv)
    }

    fn resolve(source: &impl EnvSource) -> Result<Self, ConfigError> {
        let listen_addr = source
            .get("LISTEN_ADDR")
            .unwrap_or_else(|| DEFAULT_LISTEN_ADDR.to_string());
        let vllm_base_url = source
            .get("VLLM_BASE_URL")
            .unwrap_or_else(|| DEFAULT_VLLM_BASE_URL.to_string());
        let notice_prefix = source
            .get("NOTICE_PREFIX")
            .unwrap_or_else(|| DEFAULT_NOTICE_PREFIX.to_string());
        let log_level = source
            .get("LOG_LEVEL")
            .unwrap_or_else(|| DEFAULT_LOG_LEVEL.to_string());
        let max_request_body_size = parse_or_default(
            source.get("MAX_REQUEST_BODY_SIZE"),
            DEFAULT_MAX_REQUEST_BODY_SIZE,
            "MAX_REQUEST_BODY_SIZE",
            "must be a positive integer number of bytes",
        )?;
        let connect_timeout_secs = parse_or_default(
            source.get("UPSTREAM_CONNECT_TIMEOUT_SECS"),
            DEFAULT_CONNECT_TIMEOUT_SECS,
            "UPSTREAM_CONNECT_TIMEOUT_SECS",
            "must be a positive integer number of seconds",
        )?;
        let total_timeout_secs = parse_or_default(
            source.get("UPSTREAM_TOTAL_TIMEOUT_SECS"),
            DEFAULT_TOTAL_TIMEOUT_SECS,
            "UPSTREAM_TOTAL_TIMEOUT_SECS",
            "must be a positive integer number of seconds",
        )?;
        let verbose_payload_logging = parse_or_default(
            source.get("VERBOSE_PAYLOAD_LOGGING"),
            DEFAULT_VERBOSE_PAYLOAD_LOGGING,
            "VERBOSE_PAYLOAD_LOGGING",
            "must be `true` or `false`",
        )?;

        Ok(Self {
            listen_addr,
            vllm_base_url,
            notice_prefix,
            log_level,
            max_request_body_size,
            connect_timeout: Duration::from_secs(connect_timeout_secs),
            total_timeout: Duration::from_secs(total_timeout_secs),
            verbose_payload_logging,
        })
    }
}

fn parse_or_default<T>(
    raw: Option<String>,
    default: T,
    var: &'static str,
    reason: &'static str,
) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    match raw {
        None => Ok(default),
        Some(value) => value
            .parse::<T>()
            .map_err(|_| ConfigError { var, value, reason }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeEnv(HashMap<&'static str, &'static str>);

    impl EnvSource for FakeEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).map(|v| v.to_string())
        }
    }

    fn fake_env(pairs: &[(&'static str, &'static str)]) -> FakeEnv {
        FakeEnv(pairs.iter().copied().collect())
    }

    #[test]
    fn all_defaults_apply_when_no_env_vars_are_set() {
        let config = ProxyConfig::resolve(&fake_env(&[])).expect("defaults are valid");
        assert_eq!(config.listen_addr, DEFAULT_LISTEN_ADDR);
        assert_eq!(config.vllm_base_url, DEFAULT_VLLM_BASE_URL);
        assert_eq!(config.notice_prefix, DEFAULT_NOTICE_PREFIX);
        assert_eq!(config.log_level, DEFAULT_LOG_LEVEL);
        assert_eq!(config.max_request_body_size, DEFAULT_MAX_REQUEST_BODY_SIZE);
        assert_eq!(
            config.connect_timeout,
            Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS)
        );
        assert_eq!(
            config.total_timeout,
            Duration::from_secs(DEFAULT_TOTAL_TIMEOUT_SECS)
        );
        assert_eq!(
            config.verbose_payload_logging,
            DEFAULT_VERBOSE_PAYLOAD_LOGGING
        );
    }

    #[test]
    fn verbose_payload_logging_is_overridable() {
        let config = ProxyConfig::resolve(&fake_env(&[("VERBOSE_PAYLOAD_LOGGING", "true")]))
            .expect("valid override");
        assert!(config.verbose_payload_logging);
    }

    #[test]
    fn listen_addr_is_overridable() {
        let config = ProxyConfig::resolve(&fake_env(&[("LISTEN_ADDR", "127.0.0.1:9090")]))
            .expect("valid override");
        assert_eq!(config.listen_addr, "127.0.0.1:9090");
    }

    #[test]
    fn vllm_base_url_is_overridable() {
        let config =
            ProxyConfig::resolve(&fake_env(&[("VLLM_BASE_URL", "http://vllm.internal:9000")]))
                .expect("valid override");
        assert_eq!(config.vllm_base_url, "http://vllm.internal:9000");
    }

    #[test]
    fn notice_prefix_is_overridable() {
        let config = ProxyConfig::resolve(&fake_env(&[("NOTICE_PREFIX", "[HEADS UP] ")]))
            .expect("valid override");
        assert_eq!(config.notice_prefix, "[HEADS UP] ");
    }

    #[test]
    fn log_level_is_overridable() {
        let config =
            ProxyConfig::resolve(&fake_env(&[("LOG_LEVEL", "debug")])).expect("valid override");
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn max_request_body_size_is_overridable_and_parses_as_bytes() {
        let config = ProxyConfig::resolve(&fake_env(&[("MAX_REQUEST_BODY_SIZE", "2048")]))
            .expect("valid override");
        assert_eq!(config.max_request_body_size, 2048);
    }

    #[test]
    fn upstream_timeouts_are_independently_overridable() {
        let config = ProxyConfig::resolve(&fake_env(&[
            ("UPSTREAM_CONNECT_TIMEOUT_SECS", "1"),
            ("UPSTREAM_TOTAL_TIMEOUT_SECS", "60"),
        ]))
        .expect("valid override");
        assert_eq!(config.connect_timeout, Duration::from_secs(1));
        assert_eq!(config.total_timeout, Duration::from_secs(60));
    }

    #[test]
    fn invalid_numeric_env_var_is_rejected_with_the_offending_name_and_value() {
        let err = ProxyConfig::resolve(&fake_env(&[("MAX_REQUEST_BODY_SIZE", "not-a-number")]))
            .unwrap_err();
        assert_eq!(err.var, "MAX_REQUEST_BODY_SIZE");
        assert_eq!(err.value, "not-a-number");
    }
}

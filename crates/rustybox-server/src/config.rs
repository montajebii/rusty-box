//! Configuration from the environment, resolved once at boot.
//!
//! Every setting has a Rusty Box name (`RUSTYBOX_*`) and a legacy name
//! (`IDEABUCKET_*`) that is still honored so existing setups and service
//! files keep working across the rename. A variable that is set but blank
//! counts as unset — `RUSTYBOX_PORT=""` must not parse as an error. When
//! both names are set, the Rusty Box name wins.
//!
//! Each resolved value remembers where it came from, so the boot log can say
//! `(default)`, `(from RUSTYBOX_DATA)`, or `(from IDEABUCKET_PORT (legacy
//! name, still honored))` — the human should never have to guess which
//! configuration actually applied.

use std::path::{Path, PathBuf};

/// Where a resolved value came from, kept so the boot log can be honest.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Default,
    /// Set under its Rusty Box name.
    Primary(&'static str),
    /// Set under its pre-rename name, still honored.
    Legacy(&'static str),
}

impl Source {
    pub fn suffix(&self) -> String {
        match self {
            Source::Default => "(default)".to_string(),
            Source::Primary(var) => format!("(from {var})"),
            Source::Legacy(var) => format!("(from {var} (legacy name, still honored))"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A port variable was set to something that is not a `u16`. The error
    /// names the exact variable that was used, so a stale legacy name in an
    /// old service file is not a mystery.
    BadPort { var: &'static str, value: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::BadPort { var, value } => {
                write!(f, "{var} must be a number, e.g. 8088 — got {value:?}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// The fully resolved runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub data: PathBuf,
    host_src: Source,
    port_src: Source,
    data_src: Source,
}

impl Config {
    /// The host the boot log should print, with its provenance suffix.
    pub fn host_line(&self) -> String {
        format!("host = {} {}", self.host, self.host_src.suffix())
    }

    /// The port line for the boot log.
    pub fn port_line(&self) -> String {
        format!("port = {} {}", self.port, self.port_src.suffix())
    }

    /// The data line for the boot log.
    pub fn data_line(&self) -> String {
        format!("data = {} {}", self.data.display(), self.data_src.suffix())
    }

    /// Resolve from the real process environment.
    pub fn from_env() -> Result<Config, ConfigError> {
        Self::resolve(|key| std::env::var(key).ok())
    }

    /// Resolve from any lookup — the seam that makes the whole matrix
    /// testable without mutating process-global env state.
    pub fn resolve(lookup: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        // blank counts as unset; the first name that yields a real value wins
        let pick =
            |primary: &'static str, legacy: &'static str, default: &str| -> (String, Source) {
                for (var, source) in [
                    (primary, Source::Primary(primary)),
                    (legacy, Source::Legacy(legacy)),
                ] {
                    if let Some(v) = lookup(var) {
                        if !v.trim().is_empty() {
                            return (v, source);
                        }
                    }
                }
                (default.to_string(), Source::Default)
            };

        let (host, host_src) = pick("RUSTYBOX_HOST", "IDEABUCKET_HOST", "127.0.0.1");

        let (port_raw, port_src) = pick("RUSTYBOX_PORT", "IDEABUCKET_PORT", "8088");
        let port: u16 = port_raw.trim().parse().map_err(|_| ConfigError::BadPort {
            var: match port_src {
                Source::Primary(var) | Source::Legacy(var) => var,
                Source::Default => "RUSTYBOX_PORT",
            },
            value: port_raw.clone(),
        })?;

        // data: the default keeps an existing v2.0 store where it is, so the
        // rename never strands the user's ideas in an orphaned file
        let existing = Path::new("data/idea-bucket.json");
        let default_data = if existing.is_file() {
            existing.to_path_buf()
        } else {
            PathBuf::from("data/rusty-box.json")
        };
        let (data_raw, data_src) = pick(
            "RUSTYBOX_DATA",
            "IDEABUCKET_DATA",
            &default_data.display().to_string(),
        );

        Ok(Config {
            host,
            port,
            data: PathBuf::from(data_raw),
            host_src,
            port_src,
            data_src,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A fake environment: a plain map, no process-global state.
    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn defaults_when_nothing_is_set() {
        let cfg = Config::resolve(env(&[])).unwrap();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8088);
        assert!(cfg.data.ends_with("data/rusty-box.json"));
        assert_eq!(cfg.host_line(), "host = 127.0.0.1 (default)");
        assert_eq!(cfg.port_line(), "port = 8088 (default)");
    }

    #[test]
    fn rustybox_names_win_over_legacy_names() {
        let cfg = Config::resolve(env(&[
            ("RUSTYBOX_PORT", "9001"),
            ("IDEABUCKET_PORT", "9002"),
            ("RUSTYBOX_DATA", "/tmp/new.json"),
            ("IDEABUCKET_DATA", "/tmp/old.json"),
        ]))
        .unwrap();
        assert_eq!(cfg.port, 9001);
        assert_eq!(cfg.data, PathBuf::from("/tmp/new.json"));
        assert_eq!(cfg.port_line(), "port = 9001 (from RUSTYBOX_PORT)");
        assert_eq!(cfg.data_line(), "data = /tmp/new.json (from RUSTYBOX_DATA)");
    }

    #[test]
    fn legacy_names_are_still_honored() {
        let cfg = Config::resolve(env(&[
            ("IDEABUCKET_PORT", "9002"),
            ("IDEABUCKET_DATA", "/tmp/old.json"),
        ]))
        .unwrap();
        assert_eq!(cfg.port, 9002);
        assert_eq!(cfg.data, PathBuf::from("/tmp/old.json"));
        assert_eq!(
            cfg.port_line(),
            "port = 9002 (from IDEABUCKET_PORT (legacy name, still honored))"
        );
        assert_eq!(
            cfg.data_line(),
            "data = /tmp/old.json (from IDEABUCKET_DATA (legacy name, still honored))"
        );
    }

    #[test]
    fn blank_values_count_as_unset() {
        let cfg = Config::resolve(env(&[
            ("RUSTYBOX_PORT", "   "),
            ("IDEABUCKET_PORT", "9003"),
        ]))
        .unwrap();
        assert_eq!(
            cfg.port, 9003,
            "blank primary falls through to the legacy name"
        );
        assert_eq!(
            cfg.port_line(),
            "port = 9003 (from IDEABUCKET_PORT (legacy name, still honored))"
        );
    }

    #[test]
    fn bad_port_error_names_the_primary_variable() {
        let err = Config::resolve(env(&[("RUSTYBOX_PORT", "not-a-port")])).unwrap_err();
        assert_eq!(
            err.to_string(),
            "RUSTYBOX_PORT must be a number, e.g. 8088 — got \"not-a-port\""
        );
    }

    #[test]
    fn bad_port_error_names_the_legacy_variable_when_it_is_the_one_used() {
        let err = Config::resolve(env(&[("IDEABUCKET_PORT", "0x50")])).unwrap_err();
        assert!(err
            .to_string()
            .starts_with("IDEABUCKET_PORT must be a number"),);
    }

    #[test]
    fn data_default_keeps_an_existing_v2_store_file() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = std::env::current_dir().unwrap();
        std::fs::create_dir_all(dir.path().join("data")).unwrap();
        std::fs::write(dir.path().join("data/idea-bucket.json"), "[]").unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let cfg = Config::resolve(env(&[])).unwrap();
        std::env::set_current_dir(cwd).unwrap();
        assert_eq!(cfg.data, Path::new("data/idea-bucket.json"));
        assert_eq!(
            cfg.data_line(),
            "data = data/idea-bucket.json (default)",
            "an existing v2.0 store is not orphaned by the rename"
        );
    }

    #[test]
    fn host_provenance_covers_all_three_cases() {
        let cfg = Config::resolve(env(&[])).unwrap();
        assert_eq!(cfg.host_line(), "host = 127.0.0.1 (default)");
        let cfg = Config::resolve(env(&[("RUSTYBOX_HOST", "0.0.0.0")])).unwrap();
        assert_eq!(cfg.host_line(), "host = 0.0.0.0 (from RUSTYBOX_HOST)");
        let cfg = Config::resolve(env(&[("IDEABUCKET_HOST", "0.0.0.0")])).unwrap();
        assert_eq!(
            cfg.host_line(),
            "host = 0.0.0.0 (from IDEABUCKET_HOST (legacy name, still honored))"
        );
    }
}

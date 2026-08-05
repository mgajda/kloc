//! AI-platform configuration.
//!
//! Platforms are defined in a TOML config file (default embedded in the
//! binary) so multiple AI providers can be calibrated independently, each
//! with its own token caps and effort multiplier. The file is discovered via
//! `$XDG_CONFIG_HOME/kloc/ai.toml` (or `~/.config/kloc/ai.toml`), overridable
//! with `--ai-config`; `--write-ai-config` emits the embedded default.

use std::path::PathBuf;

/// One AI platform: a name, a monotonic list of `(tokens, duration)`
/// breakpoints, and an optional effort multiplier (default 5x).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiPlatformCfg {
    pub name: String,
    /// `(tokens, duration_seconds)` caps, strictly increasing in tokens.
    pub caps: Vec<(u64, u64)>,
    /// Effort multiplier: effective = tokens × (1 + multiplier).
    /// Standard 3-5x, complex reasoning 10-20x.
    #[serde(default)]
    pub multiplier: Option<f64>,
}

/// The whole AI config file.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AiConfig {
    pub platforms: Vec<AiPlatformCfg>,
}

/// The embedded default configuration.
pub const DEFAULT_CONFIG: &str = include_str!("../assets/ai.toml");

/// The default config, parsed once.
pub fn default_config() -> AiConfig {
    parse(DEFAULT_CONFIG).expect("embedded default ai.toml must be valid")
}

/// Parse a TOML string into an `AiConfig`, validating platform caps monotonicity.
pub fn parse(text: &str) -> Result<AiConfig, String> {
    let cfg: AiConfig = toml::from_str(text).map_err(|e| format!("invalid AI config: {e}"))?;
    for p in &cfg.platforms {
        if p.caps.is_empty() {
            return Err(format!(
                "platform '{}': needs at least one (tokens, duration) cap",
                p.name
            ));
        }
        for (i, (t, d)) in p.caps.iter().enumerate() {
            if *t == 0 {
                return Err(format!(
                    "platform '{}': cap {i} token count must be > 0",
                    p.name
                ));
            }
            if *d == 0 {
                return Err(format!(
                    "platform '{}': cap {i} duration must be > 0",
                    p.name
                ));
            }
            if i > 0 {
                let (pt, pd) = p.caps[i - 1];
                if *t <= pt {
                    return Err(format!(
                        "platform '{}': caps not monotonic (tokens[{i}]={t} <= tokens[{}]={pt})",
                        p.name,
                        i - 1
                    ));
                }
                if *d <= pd {
                    return Err(format!(
                        "platform '{}': durations not monotonic (duration[{i}]={d} <= duration[{}]={pd})",
                        p.name,
                        i - 1
                    ));
                }
            }
        }
    }
    Ok(cfg)
}

/// The config-file path to load: `--ai-config` if given, else XDG discovery.
/// Returns `None` when no file exists (caller should fall back to the default).
pub fn config_path(override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return Some(PathBuf::from(p));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|b| b.join("kloc"));
    base.map(|b| b.join("ai.toml"))
}

/// Load the config: from the given/XDG path if the file exists, else the
/// embedded default.
pub fn load(override_path: Option<&str>) -> Result<AiConfig, String> {
    if let Some(path) = config_path(override_path)
        && path.exists()
    {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read AI config {}: {e}", path.display()))?;
        return parse(&text);
    }
    Ok(default_config())
}

/// Emit the default config to the given path (creating parent dirs).
pub fn write_default(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, DEFAULT_CONFIG)
        .map_err(|e| format!("failed to write AI config {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform(name: &str, caps: &[(u64, u64)]) -> String {
        let caps = caps
            .iter()
            .map(|(t, d)| format!("[{t}, {d}]"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[[platforms]]\nname = \"{name}\"\ncaps = [{caps}]\n")
    }

    #[test]
    fn parse_valid_config() {
        let cfg = parse(&platform("p", &[(100, 60), (200, 120)])).unwrap();
        assert_eq!(cfg.platforms.len(), 1);
        assert_eq!(cfg.platforms[0].caps, vec![(100, 60), (200, 120)]);
    }

    #[test]
    fn parse_rejects_empty_caps() {
        let e = parse("[[platforms]]\nname = \"p\"\ncaps = []\n").unwrap_err();
        assert!(e.contains("at least one"), "{e}");
    }

    #[test]
    fn parse_rejects_zero_token_cap() {
        let e = parse(&platform("p", &[(0, 60)])).unwrap_err();
        assert!(e.contains("token count must be > 0"), "{e}");
    }

    #[test]
    fn parse_rejects_zero_duration_cap() {
        let e = parse(&platform("p", &[(100, 0)])).unwrap_err();
        assert!(e.contains("duration must be > 0"), "{e}");
    }

    #[test]
    fn parse_rejects_non_monotonic_tokens() {
        let e = parse(&platform("p", &[(200, 60), (100, 120)])).unwrap_err();
        assert!(e.contains("caps not monotonic"), "{e}");
    }

    #[test]
    fn parse_rejects_non_monotonic_durations() {
        let e = parse(&platform("p", &[(100, 120), (200, 60)])).unwrap_err();
        assert!(e.contains("durations not monotonic"), "{e}");
    }

    #[test]
    fn parse_rejects_invalid_toml() {
        assert!(parse("not toml [[[").is_err());
    }

    #[test]
    fn config_path_override_wins() {
        assert_eq!(
            config_path(Some("/tmp/override/ai.toml")),
            Some(PathBuf::from("/tmp/override/ai.toml"))
        );
    }

    #[test]
    fn config_path_uses_xdg() {
        let _g = crate::TestEnvGuard::new(&["XDG_CONFIG_HOME", "HOME"]);
        _g.set("XDG_CONFIG_HOME", "/xdg");
        _g.remove("HOME");
        assert_eq!(config_path(None), Some(PathBuf::from("/xdg/kloc/ai.toml")));
    }

    #[test]
    fn config_path_falls_back_to_home() {
        let _g = crate::TestEnvGuard::new(&["XDG_CONFIG_HOME", "HOME"]);
        _g.remove("XDG_CONFIG_HOME");
        _g.set("HOME", "/home/t");
        assert_eq!(
            config_path(None),
            Some(PathBuf::from("/home/t/.config/kloc/ai.toml"))
        );
    }

    #[test]
    fn load_missing_file_falls_back_to_default() {
        let cfg = load(Some("/nonexistent/kloc/ai.toml")).unwrap();
        assert!(!cfg.platforms.is_empty());
    }

    #[test]
    fn load_invalid_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.toml");
        std::fs::write(&p, "bad [[[ toml").unwrap();
        assert!(load(Some(p.to_str().unwrap())).is_err());
    }

    #[test]
    fn load_reads_file_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ai.toml");
        std::fs::write(&p, platform("mine", &[(100, 60)])).unwrap();
        let cfg = load(Some(p.to_str().unwrap())).unwrap();
        assert_eq!(cfg.platforms[0].name, "mine");
    }

    #[test]
    fn write_default_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("nested/dir/ai.toml");
        write_default(&p).unwrap();
        assert!(p.exists());
        assert!(p.parent().unwrap().is_dir());
    }

    #[test]
    fn default_config_is_valid() {
        let cfg = default_config();
        assert!(!cfg.platforms.is_empty());
    }
}

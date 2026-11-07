use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::ApprovalLevelArg;

/// Environment variable holding an explicit configuration path.
pub const CONFIG_ENV: &str = "COWBOT_CONFIG";
/// Configuration file picked up from the working directory when present.
pub const DEFAULT_CONFIG_FILE: &str = "cowbot.toml";
/// Database used when neither the command line nor the file provides one.
pub const DEFAULT_DATABASE: &str = "cowbot.db";
/// Model used when neither the command line nor the file provides one.
pub const DEFAULT_MODEL: &str = "gpt-4o";
/// Maximum number of assignments executed at the same time by default.
pub const DEFAULT_CONCURRENCY: usize = 4;

/// Values read from `cowbot.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub db: Option<PathBuf>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub approval: Option<ApprovalLevelArg>,
    pub concurrency: Option<usize>,
}

/// Values supplied by the command line or its environment variables.
#[derive(Debug, Default)]
pub struct Overrides {
    pub db: Option<PathBuf>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub approval: Option<ApprovalLevelArg>,
    pub concurrency: Option<usize>,
}

/// Fully resolved CLI settings after applying precedence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub db: PathBuf,
    pub model: String,
    pub base_url: Option<String>,
    pub approval: ApprovalLevelArg,
    pub concurrency: usize,
}

/// Load the configuration file selected by an explicit path, `COWBOT_CONFIG`,
/// or `cowbot.toml` in the working directory.
///
/// A missing default file is not an error; a missing explicit file is.
pub fn load(explicit: Option<&Path>) -> Result<FileConfig> {
    let Some(path) = resolve_path(explicit) else {
        return Ok(FileConfig::default());
    };
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read configuration {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("failed to parse configuration {}", path.display()))
}

/// Resolve the effective settings. Command-line/environment values win over
/// the file, which wins over the built-in defaults.
pub fn resolve(overrides: Overrides, file: FileConfig) -> Settings {
    let model = overrides
        .model
        .or(file.model)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let base_url = overrides
        .base_url
        .or(file.base_url)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Settings {
        db: overrides
            .db
            .or(file.db)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DATABASE)),
        model: model.trim().to_string(),
        base_url,
        approval: overrides
            .approval
            .or(file.approval)
            .unwrap_or(ApprovalLevelArg::Ask),
        concurrency: overrides
            .concurrency
            .or(file.concurrency)
            .unwrap_or(DEFAULT_CONCURRENCY)
            .max(1),
    }
}

fn resolve_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os(CONFIG_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    let default = PathBuf::from(DEFAULT_CONFIG_FILE);
    default.exists().then_some(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn defaults_match_the_documented_values() {
        let settings = resolve(Overrides::default(), FileConfig::default());
        assert_eq!(settings.db, PathBuf::from("cowbot.db"));
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.base_url, None);
        assert_eq!(settings.approval, ApprovalLevelArg::Ask);
        assert_eq!(settings.concurrency, 4);
    }

    #[test]
    fn an_explicit_missing_file_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing.toml");
        assert!(load(Some(&missing)).is_err());
    }

    #[test]
    fn file_values_override_defaults() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cowbot.toml");
        fs::write(
            &path,
            "db = \"team.db\"\nmodel = \"deepseek-chat\"\nbase_url = \"https://api.deepseek.com\"\napproval = \"safe-auto\"\nconcurrency = 2\n",
        )
        .unwrap();

        let settings = resolve(Overrides::default(), load(Some(&path)).unwrap());
        assert_eq!(settings.db, PathBuf::from("team.db"));
        assert_eq!(settings.model, "deepseek-chat");
        assert_eq!(
            settings.base_url.as_deref(),
            Some("https://api.deepseek.com")
        );
        assert_eq!(settings.approval, ApprovalLevelArg::SafeAuto);
        assert_eq!(settings.concurrency, 2);
    }

    #[test]
    fn explicit_values_win_over_the_file() {
        let file = FileConfig {
            db: Some(PathBuf::from("file.db")),
            model: Some("file-model".to_string()),
            base_url: Some("https://file.example".to_string()),
            approval: Some(ApprovalLevelArg::PlanOnly),
            concurrency: Some(1),
        };
        let overrides = Overrides {
            db: Some(PathBuf::from("cli.db")),
            model: Some("cli-model".to_string()),
            base_url: Some("https://cli.example".to_string()),
            approval: Some(ApprovalLevelArg::Trusted),
            concurrency: Some(8),
        };

        let settings = resolve(overrides, file);
        assert_eq!(settings.db, PathBuf::from("cli.db"));
        assert_eq!(settings.model, "cli-model");
        assert_eq!(settings.base_url.as_deref(), Some("https://cli.example"));
        assert_eq!(settings.approval, ApprovalLevelArg::Trusted);
        assert_eq!(settings.concurrency, 8);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cowbot.toml");
        fs::write(&path, "db = \"cowbot.db\"\nunknown = true\n").unwrap();

        assert!(load(Some(&path)).is_err());
    }

    #[test]
    fn concurrency_is_never_zero() {
        let file = FileConfig {
            concurrency: Some(0),
            ..FileConfig::default()
        };
        assert_eq!(resolve(Overrides::default(), file).concurrency, 1);
    }
}

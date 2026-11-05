//! Local model provider configuration for the desktop app.
//!
//! The desktop reads and writes a single `config.json` in its app data
//! directory. The file holds an ordered list of OpenAI-compatible providers and
//! the id of the active one. API keys never cross the Tauri command boundary
//! back to the frontend; the UI only learns whether a provider has a stored key.

use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cowbot_llm::{
    AnthropicProvider, LlmProvider, MockLlmProvider, OpenAiCompatibleProvider, ResponsesProvider,
};
use serde::{Deserialize, Serialize};

/// File name of the desktop configuration inside the app data directory.
pub const CONFIG_FILE: &str = "config.json";

/// Format version written by this build.
pub const CONFIG_VERSION: u32 = 1;

/// Default model shown when no provider is active.
pub const DEFAULT_MODEL: &str = "gpt-4o";

/// Wire protocol a saved provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProviderProtocol {
    /// OpenAI Chat Completions (`/v1/chat/completions`).
    #[default]
    #[serde(rename = "openai")]
    OpenAi,
    /// Anthropic Messages (`/v1/messages`).
    #[serde(rename = "anthropic")]
    Anthropic,
    /// OpenAI Responses (`/v1/responses`).
    #[serde(rename = "openai-responses")]
    OpenAiResponses,
}

/// One model provider saved by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    /// Missing in files written before protocol selection existed; those
    /// providers are OpenAI-compatible.
    #[serde(default)]
    pub protocol: ProviderProtocol,
    pub base_url: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Whole desktop configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default)]
    pub active_provider_id: Option<String>,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    /// Whether the guided first-run setup has been completed or skipped.
    ///
    /// Legacy files written before onboarding existed are treated as completed
    /// so an upgrade does not show the guide to an existing user.
    #[serde(default = "onboarding_default_for_legacy")]
    pub onboarding_completed: bool,
}

fn current_version() -> u32 {
    CONFIG_VERSION
}

fn onboarding_default_for_legacy() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            active_provider_id: None,
            providers: Vec::new(),
            onboarding_completed: false,
        }
    }
}

impl AppConfig {
    /// Look up a provider by id.
    pub fn provider(&self, id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    /// The active provider, if one is configured and still present.
    pub fn active_provider(&self) -> Option<&ProviderConfig> {
        let id = self.active_provider_id.as_deref()?;
        self.provider(id)
    }
}

/// Which backend an effective model configuration uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Mock,
    Remote(ProviderProtocol),
}

/// The model configuration currently in effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub kind: ProviderKind,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

/// Path of the configuration file inside the app data directory.
pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CONFIG_FILE)
}

/// Load the configuration, treating a missing file as the default.
pub fn load(path: &Path) -> io::Result<AppConfig> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid configuration: {error}"),
            )
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(error) => Err(error),
    }
}

/// Write the configuration atomically with owner-only permissions where the
/// platform supports it.
pub fn save(path: &Path, config: &AppConfig) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string_pretty(config)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temporary = path.with_extension("json.tmp");
    write_private(&temporary, &contents)?;
    restrict_permissions(&temporary)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn write_private(path: &Path, contents: &str) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Validated provider fields, trimmed and normalized.
pub fn validate_endpoint(base_url: &str, model: &str) -> Result<(String, String), String> {
    let base_url = normalize_base_url(base_url)?;
    let model = model.trim();
    if model.is_empty() {
        return Err("Model is required.".to_string());
    }
    Ok((base_url, model.to_string()))
}

/// Normalize a base URL, rejecting anything that is not an absolute HTTP(S) URL.
pub fn normalize_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let host = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"));
    match host {
        Some(rest) if !rest.is_empty() => Ok(trimmed.to_string()),
        _ => Err("Base URL must start with http:// or https:// and include a host.".to_string()),
    }
}

/// Trim a typed API key, treating a blank value as absent.
pub fn normalize_api_key(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

/// Build a validated provider.
pub fn make_provider(
    id: String,
    name: &str,
    protocol: ProviderProtocol,
    base_url: &str,
    model: &str,
    api_key: Option<&str>,
) -> Result<ProviderConfig, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Provider name is required.".to_string());
    }
    let (base_url, model) = validate_endpoint(base_url, model)?;
    Ok(ProviderConfig {
        id,
        name: name.to_string(),
        protocol,
        base_url,
        model,
        api_key: normalize_api_key(api_key),
    })
}

/// Generate an id for a new provider that does not collide with existing ones.
pub fn new_provider_id(existing: &[ProviderConfig]) -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    provider_id_for(millis, existing)
}

fn provider_id_for(millis: u128, existing: &[ProviderConfig]) -> String {
    let mut candidate = format!("provider-{millis}");
    let mut suffix = 1;
    while existing.iter().any(|provider| provider.id == candidate) {
        candidate = format!("provider-{millis}-{suffix}");
        suffix += 1;
    }
    candidate
}

/// Resolve the active provider, falling back to the offline mock model.
pub fn resolve_active(config: &AppConfig) -> ResolvedModel {
    match config.active_provider() {
        Some(provider) => ResolvedModel {
            kind: ProviderKind::Remote(provider.protocol),
            model: provider.model.clone(),
            base_url: Some(provider.base_url.clone()),
            api_key: provider.api_key.clone(),
        },
        None => ResolvedModel {
            kind: ProviderKind::Mock,
            model: DEFAULT_MODEL.to_string(),
            base_url: None,
            api_key: None,
        },
    }
}

/// Build the LLM client for an effective model configuration.
pub fn build_provider(resolved: &ResolvedModel) -> Arc<dyn LlmProvider> {
    match resolved.kind {
        ProviderKind::Mock => Arc::new(MockLlmProvider::echo()),
        ProviderKind::Remote(ProviderProtocol::OpenAi) => Arc::new(OpenAiCompatibleProvider::new(
            resolved.base_url.clone().unwrap_or_default(),
            resolved.api_key.clone(),
        )),
        ProviderKind::Remote(ProviderProtocol::Anthropic) => Arc::new(AnthropicProvider::new(
            resolved.base_url.clone().unwrap_or_default(),
            resolved.api_key.clone(),
        )),
        ProviderKind::Remote(ProviderProtocol::OpenAiResponses) => {
            Arc::new(ResponsesProvider::new(
                resolved.base_url.clone().unwrap_or_default(),
                resolved.api_key.clone(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider(id: &str, name: &str) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            name: name.to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "https://api.example.com/v1".to_string(),
            model: "example-model".to_string(),
            api_key: Some("secret".to_string()),
        }
    }

    #[test]
    fn missing_file_loads_defaults() {
        let root = tempfile::tempdir().unwrap();
        let config = load(&root.path().join(CONFIG_FILE)).unwrap();
        assert_eq!(config, AppConfig::default());
        assert!(config.providers.is_empty());
        assert!(config.active_provider_id.is_none());
        assert!(!config.onboarding_completed);
    }

    #[test]
    fn legacy_config_files_are_treated_as_onboarded() {
        let root = tempfile::tempdir().unwrap();
        let path = config_path(root.path());
        fs::write(
            &path,
            r#"{"version":1,"active_provider_id":null,"providers":[]}"#,
        )
        .unwrap();

        assert!(load(&path).unwrap().onboarding_completed);
    }

    #[test]
    fn explicit_onboarding_state_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let path = config_path(root.path());
        fs::write(
            &path,
            r#"{"version":1,"providers":[],"onboarding_completed":false}"#,
        )
        .unwrap();

        assert!(!load(&path).unwrap().onboarding_completed);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let root = tempfile::tempdir().unwrap();
        let path = config_path(root.path());
        let config = AppConfig {
            version: CONFIG_VERSION,
            active_provider_id: Some("one".to_string()),
            providers: vec![sample_provider("one", "Example")],
            onboarding_completed: true,
        };

        save(&path, &config).unwrap();

        assert_eq!(load(&path).unwrap(), config);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn load_tolerates_unknown_fields() {
        let root = tempfile::tempdir().unwrap();
        let path = config_path(root.path());
        fs::write(
            &path,
            r#"{"version":1,"active_provider_id":null,"providers":[],"future":true}"#,
        )
        .unwrap();

        let config = load(&path).unwrap();
        assert!(config.providers.is_empty());
    }

    #[test]
    fn providers_without_a_protocol_default_to_openai() {
        let root = tempfile::tempdir().unwrap();
        let path = config_path(root.path());
        fs::write(
            &path,
            r#"{"version":1,"active_provider_id":"p1","providers":[{"id":"p1","name":"Legacy","base_url":"https://api.example.com/v1","model":"m"}]}"#,
        )
        .unwrap();

        let config = load(&path).unwrap();

        assert_eq!(config.providers[0].protocol, ProviderProtocol::OpenAi);
    }

    #[test]
    fn protocols_serialize_to_stable_names() {
        let config = AppConfig {
            providers: vec![
                ProviderConfig {
                    protocol: ProviderProtocol::OpenAi,
                    ..sample_provider("one", "One")
                },
                ProviderConfig {
                    protocol: ProviderProtocol::Anthropic,
                    ..sample_provider("two", "Two")
                },
                ProviderConfig {
                    protocol: ProviderProtocol::OpenAiResponses,
                    ..sample_provider("three", "Three")
                },
            ],
            ..AppConfig::default()
        };

        let value = serde_json::to_value(&config).unwrap();

        assert_eq!(value["providers"][0]["protocol"], "openai");
        assert_eq!(value["providers"][1]["protocol"], "anthropic");
        assert_eq!(value["providers"][2]["protocol"], "openai-responses");
    }

    #[test]
    fn resolve_uses_the_active_provider() {
        let config = AppConfig {
            providers: vec![sample_provider("one", "One"), sample_provider("two", "Two")],
            active_provider_id: Some("two".to_string()),
            ..AppConfig::default()
        };

        let resolved = resolve_active(&config);

        assert_eq!(
            resolved.kind,
            ProviderKind::Remote(ProviderProtocol::OpenAi)
        );
        assert_eq!(resolved.model, "example-model");
        assert_eq!(
            resolved.base_url.as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(resolved.api_key.as_deref(), Some("secret"));
    }

    #[test]
    fn resolve_carries_the_provider_protocol() {
        let config = AppConfig {
            providers: vec![ProviderConfig {
                protocol: ProviderProtocol::Anthropic,
                ..sample_provider("claude", "Claude")
            }],
            active_provider_id: Some("claude".to_string()),
            ..AppConfig::default()
        };

        assert_eq!(
            resolve_active(&config).kind,
            ProviderKind::Remote(ProviderProtocol::Anthropic)
        );
    }

    #[test]
    fn resolve_falls_back_to_mock_without_an_active_provider() {
        let config = AppConfig {
            providers: vec![sample_provider("one", "One")],
            ..AppConfig::default()
        };

        let resolved = resolve_active(&config);

        assert_eq!(resolved.kind, ProviderKind::Mock);
        assert_eq!(resolved.model, DEFAULT_MODEL);
        assert!(resolved.base_url.is_none());
        assert!(resolved.api_key.is_none());
    }

    #[test]
    fn resolve_falls_back_to_mock_when_the_active_id_is_unknown() {
        let config = AppConfig {
            providers: vec![sample_provider("one", "One")],
            active_provider_id: Some("missing".to_string()),
            ..AppConfig::default()
        };

        assert_eq!(resolve_active(&config).kind, ProviderKind::Mock);
    }

    #[test]
    fn base_url_requires_an_absolute_http_url() {
        assert!(normalize_base_url("api.example.com/v1").is_err());
        assert!(normalize_base_url("ftp://api.example.com").is_err());
        assert!(normalize_base_url("https://").is_err());
        assert_eq!(
            normalize_base_url("https://api.example.com/v1/").unwrap(),
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn provider_requires_a_name_model_and_url() {
        assert!(make_provider(
            "id".into(),
            " ",
            ProviderProtocol::OpenAi,
            "https://a.test/v1",
            "m",
            None
        )
        .is_err());
        assert!(
            make_provider("id".into(), "Name", ProviderProtocol::OpenAi, "", "m", None).is_err()
        );
        assert!(make_provider(
            "id".into(),
            "Name",
            ProviderProtocol::OpenAi,
            "https://a.test/v1",
            " ",
            None
        )
        .is_err());
        assert!(make_provider(
            "id".into(),
            "Name",
            ProviderProtocol::Anthropic,
            "https://a.test/v1",
            "m",
            None
        )
        .is_ok());
    }

    #[test]
    fn blank_api_keys_are_treated_as_absent() {
        assert_eq!(normalize_api_key(None), None);
        assert_eq!(normalize_api_key(Some("   ")), None);
        assert_eq!(normalize_api_key(Some(" key ")), Some("key".to_string()));
    }

    #[test]
    fn new_provider_ids_do_not_collide() {
        let existing = vec![sample_provider("provider-42", "One")];
        assert_eq!(provider_id_for(42, &existing), "provider-42-1");
        assert_eq!(provider_id_for(43, &existing), "provider-43");
        assert!(new_provider_id(&existing).starts_with("provider-"));
    }

    #[cfg(unix)]
    #[test]
    fn saved_config_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let path = config_path(root.path());
        save(&path, &AppConfig::default()).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

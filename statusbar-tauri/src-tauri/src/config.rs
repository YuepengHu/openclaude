use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Providers configured in `~/.openclaude/providers.json` (shared with the CLI).
pub const PROVIDERS: &[&str] = &[
    "bmw-beacon",
    "anthropic",
    "gemini",
    "vertex",
    "bedrock",
    "ollama",
    "github",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusbarConfig {
    #[serde(default = "default_provider")]
    pub last_provider: String,
    #[serde(default = "default_model")]
    pub last_model: String,
    #[serde(default)]
    pub model_history: Vec<String>,
}

fn default_provider() -> String {
    "bmw-beacon".to_string()
}

fn default_model() -> String {
    "DeepSeek-V4-Pro".to_string()
}

impl Default for StatusbarConfig {
    fn default() -> Self {
        Self {
            last_provider: default_provider(),
            last_model: default_model(),
            model_history: Vec::new(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".openclaude/statusbar/config.json")
}

pub fn providers_json_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".openclaude/providers.json")
}

pub fn load() -> StatusbarConfig {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_choices(provider: &str, model: &str) {
    let mut cfg = load();
    cfg.last_provider = provider.to_string();
    cfg.last_model = model.to_string();

    if let Some(idx) = cfg.model_history.iter().position(|m| m == model) {
        cfg.model_history.remove(idx);
    }
    cfg.model_history.insert(0, model.to_string());
    cfg.model_history.truncate(10);

    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(path, json);
    }
}

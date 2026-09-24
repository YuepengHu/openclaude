use crate::config;
use serde_json::Value;
use std::collections::HashMap;

fn providers_json() -> Option<Value> {
    let text = std::fs::read_to_string(config::providers_json_path()).ok()?;
    serde_json::from_str(&text).ok()
}

fn provider_env(provider: &str) -> HashMap<String, String> {
    let Some(json) = providers_json() else {
        return HashMap::new();
    };
    json.get(provider)
        .and_then(|p| p.get("env"))
        .and_then(|e| e.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Models already known locally (default model + configured mlm_models), no network required.
pub fn configured_models(provider: &str) -> Vec<String> {
    let Some(json) = providers_json() else {
        return Vec::new();
    };
    let mut models = Vec::new();
    if let Some(p) = json.get(provider) {
        if let Some(default_model) = p
            .get("env")
            .and_then(|e| e.get("OPENAI_MODEL"))
            .and_then(|v| v.as_str())
        {
            if !default_model.is_empty() {
                models.push(default_model.to_string());
            }
        }
        if let Some(mlm) = p.get("mlm_models").and_then(|v| v.as_array()) {
            for m in mlm {
                if let Some(s) = m.as_str() {
                    models.push(s.to_string());
                }
            }
        }
    }
    models
}

/// Fetches the `/models` list from the provider's OpenAI-compatible endpoint.
/// Unlike the original Swift implementation, TLS certificates are always verified
/// (the old code trusted any certificate, which is an MITM risk).
pub async fn fetch_remote_models(provider: &str) -> Vec<String> {
    let env = provider_env(provider);
    let Some(base) = env.get("OPENAI_BASE_URL") else {
        return Vec::new();
    };
    let base = base.trim_end_matches('/');
    let url = format!("{base}/models");

    let client = reqwest::Client::new();
    let mut request = client.get(&url);
    if let Some(key) = env.get("OPENAI_API_KEY") {
        if !key.is_empty() {
            request = request
                .header("Authorization", format!("Bearer {key}"))
                .header("api-key", key.as_str());
        }
    }

    let Ok(response) = request.send().await else {
        return Vec::new();
    };
    let Ok(json) = response.json::<Value>().await else {
        return Vec::new();
    };

    let items = if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
        arr.clone()
    } else if let Some(arr) = json.as_array() {
        arr.clone()
    } else {
        Vec::new()
    };

    items
        .into_iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(|v| v.as_str().map(str::to_string))
                .or_else(|| item.as_str().map(str::to_string))
        })
        .collect()
}

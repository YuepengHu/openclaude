use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, rename = "relativeTime")]
    pub relative_time: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub model: String,
}

fn script_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".openclaude/statusbar/list_sessions.py")
}

/// Reuses the existing cross-platform `list_sessions.py` helper script.
pub fn load_sessions(limit: u32) -> Vec<SessionInfo> {
    for python in ["python3", "python"] {
        let output = Command::new(python)
            .arg(script_path())
            .arg("--limit")
            .arg(limit.to_string())
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                if let Ok(sessions) = serde_json::from_slice::<Vec<SessionInfo>>(&out.stdout) {
                    return sessions;
                }
            }
        }
    }
    Vec::new()
}

pub fn workspace_history(sessions: &[SessionInfo]) -> Vec<String> {
    let mut paths = Vec::new();
    for s in sessions {
        if s.cwd.is_empty() {
            continue;
        }
        let normalized = s.cwd.trim_end_matches('/').to_string();
        if !paths.contains(&normalized) {
            paths.push(normalized);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let home = home.to_string_lossy().to_string();
        if !paths.contains(&home) {
            paths.push(home);
        }
    }
    paths
}

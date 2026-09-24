use std::path::Path;
use std::process::Command;

/// Wraps a value in single quotes for safe use inside a POSIX shell string.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn build_command(workspace: &str, provider: &str, model: &str, resume: Option<&str>) -> String {
    let start_script = dirs::home_dir()
        .map(|h| h.join("Yuepeng/github/openclaude/start-bmw.sh"))
        .filter(|p| p.exists());

    let base = match start_script {
        // The BMW-internal launch script only runs under a POSIX shell.
        Some(script) if cfg!(unix) => format!(
            "{} --provider {} --model {}",
            shell_quote(&script.to_string_lossy()),
            shell_quote(provider),
            shell_quote(model)
        ),
        _ => format!(
            "openclaude --provider {} --model {}",
            shell_quote(provider),
            shell_quote(model)
        ),
    };

    let base = match resume {
        Some(id) => format!("{base} --resume {}", shell_quote(id)),
        None => base,
    };

    format!("cd {} && {base}", shell_quote(workspace))
}

#[cfg(target_os = "macos")]
fn open_in_terminal(command: &str) -> Result<(), String> {
    let has_iterm = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to (name of processes) contains "iTerm2""#)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false);

    // Escape for embedding inside an AppleScript double-quoted string literal.
    let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
    let script = if has_iterm {
        format!(
            "tell application \"iTerm\"\n\
             \tactivate\n\
             \ttell current window\n\
             \t\tcreate tab with default profile\n\
             \t\ttell current session\n\
             \t\t\twrite text \"{escaped}\"\n\
             \t\tend tell\n\
             \tend tell\n\
             end tell"
        )
    } else {
        format!(
            "tell application \"Terminal\"\n\
             \tactivate\n\
             \tdo script \"{escaped}\"\n\
             end tell"
        )
    };

    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("osascript failed to open a terminal".to_string())
    }
}

#[cfg(target_os = "windows")]
fn open_in_terminal(command: &str) -> Result<(), String> {
    // Prefer Windows Terminal; fall back to a plain cmd.exe window.
    if Command::new("wt.exe")
        .args(["-w", "0", "nt", "cmd.exe", "/k", command])
        .spawn()
        .is_ok()
    {
        return Ok(());
    }
    Command::new("cmd.exe")
        .args(["/C", "start", "cmd.exe", "/K", command])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn open_in_terminal(command: &str) -> Result<(), String> {
    let candidates: [(&str, Vec<&str>); 4] = [
        ("x-terminal-emulator", vec!["-e", "bash", "-lc", command]),
        ("gnome-terminal", vec!["--", "bash", "-lc", command]),
        ("konsole", vec!["-e", "bash", "-lc", command]),
        ("xterm", vec!["-e", "bash", "-lc", command]),
    ];
    for (bin, args) in candidates {
        if Command::new(bin).args(&args).spawn().is_ok() {
            return Ok(());
        }
    }
    Err("no supported terminal emulator found (tried x-terminal-emulator, gnome-terminal, konsole, xterm)".to_string())
}

pub fn launch(
    workspace: &str,
    provider: &str,
    model: &str,
    resume: Option<&str>,
) -> Result<(), String> {
    if !Path::new(workspace).is_dir() {
        return Err(format!("Directory does not exist: {workspace}"));
    }
    let command = build_command(workspace, provider, model, resume);
    open_in_terminal(&command)
}

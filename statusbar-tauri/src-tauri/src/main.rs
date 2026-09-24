mod config;
mod launch;
mod models;
mod sessions;

use sessions::SessionInfo;
use std::sync::Mutex;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};

struct AppState {
    sessions: Mutex<Vec<SessionInfo>>,
}

#[tauri::command]
fn get_defaults() -> config::StatusbarConfig {
    config::load()
}

#[tauri::command]
fn save_choices(provider: String, model: String) {
    config::save_choices(&provider, &model);
}

#[tauri::command]
fn providers() -> Vec<&'static str> {
    config::PROVIDERS.to_vec()
}

#[tauri::command]
fn configured_models(provider: String) -> Vec<String> {
    models::configured_models(&provider)
}

#[tauri::command]
async fn fetch_remote_models(provider: String) -> Vec<String> {
    models::fetch_remote_models(&provider).await
}

#[tauri::command]
fn workspace_history(state: tauri::State<AppState>) -> Vec<String> {
    sessions::workspace_history(&state.sessions.lock().unwrap())
}

#[tauri::command]
fn launch_workspace(
    workspace: String,
    provider: String,
    model: String,
    resume: Option<String>,
) -> Result<(), String> {
    config::save_choices(&provider, &model);
    launch::launch(&workspace, &provider, &model, resume.as_deref())
}

fn refresh_sessions(app: &tauri::AppHandle) -> Vec<SessionInfo> {
    let sessions = sessions::load_sessions(15);
    *app.state::<AppState>().sessions.lock().unwrap() = sessions.clone();
    sessions
}

fn build_tray_menu(
    app: &tauri::AppHandle,
    sessions: &[SessionInfo],
) -> tauri::Result<Menu<tauri::Wry>> {
    let launch_item = MenuItem::with_id(app, "launch", "Launch OpenClaude", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;

    let mut session_items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();
    if sessions.is_empty() {
        session_items.push(Box::new(MenuItem::with_id(
            app,
            "no-sessions",
            "No history sessions",
            false,
            None::<&str>,
        )?));
    } else {
        for s in sessions {
            let mut label = if s.relative_time.is_empty() {
                s.title.clone()
            } else {
                format!("{} ({})", s.title, s.relative_time)
            };
            if label.chars().count() > 80 {
                label = format!("{}...", label.chars().take(77).collect::<String>());
            }
            session_items.push(Box::new(MenuItem::with_id(
                app,
                format!("resume:{}", s.id),
                label,
                true,
                None::<&str>,
            )?));
        }
        session_items.push(Box::new(PredefinedMenuItem::separator(app)?));
        session_items.push(Box::new(MenuItem::with_id(
            app,
            "refresh-sessions",
            "Refresh",
            true,
            None::<&str>,
        )?));
    }
    let refs: Vec<&dyn IsMenuItem<tauri::Wry>> = session_items.iter().map(|b| b.as_ref()).collect();
    let sessions_submenu =
        Submenu::with_id_and_items(app, "recent-sessions", "Recent Sessions", true, &refs)?;

    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit OpenClaude Bar", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[&launch_item, &sep1, &sessions_submenu, &sep2, &quit_item],
    )
}

fn show_picker(app: &tauri::AppHandle, event_name: &str, payload: impl serde::Serialize + Clone) {
    if let Some(window) = app.get_webview_window("picker") {
        let _ = window.emit(event_name, payload);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            sessions: Mutex::new(Vec::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_defaults,
            save_choices,
            providers,
            configured_models,
            fetch_remote_models,
            workspace_history,
            launch_workspace,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let sessions = refresh_sessions(&handle);
            let menu = build_tray_menu(&handle, &sessions)?;

            TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("OpenClaude - Click to launch")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "launch" => show_picker(app, "picker:reset", ()),
                    "quit" => app.exit(0),
                    "refresh-sessions" => {
                        let sessions = refresh_sessions(app);
                        if let Ok(menu) = build_tray_menu(app, &sessions) {
                            if let Some(tray) = app.tray_by_id("main") {
                                let _ = tray.set_menu(Some(menu));
                            }
                        }
                    }
                    id if id.starts_with("resume:") => {
                        let session_id = id.trim_start_matches("resume:").to_string();
                        show_picker(app, "picker:resume", session_id);
                    }
                    _ => {}
                })
                .build(app)?;

            if let Some(picker) = app.get_webview_window("picker") {
                let picker_clone = picker.clone();
                picker.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = picker_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the OpenClaude statusbar app");
}

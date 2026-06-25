#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use axum::{extract::State, routing::post, Json, Router};
use serde_json::Value;
use tauri::PhysicalPosition;
use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

#[tauri::command]
fn setup_claude_hooks(project_path: Option<String>) -> Result<(), String> {
    let settings_path = if let Some(path) = project_path {
        std::path::PathBuf::from(path)
            .join(".claude")
            .join("settings.json")
    } else {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "無法取得使用者目錄".to_string())?;
        std::path::PathBuf::from(home)
            .join(".claude")
            .join("settings.json")
    };

    let mut config: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("讀取設定失敗: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if config.get("hooks").is_none() {
        config["hooks"] = serde_json::json!({});
    }

    let hooks = config["hooks"]
        .as_object_mut()
        .ok_or("hooks 格式錯誤".to_string())?;

    fn ensure_hook(
        hooks: &mut serde_json::Map<String, serde_json::Value>,
        event: &str,
        new_entry: serde_json::Value,
        target_url: &str,
    ) {
        let arr = hooks
            .entry(event)
            .or_insert(serde_json::json!([]))
            .as_array_mut()
            .unwrap();
        let already_exists = arr.iter().any(|item| {
            item.get("hooks")
                .and_then(|h| h.as_array())
                .map(|h| {
                    h.iter().any(|hook| {
                        hook.get("url").and_then(|u| u.as_str()) == Some(target_url)
                    })
                })
                .unwrap_or(false)
        });
        if !already_exists {
            arr.push(new_entry);
        }
    }

    ensure_hook(
        hooks,
        "Notification",
        serde_json::json!({
            "matcher": ".*",
            "hooks": [{ "type": "http", "url": "http://127.0.0.1:9527/claude-notify" }]
        }),
        "http://127.0.0.1:9527/claude-notify",
    );
    ensure_hook(
        hooks,
        "Stop",
        serde_json::json!({
            "matcher": ".*",
            "hooks": [{ "type": "http", "url": "http://127.0.0.1:9527/claude-notify" }]
        }),
        "http://127.0.0.1:9527/claude-notify",
    );
    ensure_hook(
        hooks,
        "UserPromptSubmit",
        serde_json::json!({
            "hooks": [{ "type": "http", "url": "http://127.0.0.1:9527/claude-start" }]
        }),
        "http://127.0.0.1:9527/claude-start",
    );

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("建立目錄失敗: {}", e))?;
    }

    let content =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失敗: {}", e))?;
    std::fs::write(&settings_path, content).map_err(|e| format!("寫入設定失敗: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn handle_notify(State(app): State<AppHandle>, Json(payload): Json<Value>) {
    app.emit("claude-event", payload).unwrap_or_else(|e| {
        println!("發送事件失敗: {}", e);
    });
}

async fn handle_start(State(app): State<AppHandle>) {
    app.emit("claude-start", ()).unwrap_or_else(|e| {
        println!("發送 claude-start 事件失敗: {}", e);
    });
}

fn toggle_window_visibility(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().ok();
        } else {
            window.show().ok();
            window.set_focus().ok();
        }
    }
}

fn check_update_in_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_updater::UpdaterExt;
        if let Ok(updater) = app.updater() {
            match updater.check().await {
                Ok(Some(update)) => {
                    app.emit(
                        "update-available",
                        serde_json::json!({
                            "version": update.version,
                            "currentVersion": update.current_version,
                            "body": update.body
                        }),
                    )
                    .ok();
                }
                Ok(None) => {
                    app.emit("update-not-available", ()).ok();
                }
                Err(e) => {
                    println!("檢查更新失敗: {}", e);
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![setup_claude_hooks, install_update])
        .setup(|app| {
            // 將視窗定位到左下角
            if let Some(window) = app.get_webview_window("main") {
                if let Some(monitor) = window.current_monitor().ok().flatten() {
                    let screen_size = monitor.size();
                    let win_size = window
                        .outer_size()
                        .unwrap_or(tauri::PhysicalSize::new(250, 250));
                    let taskbar: u32 = 48; // Windows 工作列高度
                    window
                        .set_position(PhysicalPosition::new(
                            (screen_size.width - win_size.width) as i32,
                            (screen_size.height - win_size.height - taskbar) as i32,
                        ))
                        .ok();
                }
            }

            // 建立系統匣圖示與選單
            let toggle_item = MenuItem::with_id(app, "toggle", "隱藏寵物", true, None::<&str>)?;
            let check_update_item =
                MenuItem::with_id(app, "check_update", "檢查更新", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_item, &check_update_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window_visibility(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        toggle_window_visibility(app);
                    }
                    "check_update" => {
                        check_update_in_background(app.clone());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let app_handle = app.handle().clone();

            // 在背景啟動本地伺服器
            tauri::async_runtime::spawn(async move {
                let router = Router::new()
                    .route("/claude-notify", post(handle_notify))
                    .route("/claude-start", post(handle_start))
                    .with_state(app_handle);

                let listener = tokio::net::TcpListener::bind("127.0.0.1:9527")
                    .await
                    .unwrap();
                println!("Webhook 伺服器啟動於 http://127.0.0.1:9527");
                axum::serve(listener, router).await.unwrap();
            });

            // 啟動 3 秒後靜默檢查更新
            let app_handle_update = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                check_update_in_background(app_handle_update);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("啟動 Tauri 應用程式失敗");
}

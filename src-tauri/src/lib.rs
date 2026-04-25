mod bilibili;
mod storage;
mod auto_reply;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

#[tauri::command]
async fn get_qr_code() -> Result<bilibili::QrCodeResponse, String> {
    bilibili::get_qr_code().await
}

#[tauri::command]
async fn check_login_status() -> Result<bilibili::LoginStatus, String> {
    bilibili::check_login_status().await
}

#[tauri::command]
async fn get_accounts() -> Result<Vec<storage::Account>, String> {
    storage::get_accounts().await
}

#[tauri::command]
async fn sync_accounts(accounts: Vec<storage::Account>) -> Result<Vec<storage::Account>, String> {
    storage::sync_accounts(accounts).await
}

#[tauri::command]
async fn activate_account(uid: String) -> Result<(), String> {
    storage::activate_account(uid).await
}

#[tauri::command]
async fn delete_account(uid: String) -> Result<(), String> {
    storage::delete_account(uid).await
}

#[tauri::command]
async fn get_auto_reply_settings() -> Result<auto_reply::AutoReplySettings, String> {
    auto_reply::get_settings().await
}

#[tauri::command]
async fn save_auto_reply_settings(settings: auto_reply::AutoReplySettings) -> Result<(), String> {
    auto_reply::save_settings(settings).await
}

#[tauri::command]
async fn test_auto_reply() -> Result<String, String> {
    auto_reply::test_reply().await
}

#[tauri::command]
async fn manual_reply_video_comments() -> Result<String, String> {
    auto_reply::manual_reply_comments().await
}

// ============================================================
//  开机自启
// ============================================================

#[tauri::command]
async fn get_autostart_status(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args([] as [&str; 0])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_qr_code,
            check_login_status,
            get_accounts,
            sync_accounts,
            activate_account,
            delete_account,
            get_auto_reply_settings,
            save_auto_reply_settings,
            test_auto_reply,
            manual_reply_video_comments,
            get_autostart_status,
            set_autostart,
        ])
        .setup(|app| {
            let _handle = app.handle().clone();

            // 初始化存储目录和自动回复
            tauri::async_runtime::block_on(async {
                storage::init().await;
                auto_reply::init_settings().await;
                tauri::async_runtime::spawn(async move {
                    auto_reply::start_auto_reply_service().await;
                });
            });

            // 创建系统托盘
            let show_i = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)
                .expect("创建菜单项失败");
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
                .expect("创建菜单项失败");
            let menu = Menu::with_items(app, &[&show_i, &quit_i])
                .expect("创建菜单失败");

            let img = image::load_from_memory(include_bytes!("../icons/32x32.png"))
                .expect("加载图标失败")
                .into_rgba8();
            let (width, height) = img.dimensions();
            let rgba = img.into_raw();
            let icon = tauri::image::Image::new_owned(rgba, width, height);

            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)
                .expect("创建系统托盘失败");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

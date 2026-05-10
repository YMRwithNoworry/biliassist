mod bilibili;
mod storage;
mod auto_reply;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use base64::{Engine, engine::general_purpose};

#[tauri::command]
async fn get_qr_code() -> Result<bilibili::QrCodeResponse, String> {
    bilibili::get_qr_code().await
}

#[tauri::command]
async fn generate_qr_code(data: String) -> Result<String, String> {
    let code = qrcode::QrCode::new(data).map_err(|e| format!("生成二维码失败: {}", e))?;
    let image = code.render::<image::Luma<u8>>().build();
    let mut buffer = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| format!("编码PNG失败: {}", e))?;
    Ok(general_purpose::STANDARD.encode(&buffer))
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

/// 检测是否在开发模式下运行（非正式构建）
/// 开发模式下启用自启动会导致开机时连接 localhost 失败
fn is_dev_mode() -> bool {
    if std::env::var("TAURI_ENV_TAURI_DEV").is_ok() {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let target_dir = std::path::Path::new(manifest_dir).join("target");
        if exe.starts_with(&target_dir) {
            return true;
        }
    }
    false
}

#[tauri::command]
async fn get_autostart_status(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled && is_dev_mode() {
        return Err("开发模式下无法启用开机自启，请先打包为正式版再使用此功能".into());
    }
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

/// 显示主窗口（供托盘事件调用）
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
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
                .args(["--from-autostart"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_qr_code,
            generate_qr_code,
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

            // 检测是否由开机自启启动
            let is_autostart = std::env::args().any(|a| a == "--from-autostart");
            let is_dev = is_dev_mode();
            if is_autostart {
                if is_dev {
                    log::warn!("开发模式下由开机自启启动，前端无法加载，将显示窗口并自动禁用自启");
                } else {
                    log::info!("应用由开机自启启动，将隐藏到系统托盘运行");
                }
            }

            // 初始化存储目录和自动回复
            tauri::async_runtime::block_on(async {
                storage::init().await;
                auto_reply::init_settings().await;
                tauri::async_runtime::spawn(async move {
                    auto_reply::start_auto_reply_service().await;
                });
            });

            // 清理开发模式下的无效自启动注册
            if is_autostart && is_dev {
                use tauri_plugin_autostart::ManagerExt;
                if let Err(e) = app.autolaunch().disable() {
                    log::error!("清理开发模式自启注册失败: {}", e);
                } else {
                    log::info!("已自动禁用开发模式下的开机自启");
                }
            }

            // 开机自启时先隐藏窗口，等用户点击托盘再显示
            // 开发模式下不隐藏，因为前端依赖 Vite 开发服务器，隐藏后用户无法排查问题
            if is_autostart && !is_dev {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

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
                        "show" => show_main_window(app),
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
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

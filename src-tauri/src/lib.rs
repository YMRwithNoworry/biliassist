pub mod auth;
pub mod auto_reply;
pub mod bilibili;
pub mod cloud;
mod commands;
mod platform;
pub mod storage;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            commands::show_main_window(app);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--from-autostart"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_qr_code,
            commands::generate_qr_code,
            commands::check_login_status,
            commands::get_accounts,
            commands::sync_accounts,
            commands::activate_account,
            commands::delete_account,
            commands::get_auto_reply_settings,
            commands::save_auto_reply_settings,
            commands::get_replied_set,
            commands::get_liked_set,
            commands::merge_replied_set,
            commands::merge_liked_set,
            commands::test_auto_reply,
            commands::manual_reply_video_comments,
            commands::manual_reply_dynamic_comments,
            commands::auth_restore_session,
            commands::auth_sign_in,
            commands::auth_send_otp,
            commands::auth_verify_otp,
            commands::auth_logout,
            commands::is_licensed,
            commands::activate_license,
            commands::get_autostart_status,
            commands::set_autostart,
            commands::cloud_upload_all,
            commands::cloud_download_all,
            commands::open_external_url,
            commands::copy_text_to_clipboard,
        ])
        .setup(|app| {
            let is_autostart = std::env::args().any(|arg| arg == "--from-autostart");
            let is_dev = platform::is_dev_mode();

            // 开发模式下的自启注册指向 target 目录，留着只会在开机时启动失败。
            if is_autostart && is_dev {
                use tauri_plugin_autostart::ManagerExt;
                if let Err(error) = app.autolaunch().disable() {
                    log::error!("清理开发模式自启注册失败：{error}");
                }
            }

            tauri::async_runtime::block_on(async {
                storage::init().await;
                auto_reply::init_settings().await;
            });
            tauri::async_runtime::spawn(auto_reply::start_auto_reply_service());

            // 开机自启时先藏到托盘；开发模式下前端依赖 Vite 服务，隐藏后无法排查问题。
            if is_autostart && !is_dev {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            install_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}

fn install_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let icon = image::load_from_memory(include_bytes!("../icons/32x32.png"))
        .expect("解析托盘图标失败")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let icon = tauri::image::Image::new_owned(icon.into_raw(), width, height);

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("B站账号管理工具")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => commands::show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                commands::show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

mod auto_reply;
mod bilibili;
mod storage;

use base64::{engine::general_purpose, Engine};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

#[tauri::command]
async fn get_qr_code() -> Result<bilibili::QrCodeResponse, String> {
    bilibili::get_qr_code().await
}

#[tauri::command]
async fn generate_qr_code(data: String) -> Result<String, String> {
    let code = qrcode::QrCode::new(data).map_err(|e| {
        format!(
            "\u{751f}\u{6210}\u{4e8c}\u{7ef4}\u{7801}\u{5931}\u{8d25}: {}",
            e
        )
    })?;
    let image = code.render::<image::Luma<u8>>().build();
    let mut buffer = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("\u{7f16}\u{7801}PNG\u{5931}\u{8d25}: {}", e))?;
    Ok(general_purpose::STANDARD.encode(&buffer))
}

#[tauri::command]
async fn verify_license(license_key: String) -> Result<String, String> {
    if license_key != "431paojiao" {
        return Err("\u{5bc6}\u{94a5}\u{9519}\u{8bef}".into());
    }
    Ok("ok".into())
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
async fn get_replied_set() -> Result<Vec<String>, String> {
    auto_reply::get_replied_set().await
}

#[tauri::command]
async fn get_liked_set() -> Result<Vec<String>, String> {
    auto_reply::get_liked_set().await
}

#[tauri::command]
async fn merge_replied_set(entries: Vec<String>) -> Result<(), String> {
    auto_reply::merge_replied_set(entries).await
}

#[tauri::command]
async fn merge_liked_set(entries: Vec<String>) -> Result<(), String> {
    auto_reply::merge_liked_set(entries).await
}

#[tauri::command]
async fn test_auto_reply() -> Result<String, String> {
    auto_reply::test_reply().await
}

#[tauri::command]
async fn test_ai_reply() -> Result<String, String> {
    auto_reply::test_ai_reply().await
}

#[tauri::command]
async fn manual_reply_video_comments() -> Result<String, String> {
    auto_reply::manual_reply_comments().await
}

#[tauri::command]
async fn open_external_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("仅允许打开 http/https 链接".into());
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = std::process::Command::new("rundll32");
        cmd.args(["url.dll,FileProtocolHandler", &url]);
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(&url);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(&url);
        cmd
    };

    command
        .spawn()
        .map_err(|e| format!("打开系统浏览器失败: {}", e))?;
    Ok(())
}

fn run_clipboard_command(mut command: std::process::Command, text: &str) -> Result<(), String> {
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to write clipboard: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        std::io::Write::write_all(&mut stdin, text.as_bytes())
            .map_err(|e| format!("failed to write clipboard: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to write clipboard: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "failed to write clipboard".into()
        } else {
            format!("failed to write clipboard: {}", stderr)
        })
    }
}

#[tauri::command]
async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("clipboard text cannot be empty".into());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let mut command = std::process::Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Set-Clipboard -Value ([Console]::In.ReadToEnd())",
        ]);
        command.creation_flags(0x08000000);
        return run_clipboard_command(command, &text);
    }

    #[cfg(target_os = "macos")]
    {
        return run_clipboard_command(std::process::Command::new("pbcopy"), &text);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut errors = Vec::new();

        let mut wl_copy = std::process::Command::new("wl-copy");
        match run_clipboard_command(wl_copy, &text) {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(e),
        }

        let mut xclip = std::process::Command::new("xclip");
        xclip.args(["-selection", "clipboard"]);
        match run_clipboard_command(xclip, &text) {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(e),
        }

        let mut xsel = std::process::Command::new("xsel");
        xsel.args(["--clipboard", "--input"]);
        match run_clipboard_command(xsel, &text) {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(e),
        }

        return Err(format!("failed to write clipboard: {}", errors.join("; ")));
    }
}

#[tauri::command]
async fn get_current_deep_link(app: tauri::AppHandle) -> Result<Option<String>, String> {
    app.deep_link()
        .get_current()
        .map_err(|e| e.to_string())
        .map(|urls| urls.and_then(|urls| urls.into_iter().next().map(|url| url.to_string())))
}

// ============================================================
//  \u{5f00}\u{673a}\u{81ea}\u{542f}
// ============================================================

/// \u{68c0}\u{6d4b}\u{662f}\u{5426}\u{5728}\u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{8fd0}\u{884c}\u{ff08}\u{975e}\u{6b63}\u{5f0f}\u{6784}\u{5efa}\u{ff09}
/// \u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{542f}\u{7528}\u{81ea}\u{542f}\u{52a8}\u{4f1a}\u{5bfc}\u{81f4}\u{5f00}\u{673a}\u{65f6}\u{8fde}\u{63a5} localhost \u{5931}\u{8d25}
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
        return Err("\u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{65e0}\u{6cd5}\u{542f}\u{7528}\u{5f00}\u{673a}\u{81ea}\u{542f}\u{ff0c}\u{8bf7}\u{5148}\u{6253}\u{5305}\u{4e3a}\u{6b63}\u{5f0f}\u{7248}\u{518d}\u{4f7f}\u{7528}\u{6b64}\u{529f}\u{80fd}".into());
    }
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

/// \u{663e}\u{793a}\u{4e3b}\u{7a97}\u{53e3}\u{ff08}\u{4f9b}\u{6258}\u{76d8}\u{4e8b}\u{4ef6}\u{8c03}\u{7528}\u{ff09}
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
        .plugin(tauri_plugin_deep_link::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--from-autostart"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_qr_code,
            generate_qr_code,
            verify_license,
            check_login_status,
            get_accounts,
            sync_accounts,
            activate_account,
            delete_account,
            get_auto_reply_settings,
            save_auto_reply_settings,
            get_replied_set,
            get_liked_set,
            merge_replied_set,
            merge_liked_set,
            test_auto_reply,
            test_ai_reply,
            manual_reply_video_comments,
            copy_text_to_clipboard,
            get_current_deep_link,
            open_external_url,
            get_autostart_status,
            set_autostart,
        ])
        .setup(|app| {
            let _handle = app.handle().clone();

            // 注册 deep-link 用于 GitHub OAuth 回调
            #[cfg(desktop)]
            {
                match app.deep_link().register_all() {
                    Ok(()) => log::info!("Deep-link 协议注册成功"),
                    Err(e) => log::warn!("Deep-link 协议注册失败 (可能需要管理员权限或已在运行): {}", e),
                }
            }

            // 监听 deep-link 事件，将回调 URL 传给前端
            let app_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls = event.urls();
                log::info!("Received deep-link callback: {:?}", urls);
                for url in urls {
                    let _ = app_handle.emit("oauth-callback", url.to_string());
                }
                show_main_window(&app_handle);
            });

            // \u{68c0}\u{6d4b}\u{662f}\u{5426}\u{7531}\u{5f00}\u{673a}\u{81ea}\u{542f}\u{542f}\u{52a8}
            let is_autostart = std::env::args().any(|a| a == "--from-autostart");
            let is_dev = is_dev_mode();
            if is_autostart {
                if is_dev {
                    log::warn!("\u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{7531}\u{5f00}\u{673a}\u{81ea}\u{542f}\u{542f}\u{52a8}\u{ff0c}\u{524d}\u{7aef}\u{65e0}\u{6cd5}\u{52a0}\u{8f7d}\u{ff0c}\u{5c06}\u{663e}\u{793a}\u{7a97}\u{53e3}\u{5e76}\u{81ea}\u{52a8}\u{7981}\u{7528}\u{81ea}\u{542f}");
                } else {
                    log::info!("\u{5e94}\u{7528}\u{7531}\u{5f00}\u{673a}\u{81ea}\u{542f}\u{542f}\u{52a8}\u{ff0c}\u{5c06}\u{9690}\u{85cf}\u{5230}\u{7cfb}\u{7edf}\u{6258}\u{76d8}\u{8fd0}\u{884c}");
                }
            }

            // \u{521d}\u{59cb}\u{5316}\u{5b58}\u{50a8}\u{76ee}\u{5f55}\u{548c}\u{81ea}\u{52a8}\u{56de}\u{590d}
            tauri::async_runtime::block_on(async {
                storage::init().await;
                auto_reply::init_settings().await;
                tauri::async_runtime::spawn(async move {
                    auto_reply::start_auto_reply_service().await;
                });
            });

            // \u{6e05}\u{7406}\u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{7684}\u{65e0}\u{6548}\u{81ea}\u{542f}\u{52a8}\u{6ce8}\u{518c}
            if is_autostart && is_dev {
                use tauri_plugin_autostart::ManagerExt;
                if let Err(e) = app.autolaunch().disable() {
                    log::error!("\u{6e05}\u{7406}\u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{81ea}\u{542f}\u{6ce8}\u{518c}\u{5931}\u{8d25}: {}", e);
                } else {
                    log::info!("\u{5df2}\u{81ea}\u{52a8}\u{7981}\u{7528}\u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{7684}\u{5f00}\u{673a}\u{81ea}\u{542f}");
                }
            }

            // \u{5f00}\u{673a}\u{81ea}\u{542f}\u{65f6}\u{5148}\u{9690}\u{85cf}\u{7a97}\u{53e3}\u{ff0c}\u{7b49}\u{7528}\u{6237}\u{70b9}\u{51fb}\u{6258}\u{76d8}\u{518d}\u{663e}\u{793a}
            // \u{5f00}\u{53d1}\u{6a21}\u{5f0f}\u{4e0b}\u{4e0d}\u{9690}\u{85cf}\u{ff0c}\u{56e0}\u{4e3a}\u{524d}\u{7aef}\u{4f9d}\u{8d56} Vite \u{5f00}\u{53d1}\u{670d}\u{52a1}\u{5668}\u{ff0c}\u{9690}\u{85cf}\u{540e}\u{7528}\u{6237}\u{65e0}\u{6cd5}\u{6392}\u{67e5}\u{95ee}\u{9898}
            if is_autostart && !is_dev {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // \u{521b}\u{5efa}\u{7cfb}\u{7edf}\u{6258}\u{76d8}
            let show_i = MenuItem::with_id(app, "show", "\u{663e}\u{793a}\u{7a97}\u{53e3}", true, None::<&str>)
                .expect("\u{521b}\u{5efa}\u{83dc}\u{5355}\u{9879}\u{5931}\u{8d25}");
            let quit_i = MenuItem::with_id(app, "quit", "\u{9000}\u{51fa}", true, None::<&str>)
                .expect("\u{521b}\u{5efa}\u{83dc}\u{5355}\u{9879}\u{5931}\u{8d25}");
            let menu = Menu::with_items(app, &[&show_i, &quit_i])
                .expect("\u{521b}\u{5efa}\u{83dc}\u{5355}\u{5931}\u{8d25}");

            let img = image::load_from_memory(include_bytes!("../icons/32x32.png"))
                .expect("\u{52a0}\u{8f7d}\u{56fe}\u{6807}\u{5931}\u{8d25}")
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
                .expect("\u{521b}\u{5efa}\u{7cfb}\u{7edf}\u{6258}\u{76d8}\u{5931}\u{8d25}");

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

use crate::{auth, auto_reply, bilibili, cloud, platform, storage};
use base64::{engine::general_purpose, Engine};
use tauri::Manager;

/// 托盘与单实例插件共用：把主窗口重新拉到前台。
pub fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// ============================================================
//  哔哩哔哩扫码登录
// ============================================================

#[tauri::command]
pub async fn get_qr_code() -> Result<bilibili::QrCodeResponse, String> {
    bilibili::get_qr_code().await
}

#[tauri::command]
pub async fn generate_qr_code(data: String) -> Result<String, String> {
    let code = qrcode::QrCode::new(data).map_err(|e| format!("生成二维码失败: {e}"))?;
    let image = code.render::<image::Luma<u8>>().build();
    let mut buffer = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("编码PNG失败: {e}"))?;
    Ok(general_purpose::STANDARD.encode(&buffer))
}

#[tauri::command]
pub async fn check_login_status() -> Result<bilibili::LoginStatus, String> {
    bilibili::check_login_status().await
}

// ============================================================
//  本地账号
// ============================================================

#[tauri::command]
pub async fn get_accounts() -> Result<Vec<storage::Account>, String> {
    storage::get_accounts().await
}

#[tauri::command]
pub async fn sync_accounts(
    accounts: Vec<storage::Account>,
) -> Result<Vec<storage::Account>, String> {
    storage::sync_accounts(accounts).await
}

#[tauri::command]
pub async fn activate_account(uid: String) -> Result<(), String> {
    storage::activate_account(uid).await
}

#[tauri::command]
pub async fn delete_account(uid: String) -> Result<(), String> {
    storage::delete_account(uid).await
}

// ============================================================
//  自动回复
// ============================================================

#[tauri::command]
pub async fn get_auto_reply_settings() -> Result<auto_reply::AutoReplySettings, String> {
    auto_reply::get_settings().await
}

#[tauri::command]
pub async fn save_auto_reply_settings(
    settings: auto_reply::AutoReplySettings,
) -> Result<(), String> {
    auto_reply::save_settings(settings).await
}

#[tauri::command]
pub async fn get_replied_set() -> Result<Vec<String>, String> {
    auto_reply::get_replied_set().await
}

#[tauri::command]
pub async fn get_liked_set() -> Result<Vec<String>, String> {
    auto_reply::get_liked_set().await
}

#[tauri::command]
pub async fn merge_replied_set(entries: Vec<String>) -> Result<(), String> {
    auto_reply::merge_replied_set(entries).await
}

#[tauri::command]
pub async fn merge_liked_set(entries: Vec<String>) -> Result<(), String> {
    auto_reply::merge_liked_set(entries).await
}

#[tauri::command]
pub async fn test_auto_reply() -> Result<String, String> {
    auto_reply::test_reply().await
}

#[tauri::command]
pub async fn manual_reply_video_comments() -> Result<String, String> {
    auto_reply::manual_reply_comments().await
}

#[tauri::command]
pub async fn manual_reply_dynamic_comments() -> Result<String, String> {
    auto_reply::manual_reply_dynamic_comments().await
}

// ============================================================
//  Supabase 应用账号
// ============================================================

#[tauri::command]
pub async fn auth_restore_session() -> Option<auth::AuthSession> {
    auth::restore_session().await
}

#[tauri::command]
pub async fn auth_sign_in(email: String, password: String) -> Result<auth::AuthSession, String> {
    let session = auth::sign_in_password(email, password).await?;
    auth::save_session(&session)?;
    Ok(session)
}

#[tauri::command]
pub async fn auth_send_otp(email: String) -> Result<(), String> {
    auth::send_otp(email).await
}

#[tauri::command]
pub async fn auth_verify_otp(email: String, token: String) -> Result<auth::AuthSession, String> {
    let session = auth::verify_otp(email, token).await?;
    auth::save_session(&session)?;
    Ok(session)
}

#[tauri::command]
pub async fn auth_logout() -> Result<(), String> {
    auth::clear_session()
}

// ============================================================
//  激活与系统集成
// ============================================================

#[tauri::command]
pub async fn is_licensed() -> bool {
    platform::is_licensed()
}

#[tauri::command]
pub async fn activate_license(key: String) -> Result<(), String> {
    platform::activate_license(&key)
}

#[tauri::command]
pub async fn get_autostart_status(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled && platform::is_dev_mode() {
        return Err("开发模式下无法启用开机自启，请先打包为正式版再使用此功能".into());
    }
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

// ============================================================
//  云端同步
// ============================================================

#[tauri::command]
pub async fn cloud_upload_all() -> Result<String, String> {
    let session = auth::restore_session().await.ok_or("请先登录应用账号")?;
    cloud::upload_all(session).await
}

#[tauri::command]
pub async fn cloud_download_all() -> Result<cloud::CloudDownload, String> {
    let session = auth::restore_session().await.ok_or("请先登录应用账号")?;
    cloud::download_all(session).await
}

// ============================================================
//  系统能力
// ============================================================

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
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
        .map_err(|e| format!("打开系统浏览器失败: {e}"))?;
    Ok(())
}

fn run_clipboard_command(mut command: std::process::Command, text: &str) -> Result<(), String> {
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("写入剪贴板失败: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        std::io::Write::write_all(&mut stdin, text.as_bytes())
            .map_err(|e| format!("写入剪贴板失败: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("写入剪贴板失败: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "写入剪贴板失败".into()
        } else {
            format!("写入剪贴板失败: {stderr}")
        })
    }
}

#[tauri::command]
pub async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("剪贴板内容不能为空".into());
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

        Err(format!("写入剪贴板失败: {}", errors.join("; ")))
    }
}

use std::path::PathBuf;

const LICENSE_FILE: &str = "license_activated";

fn data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bilibili_account_manager")
}

pub fn is_licensed() -> bool {
    data_dir().join(LICENSE_FILE).exists()
}

pub fn activate_license(key: &str) -> Result<(), String> {
    if key.trim() != "431paojiao" {
        return Err("激活码错误".into());
    }
    std::fs::create_dir_all(data_dir()).map_err(|error| error.to_string())?;
    std::fs::write(data_dir().join(LICENSE_FILE), b"activated").map_err(|error| error.to_string())
}

/// 开发模式：debug 构建（cargo run / tauri dev）注册的自启指向 target 目录里随时会被重建的可执行文件，
/// 开机必然启动失败，所以此时不允许开启。release 构建一律视为正式版，本地 target/release 也允许。
pub fn is_dev_mode() -> bool {
    cfg!(debug_assertions) || std::env::var("TAURI_ENV_TAURI_DEV").is_ok()
}

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

/// 开发模式下注册的开机自启会指向 target 目录里的临时可执行文件，开机必然启动失败。
pub fn is_dev_mode() -> bool {
    if std::env::var("TAURI_ENV_TAURI_DEV").is_ok() {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        if exe.starts_with(&target_dir) {
            return true;
        }
    }
    false
}

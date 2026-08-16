use auto_launch::AutoLaunchBuilder;
use std::path::PathBuf;

const APP_NAME: &str = "BilibiliAccountManager";
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

fn auto_launch() -> Result<auto_launch::AutoLaunch, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(executable.to_string_lossy().as_ref())
        .set_args(&["--from-autostart"])
        .build()
        .map_err(|error| error.to_string())
}

pub fn is_autostart_enabled() -> Result<bool, String> {
    auto_launch()?
        .is_enabled()
        .map_err(|error| error.to_string())
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let launcher = auto_launch()?;
    if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|error| error.to_string())
}

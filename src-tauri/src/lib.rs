mod bilibili;
mod storage;
mod auto_reply;

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

pub fn run() {
    // 配置日志输出到控制台
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
            manual_reply_video_comments
        ])
        .setup(|app| {
            // 初始化存储目录
            let _handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                storage::init().await;
                // 加载自动回复设置（包含历史记录）
                auto_reply::init_settings().await;
                // 启动自动回复后台服务
                tauri::async_runtime::spawn(async move {
                    auto_reply::start_auto_reply_service().await;
                });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

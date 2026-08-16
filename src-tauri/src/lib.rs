pub mod auto_reply;
pub mod bilibili;
pub mod storage;
mod ui;

use std::sync::OnceLock;
use tokio::runtime::Runtime;

pub(crate) fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("无法创建 Tokio 运行时"))
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    runtime().block_on(async {
        storage::init().await;
        auto_reply::init_settings().await;
    });
    runtime().spawn(auto_reply::start_auto_reply_service());

    ui::run();
}

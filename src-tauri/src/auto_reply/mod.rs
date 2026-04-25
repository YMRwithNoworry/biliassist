pub mod models;
pub mod state;
pub mod wbi;
pub mod http;
pub mod handler;
pub mod comment;
pub mod direct_message;
pub mod follow;

pub use models::{AutoReplySettings, MsgSource};
pub use state::get_global_state;

use handler::HandlerRegistry;

/// 自动回复服务
pub struct AutoReplyService {
    registry: HandlerRegistry,
}

impl AutoReplyService {
    pub fn new() -> Self {
        let mut registry = HandlerRegistry::new();
        registry.register(Box::new(comment::CommentHandler::new()));
        registry.register(Box::new(direct_message::DirectMessageHandler::new()));
        registry.register(Box::new(follow::FollowHandler::new()));
        Self { registry }
    }

    pub async fn start(&self) {
        log::info!("自动回复服务启动");

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let state = get_global_state();
            let settings = state.get_settings().await;

            if !settings.enabled {
                continue;
            }

            for source in &settings.sources {
                if let Some(handler) = self.registry.get_handler(source) {
                    let account = match crate::storage::get_active_account().await {
                        Some(acc) => acc,
                        None => {
                            log::warn!("没有激活的账号");
                            continue;
                        }
                    };

                    let has_sessdata = account.cookie.contains("SESSDATA=");
                    let has_bili_jct = account.cookie.contains("bili_jct=");
                    let has_dede = account.cookie.contains("DedeUserID=");
                    log::info!(
                        "账号 cookie 诊断: len={}, SESSDATA={}, bili_jct={}, DedeUserID={}",
                        account.cookie.len(), has_sessdata, has_bili_jct, has_dede
                    );

                    if !has_sessdata || !has_bili_jct {
                        log::error!("cookie 不完整（缺少 SESSDATA 或 bili_jct），请删除账号重新扫码登录");
                        continue;
                    }

                    match handler.handle(&account, state).await {
                        Ok(result) => {
                            if result.success_count > 0 || result.error_count > 0 {
                                log::info!(
                                    "{} 处理完成: 成功={}, 失败={}",
                                    handler.name(),
                                    result.success_count,
                                    result.error_count
                                );
                            }
                            if result.stopped_by_rate_limit {
                                log::warn!("{} 触发风控限制，停止处理", handler.name());
                            }
                        }
                        Err(e) => {
                            log::error!("{} 处理失败: {}", handler.name(), e);
                        }
                    }
                }
            }

            let interval = {
                let s = state.get_settings().await;
                s.interval
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    }

    pub async fn manual_trigger(&self, source: Option<MsgSource>) -> Result<String, String> {
        let state = get_global_state();
        let settings = state.get_settings().await;
        let account = crate::storage::get_active_account()
            .await
            .ok_or("没有激活的账号")?;

        let sources = if let Some(s) = source {
            vec![s]
        } else {
            settings.sources.clone()
        };

        let mut results = Vec::new();

        for source in sources {
            if let Some(handler) = self.registry.get_handler(&source) {
                match handler.handle(&account, state).await {
                    Ok(result) => {
                        results.push(format!(
                            "{}: 成功{}条, 失败{}条",
                            handler.name(),
                            result.success_count,
                            result.error_count
                        ));
                    }
                    Err(e) => {
                        results.push(format!("{}: 失败 - {}", handler.name(), e));
                    }
                }
            }
        }

        Ok(results.join("\n"))
    }
}

impl Default for AutoReplyService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
//  向后兼容的公开 API 函数
// ============================================================

pub async fn init_settings() {
    state::init_global_state().await;
}

pub async fn get_settings() -> Result<AutoReplySettings, String> {
    let state = get_global_state();
    Ok(state.get_settings().await)
}

pub async fn save_settings(new_settings: AutoReplySettings) -> Result<(), String> {
    let state = get_global_state();
    state.update_settings(|s| *s = new_settings).await?;
    Ok(())
}

pub async fn test_reply() -> Result<String, String> {
    let state = get_global_state();
    let settings = state.get_settings().await;
    let formatted = handler::format_message(&settings.message, "测试用户");
    Ok(format!("测试回复内容:\n{}", formatted))
}

pub async fn manual_reply_comments() -> Result<String, String> {
    let service = AutoReplyService::new();
    service.manual_trigger(Some(MsgSource::Comment)).await
}

pub async fn start_auto_reply_service() {
    let service = AutoReplyService::new();
    service.start().await;
}

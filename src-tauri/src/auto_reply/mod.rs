pub mod models;
pub mod state;
pub mod wbi;
pub mod http;
pub mod handler;
pub mod comment;
pub mod direct_message;
pub mod follow;
pub mod ai;

pub use models::{AutoReplySettings, MsgSource, AiReplyConfig};
pub use state::get_global_state;

use handler::HandlerRegistry;

/// \u{81ea}\u{52a8}\u{56de}\u{590d}\u{670d}\u{52a1}
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
        log::info!("\u{81ea}\u{52a8}\u{56de}\u{590d}\u{670d}\u{52a1}\u{542f}\u{52a8}");

        loop {
            let state = get_global_state();
            let settings = state.get_settings().await;

            if !settings.enabled {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }

            for source in &settings.sources {
                if let Some(handler) = self.registry.get_handler(source) {
                    let account = match crate::storage::get_active_account().await {
                        Some(acc) => acc,
                        None => {
                            log::warn!("\u{6ca1}\u{6709}\u{6fc0}\u{6d3b}\u{7684}\u{8d26}\u{53f7}");
                            continue;
                        }
                    };

                    let has_sessdata = account.cookie.contains("SESSDATA=");
                    let has_bili_jct = account.cookie.contains("bili_jct=");
                    let has_dede = account.cookie.contains("DedeUserID=");
                    log::info!(
                        "\u{8d26}\u{53f7} cookie \u{8bca}\u{65ad}: len={}, SESSDATA={}, bili_jct={}, DedeUserID={}",
                        account.cookie.len(), has_sessdata, has_bili_jct, has_dede
                    );

                    if !has_sessdata || !has_bili_jct {
                        log::error!("cookie \u{4e0d}\u{5b8c}\u{6574}\u{ff08}\u{7f3a}\u{5c11} SESSDATA \u{6216} bili_jct\u{ff09}\u{ff0c}\u{8bf7}\u{5220}\u{9664}\u{8d26}\u{53f7}\u{91cd}\u{65b0}\u{626b}\u{7801}\u{767b}\u{5f55}");
                        continue;
                    }

                    match handler.handle(&account, state).await {
                        Ok(result) => {
                            if result.success_count > 0 || result.error_count > 0 {
                                log::info!(
                                    "{} \u{5904}\u{7406}\u{5b8c}\u{6210}: \u{6210}\u{529f}={}, \u{5931}\u{8d25}={}",
                                    handler.name(),
                                    result.success_count,
                                    result.error_count
                                );
                            }
                            if result.stopped_by_rate_limit {
                                log::warn!("{} \u{89e6}\u{53d1}\u{98ce}\u{63a7}\u{9650}\u{5236}\u{ff0c}\u{505c}\u{6b62}\u{5904}\u{7406}", handler.name());
                            }
                        }
                        Err(e) => {
                            log::error!("{} \u{5904}\u{7406}\u{5931}\u{8d25}: {}", handler.name(), e);
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
            .ok_or("\u{6ca1}\u{6709}\u{6fc0}\u{6d3b}\u{7684}\u{8d26}\u{53f7}")?;

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
                            "{}: \u{6210}\u{529f}{}\u{6761} \u{5931}\u{8d25}{}\u{6761}",
                            handler.name(),
                            result.success_count,
                            result.error_count
                        ));
                    }
                    Err(e) => {
                        results.push(format!("{}: \u{5931}\u{8d25} - {}", handler.name(), e));
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
//  \u{5411}\u{540e}\u{517c}\u{5bb9}\u{7684}\u{516c}\u{5f00} API \u{51fd}\u{6570}
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
    let formatted = handler::format_message(&settings.message, "\u{6d4b}\u{8bd5}\u{7528}\u{6237}");
    Ok(format!("\u{6d4b}\u{8bd5}\u{56de}\u{590d}\u{5185}\u{5bb9}:\n{}", formatted))
}

pub async fn test_ai_reply() -> Result<String, String> {
    let state = get_global_state();
    let settings = state.get_settings().await;
    ai::test_ai_config(&settings.ai).await
}

pub async fn manual_reply_comments() -> Result<String, String> {
    let service = AutoReplyService::new();
    service.manual_trigger(Some(MsgSource::Comment)).await
}

pub async fn start_auto_reply_service() {
    let service = AutoReplyService::new();
    service.start().await;
}

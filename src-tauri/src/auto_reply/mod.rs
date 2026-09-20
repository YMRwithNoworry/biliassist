pub mod comment;
pub mod direct_message;
pub mod follow;
pub mod handler;
pub mod http;
pub mod models;
pub mod state;
pub mod wbi;

pub use models::{AutoReplySettings, MsgSource};
pub use state::get_global_state;
use state::AutoReplyState;

use handler::{HandlerRegistry, ScanMode};
use tokio::time::{Duration, Instant};

const DISABLED_POLL_INTERVAL_SECS: u64 = 5;
const MIN_POLL_GAP: Duration = Duration::from_millis(250);

fn schedule_next_poll(started_at: Instant, interval_secs: u64, now: Instant) -> Instant {
    let configured = Duration::from_secs(interval_secs.max(1));
    let deadline = started_at + configured;
    let earliest = now + MIN_POLL_GAP;
    deadline.max(earliest)
}

/// \u{81ea}\u{52a8}\u{56de}\u{590d}\u{670d}\u{52a1}
pub struct AutoReplyService {
    registry: HandlerRegistry,
}

impl AutoReplyService {
    pub fn new() -> Self {
        let mut registry = HandlerRegistry::new();
        registry.register(Box::new(comment::CommentHandler::new()));
        registry.register(Box::new(comment::CommentHandler::dynamic()));
        registry.register(Box::new(direct_message::DirectMessageHandler::new()));
        registry.register(Box::new(follow::FollowHandler::new()));
        Self { registry }
    }

    pub async fn start(&self) {
        log::info!("自动回复服务启动");
        // 快速通道：只抓每个评论目标的最新一页，让新评论在几秒内得到回复。
        // 完整通道：仍按用户设置的间隔补扫历史评论，并处理私信、关注等渠道。
        let mut next_fast = Instant::now();
        let mut next_full = Instant::now();

        loop {
            tokio::time::sleep_until(next_fast.min(next_full)).await;
            let tick_started_at = Instant::now();
            let state = get_global_state();
            let settings = state.get_settings().await;

            let fast_due = tick_started_at >= next_fast;
            let full_due = tick_started_at >= next_full;

            let replies_enabled = settings.enabled && settings.any_enabled();
            let likes_enabled = MsgSource::COMMENT_SOURCES
                .iter()
                .any(|source| settings.likes_enabled_for_source(*source));
            if !replies_enabled && !likes_enabled {
                let idle = Instant::now() + Duration::from_secs(DISABLED_POLL_INTERVAL_SECS);
                next_fast = idle;
                next_full = idle;
                continue;
            }

            // 先推进时间表：即使本轮因账号不可用而空转，快速通道也会继续按节奏重试。
            if fast_due {
                next_fast = schedule_next_poll(
                    tick_started_at,
                    settings.fast_interval_secs(),
                    Instant::now(),
                );
            }
            if full_due {
                next_full = schedule_next_poll(tick_started_at, settings.interval, Instant::now());
            }

            let account = match crate::storage::get_active_account().await {
                Some(acc) => acc,
                None => {
                    // 快速通道每几秒都会重试，只有完整通道才需要提醒。
                    if full_due {
                        log::warn!("没有激活的账号");
                    }
                    continue;
                }
            };

            let has_sessdata = account.cookie.contains("SESSDATA=");
            let has_bili_jct = account.cookie.contains("bili_jct=");
            let has_dede = account.cookie.contains("DedeUserID=");
            if full_due {
                log::info!(
                    "账号 cookie 诊断: len={}, SESSDATA={}, bili_jct={}, DedeUserID={}",
                    account.cookie.len(),
                    has_sessdata,
                    has_bili_jct,
                    has_dede
                );
            }

            if !has_sessdata || !has_bili_jct {
                if full_due {
                    log::error!(
                        "cookie 不完整（缺少 SESSDATA 或 bili_jct），请删除账号重新扫码登录"
                    );
                }
                continue;
            }

            if fast_due {
                self.run_pass(&account, state, &settings, ScanMode::Newest)
                    .await;
            }
            if full_due {
                self.run_pass(&account, state, &settings, ScanMode::Full)
                    .await;
            }
        }
    }

    /// 执行一轮抓取并回复。
    ///
    /// `ScanMode::Newest` 只抓每个目标的最新一页（快速通道），
    /// `ScanMode::Full` 会按上限补扫历史评论（完整通道）。
    async fn run_pass(
        &self,
        account: &crate::bilibili::UserInfo,
        state: &AutoReplyState,
        settings: &AutoReplySettings,
        mode: ScanMode,
    ) {
        if settings.enabled {
            for source in settings.enabled_sources() {
                if !mode.includes(source) {
                    continue;
                }
                let Some(handler) = self.registry.get_handler(&source) else {
                    continue;
                };
                match handler.handle_in(account, state, mode).await {
                    Ok(result) => {
                        if result.success_count > 0
                            || result.error_count > 0
                            || result.like_success_count > 0
                            || result.like_error_count > 0
                        {
                            log::info!(
                                "{}({}) 处理完成: 成功={}, 失败={}, 点赞成功={}, 点赞失败={}",
                                handler.name(),
                                mode.label(),
                                result.success_count,
                                result.error_count,
                                result.like_success_count,
                                result.like_error_count
                            );
                        }
                        if result.stopped_by_rate_limit {
                            log::warn!("{}触发风控限制，停止处理", handler.name());
                        }
                    }
                    Err(e) => log::error!("{}处理失败: {}", handler.name(), e),
                }
            }
        }

        for source in MsgSource::COMMENT_SOURCES {
            if !mode.includes(source) {
                continue;
            }
            let reply_enabled = settings
                .comment_settings(source)
                .map(|channel| channel.reply.enabled)
                .unwrap_or(false);
            if settings.likes_enabled_for_source(source)
                && (!settings.enabled || !reply_enabled)
                && !(settings.enabled
                    && source == MsgSource::Comment
                    && settings.has_enabled_tracked_videos())
            {
                let Some(handler) = self.registry.get_handler(&source) else {
                    continue;
                };
                match handler.handle_likes_only_in(account, state, mode).await {
                    Ok(result) => {
                        if result.like_success_count > 0 || result.like_error_count > 0 {
                            log::info!(
                                "{}({}) 点赞处理完成: 点赞成功={}, 点赞失败={}",
                                handler.name(),
                                mode.label(),
                                result.like_success_count,
                                result.like_error_count
                            );
                        }
                        if result.stopped_by_rate_limit {
                            log::warn!("{}点赞触发风控限制，停止处理", handler.name());
                        }
                    }
                    Err(e) => log::error!("{}点赞处理失败: {}", handler.name(), e),
                }
            }
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
            settings.enabled_sources()
        };

        let mut results = Vec::new();

        for source in sources {
            if let Some(handler) = self.registry.get_handler(&source) {
                match handler.handle(&account, state).await {
                    Ok(result) => {
                        results.push(format!(
                            "{}: \u{6210}\u{529f}{}\u{6761} \u{5931}\u{8d25}{}\u{6761} \u{70b9}\u{8d5e}\u{6210}\u{529f}{}\u{6761} \u{70b9}\u{8d5e}\u{5931}\u{8d25}{}\u{6761}",
                            handler.name(),
                            result.success_count,
                            result.error_count,
                            result.like_success_count,
                            result.like_error_count
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
    let previews = MsgSource::ALL.map(|source| {
        let formatted = handler::format_message(
            &settings.channel(source).message,
            "\u{6d4b}\u{8bd5}\u{7528}\u{6237}",
        );
        format!("{}: {}", source.display_name(), formatted)
    });
    Ok(format!(
        "\u{6d4b}\u{8bd5}\u{56de}\u{590d}\u{5185}\u{5bb9}:\n{}",
        previews.join("\n")
    ))
}

pub async fn manual_reply_comments() -> Result<String, String> {
    let service = AutoReplyService::new();
    service.manual_trigger(Some(MsgSource::Comment)).await
}

pub async fn manual_reply_dynamic_comments() -> Result<String, String> {
    let service = AutoReplyService::new();
    service.manual_trigger(Some(MsgSource::Dynamic)).await
}

pub async fn start_auto_reply_service() {
    let service = AutoReplyService::new();
    service.start().await;
}

// ============================================================
//  云同步导出/导入 API
// ============================================================

pub async fn get_replied_set() -> Result<Vec<String>, String> {
    let state = get_global_state();
    Ok(state.get_replied_set_snapshot().await)
}

pub async fn get_liked_set() -> Result<Vec<String>, String> {
    let state = get_global_state();
    Ok(state.get_liked_set_snapshot().await)
}

pub async fn merge_replied_set(entries: Vec<String>) -> Result<(), String> {
    let state = get_global_state();
    state.merge_replied_set(entries).await;
    Ok(())
}

pub async fn merge_liked_set(entries: Vec<String>) -> Result<(), String> {
    let state = get_global_state();
    state.merge_liked_set(entries).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_interval_is_measured_from_cycle_start() {
        let base = Instant::now();
        let next = schedule_next_poll(base, 5, base);

        assert_eq!(next, base + Duration::from_secs(5));
    }

    #[test]
    fn slow_cycles_wait_only_for_the_minimum_gap() {
        let started = Instant::now();
        let now = started + Duration::from_secs(10);
        let next = schedule_next_poll(started, 5, now);

        assert_eq!(next, now + MIN_POLL_GAP);
    }

    #[test]
    fn zero_interval_is_clamped_to_one_second() {
        let base = Instant::now();
        let next = schedule_next_poll(base, 0, base);

        assert_eq!(next, base + Duration::from_secs(1));
    }
}

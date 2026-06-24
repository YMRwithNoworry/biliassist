use super::models::MsgSource;
use super::state::AutoReplyState;
use crate::bilibili::UserInfo;
use async_trait::async_trait;

/// \u{6d88}\u{606f}\u{7ed3}\u{6784}
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    #[allow(dead_code)]
    pub content: Option<String>,
    pub extra_data: serde_json::Value,
}

/// \u{5904}\u{7406}\u{7ed3}\u{679c}
#[derive(Debug, Default)]
pub struct HandleResult {
    pub success_count: u32,
    pub error_count: u32,
    pub like_success_count: u32,
    pub like_error_count: u32,
    pub stopped_by_rate_limit: bool,
}

/// \u{6d88}\u{606f}\u{5904}\u{7406}\u{5668} trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    fn name(&self) -> &'static str;

    fn source_type(&self) -> MsgSource;

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String>;

    async fn send_reply(
        &self,
        account: &UserInfo,
        message: &Message,
        reply_msg: &str,
    ) -> Result<(), String>;

    async fn on_reply_success(
        &self,
        _account: &UserInfo,
        _message: &Message,
        _state: &AutoReplyState,
    ) -> Result<(), String> {
        Ok(())
    }

    fn needs_history_fallback(&self) -> bool {
        matches!(
            self.source_type(),
            MsgSource::DirectMessage | MsgSource::Follow
        )
    }

    async fn like_comment_if_needed(
        &self,
        account: &UserInfo,
        message: &Message,
        state: &AutoReplyState,
        result: &mut HandleResult,
    ) {
        let settings = state.get_settings().await;
        if !settings.like_comments || self.source_type() != MsgSource::Comment {
            return;
        }

        let like_key = format!(
            "like:reply_action:{}:{}",
            self.source_type().id(),
            message.id
        );
        if state.is_liked(&like_key).await {
            return;
        }

        match self.on_reply_success(account, message, state).await {
            Ok(_) => {
                state.mark_liked(like_key).await;
                result.like_success_count += 1;
            }
            Err(e) => {
                log::warn!("{}自动点赞失败: {}", self.name(), e);
                result.like_error_count += 1;
                if is_rate_limit_error(&e) {
                    result.stopped_by_rate_limit = true;
                }
            }
        }
    }

    async fn handle(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
    ) -> Result<HandleResult, String> {
        let settings = state.get_settings().await;
        let messages = self.fetch_messages(account).await?;

        let mut result = HandleResult::default();

        for message in messages {
            let dedup_key = format!("{}:{}", self.source_type().id(), message.id);

            let already_replied_on_bilibili = message.extra_data["already_replied"]
                .as_bool()
                .unwrap_or(false);

            if already_replied_on_bilibili
                || (settings.reply_only_once && state.is_replied(&dedup_key).await)
            {
                if settings.reply_only_once {
                    state.mark_replied(dedup_key).await;
                }
                self.like_comment_if_needed(account, &message, state, &mut result)
                    .await;
                if result.stopped_by_rate_limit {
                    break;
                }
                continue;
            }

            // \u{5386}\u{53f2}\u{8bb0}\u{5f55}\u{56de}\u{67e5}\u{ff08}\u{79c1}\u{4fe1}/\u{5173}\u{6ce8}\u{964d}\u{7ea7}\u{4fdd}\u{969c}\u{ff09}
            if settings.reply_only_once
                && self.needs_history_fallback()
                && state
                    .is_replied_in_history(&message.user_id, &self.source_type())
                    .await
            {
                log::info!("\u{5386}\u{53f2}\u{8bb0}\u{5f55}\u{56de}\u{67e5}\u{547d}\u{4e2d}\u{ff0c}\u{8df3}\u{8fc7}\u{5df2}\u{56de}\u{590d}\u{7528}\u{6237}: {}", message.user_id);
                // \u{540c}\u{6b65}\u{5230} replied_set \u{907f}\u{514d}\u{4e0b}\u{6b21}\u{518d}\u{67e5}\u{5386}\u{53f2}
                state.mark_replied(dedup_key).await;
                continue;
            }

            // \u{751f}\u{6210}\u{56de}\u{590d}\u{5185}\u{5bb9}\u{ff1a}\u{4f18}\u{5148}\u{4f7f}\u{7528} AI\u{ff0c}\u{5426}\u{5219}\u{4f7f}\u{7528}\u{6a21}\u{677f}
            let reply_text = if settings.ai.enabled {
                let source_name = self.source_type().display_name();
                let msg_content = message.content.as_deref().unwrap_or("");
                match super::ai::generate_reply(
                    &settings.ai,
                    &message.user_name,
                    msg_content,
                    source_name,
                )
                .await
                {
                    Ok(ai_reply) => {
                        log::info!("AI \u{751f}\u{6210}\u{56de}\u{590d}\u{6210}\u{529f}: user={}, reply={}", message.user_name, ai_reply);
                        ai_reply
                    }
                    Err(e) => {
                        log::warn!("AI \u{751f}\u{6210}\u{56de}\u{590d}\u{5931}\u{8d25}\u{ff0c}\u{56de}\u{9000}\u{5230}\u{6a21}\u{677f}: {}", e);
                        format_message(&settings.message, &message.user_name)
                    }
                }
            } else {
                format_message(&settings.message, &message.user_name)
            };

            match self.send_reply(account, &message, &reply_text).await {
                Ok(_) => {
                    if settings.reply_only_once {
                        state.mark_replied(dedup_key).await;
                    }
                    state
                        .add_history(message.user_name.clone(), reply_text, self.source_type())
                        .await;
                    result.success_count += 1;

                    self.like_comment_if_needed(account, &message, state, &mut result)
                        .await;
                    if result.stopped_by_rate_limit {
                        break;
                    }
                }
                Err(e) => {
                    log::error!("{}\u{56de}\u{590d}\u{5931}\u{8d25}: {}", self.name(), e);
                    result.error_count += 1;

                    if is_rate_limit_error(&e) {
                        result.stopped_by_rate_limit = true;
                        break;
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }

        Ok(result)
    }

    async fn handle_likes_only(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
    ) -> Result<HandleResult, String> {
        let settings = state.get_settings().await;
        if !settings.like_comments || self.source_type() != MsgSource::Comment {
            return Ok(HandleResult::default());
        }

        let messages = self.fetch_messages(account).await?;
        let mut result = HandleResult::default();

        for message in messages {
            self.like_comment_if_needed(account, &message, state, &mut result)
                .await;
            if result.stopped_by_rate_limit {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }

        Ok(result)
    }
}

/// \u{683c}\u{5f0f}\u{5316}\u{6d88}\u{606f}
pub fn format_message(template: &str, username: &str) -> String {
    use chrono::{FixedOffset, TimeZone};
    let beijing_now = FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .from_utc_datetime(&chrono::Utc::now().naive_utc());
    template
        .replace("{\u{7528}\u{6237}\u{540d}}", username)
        .replace(
            "{\u{65f6}\u{95f4}}",
            &beijing_now.format("%Y-%m-%d %H:%M:%S").to_string(),
        )
}

/// \u{5224}\u{65ad}\u{662f}\u{5426}\u{4e3a}\u{98ce}\u{63a7}\u{9519}\u{8bef}
pub fn is_rate_limit_error(error: &str) -> bool {
    error.contains("banned") || error.contains("\u{9891}\u{7e41}")
}

/// \u{751f}\u{6210}\u{8bbe}\u{5907}ID (dev_id)
pub fn generate_dev_id() -> String {
    let mut result = String::with_capacity(36);
    let mut rng = rand::thread_rng();
    use rand::Rng;
    let template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx";
    for c in template.chars() {
        match c {
            'x' => {
                let r: u8 = rng.gen_range(0..16);
                result.push_str(&format!("{:X}", r));
            }
            'y' => {
                let r: u8 = rng.gen_range(0..16);
                result.push_str(&format!("{:X}", 3 & r | 8));
            }
            _ => result.push(c),
        }
    }
    result
}

/// \u{6d88}\u{606f}\u{5904}\u{7406}\u{5668}\u{6ce8}\u{518c}\u{8868}
pub struct HandlerRegistry {
    handlers: Vec<Box<dyn MessageHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Box<dyn MessageHandler>) {
        self.handlers.push(handler);
    }

    pub fn get_handler(&self, source: &MsgSource) -> Option<&dyn MessageHandler> {
        self.handlers
            .iter()
            .find(|h| h.source_type() == *source)
            .map(|h| h.as_ref())
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_reply::state::AutoReplyState;
    use crate::bilibili::UserInfo;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct MockCommentHandler {
        liked: Arc<AtomicUsize>,
        messages: Vec<Message>,
    }

    #[async_trait]
    impl MessageHandler for MockCommentHandler {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn source_type(&self) -> MsgSource {
            MsgSource::Comment
        }

        async fn fetch_messages(&self, _account: &UserInfo) -> Result<Vec<Message>, String> {
            Ok(self.messages.clone())
        }

        async fn send_reply(
            &self,
            _account: &UserInfo,
            _message: &Message,
            _reply_msg: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn on_reply_success(
            &self,
            _account: &UserInfo,
            _message: &Message,
            _state: &AutoReplyState,
        ) -> Result<(), String> {
            self.liked.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bilibili-account-manager-{name}-{nanos}"))
    }

    fn test_account() -> UserInfo {
        UserInfo {
            uid: "1".to_string(),
            name: "tester".to_string(),
            avatar: String::new(),
            cookie: "SESSDATA=x; bili_jct=y; DedeUserID=1".to_string(),
        }
    }

    fn comment_message(id: &str, already_replied: bool) -> Message {
        Message {
            id: id.to_string(),
            user_id: "2".to_string(),
            user_name: "commenter".to_string(),
            content: Some("hello".to_string()),
            extra_data: serde_json::json!({
                "already_replied": already_replied,
                "aid": 1,
                "rpid": 2,
            }),
        }
    }

    #[tokio::test]
    async fn likes_new_comments_when_only_like_comments_is_enabled() {
        let data_dir = temp_data_dir("likes-new-comments");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.enabled = false;
                settings.like_comments = true;
                settings.sources = Vec::new();
            })
            .await
            .unwrap();

        let liked = Arc::new(AtomicUsize::new(0));
        let handler = MockCommentHandler {
            liked: Arc::clone(&liked),
            messages: vec![comment_message("1:2", false)],
        };

        let result = handler
            .handle_likes_only(&test_account(), &state)
            .await
            .unwrap();

        let _ = std::fs::remove_dir_all(data_dir);

        assert_eq!(1, result.like_success_count);
        assert_eq!(1, liked.load(Ordering::SeqCst));
    }
}

use super::models::{MsgSource, ReplyPolicy};
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

    async fn like_comment_if_needed(
        &self,
        account: &UserInfo,
        message: &Message,
        state: &AutoReplyState,
        result: &mut HandleResult,
    ) {
        let settings = state.get_settings().await;
        if !settings.channels.comment.like_comments || self.source_type() != MsgSource::Comment {
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
        let source = self.source_type();
        let channel = settings.channel(source).clone();
        let messages = self.fetch_messages(account).await?;

        let mut result = HandleResult::default();
        let message_count = messages.len();

        for (index, message) in messages.into_iter().enumerate() {
            let event_key = format!("event:{}:{}:{}", account.uid, source.id(), message.id);
            let user_key = format!("user:{}:{}:{}", account.uid, source.id(), message.user_id);

            let already_replied_on_bilibili = message.extra_data["already_replied"]
                .as_bool()
                .unwrap_or(false);
            let legacy_event_key = match source {
                MsgSource::Comment => Some(format!("{}:{}", source.id(), message.id)),
                MsgSource::DirectMessage | MsgSource::Follow => None,
            };
            let already_processed = state.is_replied(&event_key).await
                || match legacy_event_key.as_deref() {
                    Some(key) => state.is_replied(key).await,
                    None => false,
                };

            if already_replied_on_bilibili || already_processed {
                if already_replied_on_bilibili && !already_processed {
                    let mut keys = vec![event_key.clone()];
                    if channel.reply_policy == ReplyPolicy::OncePerUser {
                        keys.push(user_key.clone());
                    }
                    state.mark_replied_many(keys).await;
                }
                self.like_comment_if_needed(account, &message, state, &mut result)
                    .await;
                if result.stopped_by_rate_limit {
                    break;
                }
                continue;
            }

            if channel.reply_policy == ReplyPolicy::OncePerUser {
                let legacy_user_key = match source {
                    MsgSource::DirectMessage | MsgSource::Follow => {
                        Some(format!("{}:{}", source.id(), message.user_id))
                    }
                    MsgSource::Comment => None,
                };
                let replied_by_key = state.is_replied(&user_key).await
                    || match legacy_user_key.as_deref() {
                        Some(key) => state.is_replied(key).await,
                        None => false,
                    };
                let replied_by_history = source != MsgSource::Comment
                    && state
                        .is_replied_in_history(&message.user_id, &message.user_name, &source)
                        .await;

                if replied_by_key || replied_by_history {
                    log::info!(
                        "已回复用户回查命中，跳过: source={}, user={}",
                        source.id(),
                        message.user_id
                    );
                    state
                        .mark_replied_many(vec![event_key.clone(), user_key.clone()])
                        .await;
                    continue;
                }
            }

            // \u{751f}\u{6210}\u{56de}\u{590d}\u{5185}\u{5bb9}\u{ff1a}\u{4f18}\u{5148}\u{4f7f}\u{7528} AI\u{ff0c}\u{5426}\u{5219}\u{4f7f}\u{7528}\u{6a21}\u{677f}
            let ai_config = settings.resolved_ai(source);
            let reply_text = if ai_config.enabled {
                let source_name = source.display_name();
                let msg_content = message.content.as_deref().unwrap_or("");
                match super::ai::generate_reply(
                    &ai_config,
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
                        format_message(&channel.message, &message.user_name)
                    }
                }
            } else {
                format_message(&channel.message, &message.user_name)
            };

            match self.send_reply(account, &message, &reply_text).await {
                Ok(_) => {
                    let mut keys = vec![event_key];
                    if channel.reply_policy == ReplyPolicy::OncePerUser {
                        keys.push(user_key);
                    }
                    state.mark_replied_many(keys).await;
                    state
                        .add_history(message.user_name.clone(), reply_text, source)
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

            if index + 1 < message_count {
                processing_delay().await;
            }
        }

        Ok(result)
    }

    async fn handle_likes_only(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
    ) -> Result<HandleResult, String> {
        let settings = state.get_settings().await;
        if !settings.channels.comment.like_comments || self.source_type() != MsgSource::Comment {
            return Ok(HandleResult::default());
        }

        let messages = self.fetch_messages(account).await?;
        let mut result = HandleResult::default();
        let message_count = messages.len();

        for (index, message) in messages.into_iter().enumerate() {
            self.like_comment_if_needed(account, &message, state, &mut result)
                .await;
            if result.stopped_by_rate_limit {
                break;
            }
            if index + 1 < message_count {
                processing_delay().await;
            }
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

async fn processing_delay() {
    #[cfg(not(test))]
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

    struct MockDirectMessageHandler {
        sent: Arc<AtomicUsize>,
        messages: Vec<Message>,
        should_fail: bool,
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

    #[async_trait]
    impl MessageHandler for MockDirectMessageHandler {
        fn name(&self) -> &'static str {
            "mock direct message"
        }

        fn source_type(&self) -> MsgSource {
            MsgSource::DirectMessage
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
            self.sent.fetch_add(1, Ordering::SeqCst);
            if self.should_fail {
                Err("mock send failure".to_string())
            } else {
                Ok(())
            }
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

    fn direct_message(id: &str) -> Message {
        Message {
            id: id.to_string(),
            user_id: "2".to_string(),
            user_name: "2".to_string(),
            content: Some("hello".to_string()),
            extra_data: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn likes_new_comments_when_only_like_comments_is_enabled() {
        let data_dir = temp_data_dir("likes-new-comments");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.enabled = false;
                settings.channels.comment.like_comments = true;
                settings.channels.comment.reply.enabled = false;
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

    #[tokio::test]
    async fn does_not_reply_twice_to_the_same_direct_message() {
        let data_dir = temp_data_dir("deduplicate-direct-message");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.channels.direct_message.reply_policy = ReplyPolicy::PerMessage;
            })
            .await
            .unwrap();

        let sent = Arc::new(AtomicUsize::new(0));
        let handler = MockDirectMessageHandler {
            sent: Arc::clone(&sent),
            messages: vec![direct_message("message-key-1")],
            should_fail: false,
        };

        handler.handle(&test_account(), &state).await.unwrap();
        handler.handle(&test_account(), &state).await.unwrap();

        let _ = std::fs::remove_dir_all(data_dir);
        assert_eq!(1, sent.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn replies_to_each_direct_message_under_per_message_policy() {
        let data_dir = temp_data_dir("per-message-direct-messages");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.channels.direct_message.reply_policy = ReplyPolicy::PerMessage;
            })
            .await
            .unwrap();

        let sent = Arc::new(AtomicUsize::new(0));
        let handler = MockDirectMessageHandler {
            sent: Arc::clone(&sent),
            messages: vec![
                direct_message("message-key-1"),
                direct_message("message-key-2"),
            ],
            should_fail: false,
        };

        handler.handle(&test_account(), &state).await.unwrap();

        assert_eq!(2, sent.load(Ordering::SeqCst));
        assert!(state.is_replied("event:1:dm:message-key-1").await);
        assert!(state.is_replied("event:1:dm:message-key-2").await);
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[tokio::test]
    async fn replies_once_per_user_across_different_direct_messages() {
        let data_dir = temp_data_dir("once-per-user-direct-messages");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.channels.direct_message.reply_policy = ReplyPolicy::OncePerUser;
            })
            .await
            .unwrap();

        let sent = Arc::new(AtomicUsize::new(0));
        let handler = MockDirectMessageHandler {
            sent: Arc::clone(&sent),
            messages: vec![
                direct_message("message-key-1"),
                direct_message("message-key-2"),
            ],
            should_fail: false,
        };

        handler.handle(&test_account(), &state).await.unwrap();

        assert_eq!(1, sent.load(Ordering::SeqCst));
        assert!(state.is_replied("user:1:dm:2").await);
        assert!(state.is_replied("event:1:dm:message-key-2").await);
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[tokio::test]
    async fn failed_direct_message_send_is_not_marked_as_replied() {
        let data_dir = temp_data_dir("failed-direct-message");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        let sent = Arc::new(AtomicUsize::new(0));
        let messages = vec![direct_message("message-key-1")];

        let failing_handler = MockDirectMessageHandler {
            sent: Arc::clone(&sent),
            messages: messages.clone(),
            should_fail: true,
        };
        let failed = failing_handler
            .handle(&test_account(), &state)
            .await
            .unwrap();

        assert_eq!(1, failed.error_count);
        assert!(!state.is_replied("event:1:dm:message-key-1").await);

        let successful_handler = MockDirectMessageHandler {
            sent: Arc::clone(&sent),
            messages,
            should_fail: false,
        };
        let retried = successful_handler
            .handle(&test_account(), &state)
            .await
            .unwrap();

        assert_eq!(1, retried.success_count);
        assert_eq!(2, sent.load(Ordering::SeqCst));
        assert!(state.is_replied("event:1:dm:message-key-1").await);
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[tokio::test]
    async fn honors_legacy_once_per_user_direct_message_key() {
        let data_dir = temp_data_dir("legacy-direct-message-key");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.channels.direct_message.reply_policy = ReplyPolicy::OncePerUser;
            })
            .await
            .unwrap();
        state.merge_replied_set(vec!["dm:2".to_string()]).await;

        let sent = Arc::new(AtomicUsize::new(0));
        let handler = MockDirectMessageHandler {
            sent: Arc::clone(&sent),
            messages: vec![direct_message("message-key-1")],
            should_fail: false,
        };

        handler.handle(&test_account(), &state).await.unwrap();

        assert_eq!(0, sent.load(Ordering::SeqCst));
        assert!(state.is_replied("event:1:dm:message-key-1").await);
        assert!(state.is_replied("user:1:dm:2").await);
        let _ = std::fs::remove_dir_all(data_dir);
    }
}

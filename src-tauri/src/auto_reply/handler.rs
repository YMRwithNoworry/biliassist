use super::models::{ChannelReplySettings, MsgSource, ReplyPolicy};
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

/// 通道容量：生产者抓到的消息先放进缓冲区，消费者按节奏回复。
const STREAM_BUFFER: usize = 16;

/// 单条消息的处理结果。
#[derive(Debug, Clone, Copy)]
pub struct MessageOutcome {
    /// 命中风控限制，本轮应立即停止。
    stop: bool,
    /// 是否真的请求了 B站写接口（回复或点赞），用于决定是否需要节流。
    acted: bool,
}

impl MessageOutcome {
    /// 直接跳过（本地判重命中，没有产生任何请求）。
    fn skipped() -> Self {
        Self {
            stop: false,
            acted: false,
        }
    }
}

/// 抓取模式。
///
/// - `Newest`：只抓每个目标的最新一页，用于秒回新评论（快速通道）。
/// - `Full`：逐页补扫历史评论（完整通道），沿用原有的页数上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Newest,
    Full,
}

/// 完整通道每个目标最多扫描的页数。
pub const FULL_SCAN_MAX_PAGES: u32 = 30;

impl ScanMode {
    /// 每个目标最多抓取的页数。
    pub fn max_pages(self) -> u32 {
        match self {
            Self::Newest => 1,
            Self::Full => FULL_SCAN_MAX_PAGES,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Newest => "\u{5feb}\u{901f}\u{901a}\u{9053}",
            Self::Full => "\u{5b8c}\u{6574}\u{626b}\u{63cf}",
        }
    }

    /// 快速通道只处理评论类来源，私信与关注仍按用户设置的间隔检查。
    pub fn includes(self, source: MsgSource) -> bool {
        match self {
            Self::Newest => MsgSource::COMMENT_SOURCES.contains(&source),
            Self::Full => true,
        }
    }
}

/// \u{6d88}\u{606f}\u{5904}\u{7406}\u{5668} trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    fn name(&self) -> &'static str;

    fn source_type(&self) -> MsgSource;

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String>;

    /// 流式产出待处理消息。生产者与消费者并发运行，抓到第一条就能立刻回复，
    /// 因此新评论不必等待整轮扫描结束。
    ///
    /// 默认实现退化为一次性抓取（私信、关注渠道行为不变），
    /// 评论渠道会重写为按最新页优先边抓边发。
    async fn stream_messages(
        &self,
        account: &UserInfo,
        sink: &async_channel::Sender<Message>,
        _mode: ScanMode,
    ) -> Result<(), String> {
        for message in self.fetch_messages(account).await? {
            if sink.send(message).await.is_err() {
                break;
            }
        }
        Ok(())
    }

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
        let source = self.source_type();
        let Some(comment_settings) = settings.comment_settings(source) else {
            return;
        };
        let should_like = message.extra_data["custom_like_comments"]
            .as_bool()
            .unwrap_or(comment_settings.like_comments);
        if !should_like {
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

    /// 完整通道处理，等价于 `handle_in(.., ScanMode::Full)`。
    async fn handle(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
    ) -> Result<HandleResult, String> {
        self.handle_in(account, state, ScanMode::Full).await
    }

    /// 边抓边回：生产者持续把新消息推进通道，消费者收到一条就立即处理一条。
    ///
    /// 首条回复只需要等待第一页抓取时间，不再等待整轮扫描结束。
    async fn handle_in(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
        mode: ScanMode,
    ) -> Result<HandleResult, String> {
        let settings = state.get_settings().await;
        let source = self.source_type();
        let default_channel = settings.channel(source).clone();
        let (sink, stream) = async_channel::bounded::<Message>(STREAM_BUFFER);

        let producer = async {
            let result = self.stream_messages(account, &sink, mode).await;
            sink.close();
            result
        };
        let consumer = async {
            let mut result = HandleResult::default();
            while let Ok(message) = stream.recv().await {
                let outcome = self
                    .reply_to_message(account, state, &default_channel, message, &mut result)
                    .await;
                if outcome.stop {
                    break;
                }
                // 只有真正发出过写请求才节流：跳过已处理的消息不会拖慢快速通道。
                if outcome.acted {
                    processing_delay().await;
                }
            }
            // 命中风控提前停止时关闭接收端，生产者会随之结束，不再继续请求接口。
            stream.close();
            result
        };

        let (stream_result, result) = tokio::join!(producer, consumer);

        if let Err(error) = stream_result {
            log::warn!("{}抓取中断: {}", self.name(), error);
            if result.success_count == 0 && result.error_count == 0 {
                return Err(error);
            }
        }

        Ok(result)
    }

    async fn handle_likes_only(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
    ) -> Result<HandleResult, String> {
        self.handle_likes_only_in(account, state, ScanMode::Full)
            .await
    }

    /// 只点赞的处理同样边抓边做，判断规则与完整通道保持一致。
    async fn handle_likes_only_in(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
        mode: ScanMode,
    ) -> Result<HandleResult, String> {
        let (sink, stream) = async_channel::bounded::<Message>(STREAM_BUFFER);

        let producer = async {
            let result = self.stream_messages(account, &sink, mode).await;
            sink.close();
            result
        };
        let consumer = async {
            let mut result = HandleResult::default();
            while let Ok(message) = stream.recv().await {
                let likes_before = result.like_success_count + result.like_error_count;
                self.like_comment_if_needed(account, &message, state, &mut result)
                    .await;
                if result.stopped_by_rate_limit {
                    break;
                }
                if result.like_success_count + result.like_error_count > likes_before {
                    processing_delay().await;
                }
            }
            stream.close();
            result
        };

        let (stream_result, result) = tokio::join!(producer, consumer);

        if let Err(error) = stream_result {
            log::warn!("{}抓取中断: {}", self.name(), error);
        }

        Ok(result)
    }

    /// 处理单条消息。
    async fn reply_to_message(
        &self,
        account: &UserInfo,
        state: &AutoReplyState,
        default_channel: &ChannelReplySettings,
        message: Message,
        result: &mut HandleResult,
    ) -> MessageOutcome {
        let source = self.source_type();
        let channel = effective_channel(default_channel, &message.extra_data);
        let event_key = format!("event:{}:{}:{}", account.uid, source.id(), message.id);
        let user_key = format!("user:{}:{}:{}", account.uid, source.id(), message.user_id);

        let already_replied_on_bilibili = message.extra_data["already_replied"]
            .as_bool()
            .unwrap_or(false);
        let legacy_event_key = match source {
            MsgSource::Comment => Some(format!("{}:{}", source.id(), message.id)),
            MsgSource::Dynamic | MsgSource::DirectMessage | MsgSource::Follow => None,
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
            let likes_before = result.like_success_count + result.like_error_count;
            self.like_comment_if_needed(account, &message, state, result)
                .await;
            return MessageOutcome {
                stop: result.stopped_by_rate_limit,
                acted: result.like_success_count + result.like_error_count > likes_before,
            };
        }

        if channel.reply_policy == ReplyPolicy::OncePerUser {
            let legacy_user_key = match source {
                MsgSource::DirectMessage | MsgSource::Follow => {
                    Some(format!("{}:{}", source.id(), message.user_id))
                }
                MsgSource::Comment | MsgSource::Dynamic => None,
            };
            let replied_by_key = state.is_replied(&user_key).await
                || match legacy_user_key.as_deref() {
                    Some(key) => state.is_replied(key).await,
                    None => false,
                };
            let replied_by_history = !matches!(source, MsgSource::Comment | MsgSource::Dynamic)
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
                return MessageOutcome::skipped();
            }
        }

        let reply_text = format_message(&channel.message, &message.user_name);

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

                self.like_comment_if_needed(account, &message, state, result)
                    .await;
                MessageOutcome {
                    stop: result.stopped_by_rate_limit,
                    acted: true,
                }
            }
            Err(e) => {
                log::error!("{}回复失败: {}", self.name(), e);
                result.error_count += 1;

                if is_rate_limit_error(&e) {
                    result.stopped_by_rate_limit = true;
                }
                MessageOutcome {
                    stop: result.stopped_by_rate_limit,
                    acted: true,
                }
            }
        }
    }
}

fn effective_channel(
    default_channel: &ChannelReplySettings,
    extra_data: &serde_json::Value,
) -> ChannelReplySettings {
    let mut channel = default_channel.clone();
    if let Some(message) = extra_data["custom_message"].as_str() {
        channel.message = message.to_string();
    }
    if let Some(policy) = extra_data["custom_reply_policy"].as_str() {
        channel.reply_policy = match policy {
            "oncePerUser" => ReplyPolicy::OncePerUser,
            _ => ReplyPolicy::PerMessage,
        };
    }
    channel
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
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    struct MockCommentHandler {
        source: MsgSource,
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
            self.source
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

    #[test]
    fn custom_channel_overrides_message_and_policy() {
        let default_channel = ChannelReplySettings::default();
        let extra_data = serde_json::json!({
            "custom_message": "指定视频回复",
            "custom_reply_policy": "oncePerUser"
        });
        let channel = effective_channel(&default_channel, &extra_data);
        assert_eq!("指定视频回复", channel.message);
        assert_eq!(ReplyPolicy::OncePerUser, channel.reply_policy);
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
            source: MsgSource::Comment,
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
    async fn dynamic_comments_use_their_own_like_setting() {
        let data_dir = temp_data_dir("likes-dynamic-comments");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.enabled = false;
                settings.channels.comment.like_comments = false;
                settings.channels.dynamic.like_comments = true;
                settings.channels.dynamic.reply.enabled = false;
            })
            .await
            .unwrap();

        let liked = Arc::new(AtomicUsize::new(0));
        let handler = MockCommentHandler {
            source: MsgSource::Dynamic,
            liked: Arc::clone(&liked),
            messages: vec![comment_message("100:2", false)],
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

    #[test]
    fn newest_mode_only_covers_comment_sources() {
        assert_eq!(1, ScanMode::Newest.max_pages());
        assert_eq!(FULL_SCAN_MAX_PAGES, ScanMode::Full.max_pages());
        assert!(ScanMode::Newest.includes(MsgSource::Comment));
        assert!(ScanMode::Newest.includes(MsgSource::Dynamic));
        assert!(!ScanMode::Newest.includes(MsgSource::DirectMessage));
        assert!(!ScanMode::Newest.includes(MsgSource::Follow));
        assert!(ScanMode::Full.includes(MsgSource::DirectMessage));
        assert!(ScanMode::Full.includes(MsgSource::Follow));
    }

    /// 推送第一条后等待消费者回复，再推送第二条。
    /// 如果实现仍是"先抓完再统一回复"，这里的等待会超时，`replied_while_streaming` 保持 false。
    struct StreamingCommentHandler {
        replied: Arc<AtomicUsize>,
        sent: Arc<AtomicUsize>,
        replied_while_streaming: Arc<AtomicBool>,
    }

    #[async_trait]
    impl MessageHandler for StreamingCommentHandler {
        fn name(&self) -> &'static str {
            "streaming"
        }

        fn source_type(&self) -> MsgSource {
            MsgSource::Comment
        }

        async fn fetch_messages(&self, _account: &UserInfo) -> Result<Vec<Message>, String> {
            Err("流式通道不应该退化为一次性抓取".to_string())
        }

        async fn send_reply(
            &self,
            _account: &UserInfo,
            _message: &Message,
            _reply_msg: &str,
        ) -> Result<(), String> {
            self.sent.fetch_add(1, Ordering::SeqCst);
            self.replied.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn stream_messages(
            &self,
            _account: &UserInfo,
            sink: &async_channel::Sender<Message>,
            _mode: ScanMode,
        ) -> Result<(), String> {
            sink.send(comment_message("stream-1", false))
                .await
                .map_err(|e| e.to_string())?;

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
            while self.replied.load(Ordering::SeqCst) == 0 {
                if tokio::time::Instant::now() >= deadline {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            if self.replied.load(Ordering::SeqCst) > 0 {
                self.replied_while_streaming.store(true, Ordering::SeqCst);
            }

            sink.send(comment_message("stream-2", false))
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn replies_before_the_stream_finishes() {
        let data_dir = temp_data_dir("streaming-replies");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();

        let replied = Arc::new(AtomicUsize::new(0));
        let sent = Arc::new(AtomicUsize::new(0));
        let replied_while_streaming = Arc::new(AtomicBool::new(false));
        let handler = StreamingCommentHandler {
            replied: Arc::clone(&replied),
            sent: Arc::clone(&sent),
            replied_while_streaming: Arc::clone(&replied_while_streaming),
        };

        handler.handle(&test_account(), &state).await.unwrap();

        assert!(
            replied_while_streaming.load(Ordering::SeqCst),
            "第二条消息推送前，第一条回复就应该已经发出"
        );
        assert_eq!(2, sent.load(Ordering::SeqCst));
        assert!(state.is_replied("event:1:c:stream-1").await);
        assert!(state.is_replied("event:1:c:stream-2").await);
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[tokio::test]
    async fn likes_only_streams_through_the_same_channel() {
        let data_dir = temp_data_dir("streaming-likes");
        let state = AutoReplyState::new_for_test(data_dir.clone()).unwrap();
        state
            .update_settings(|settings| {
                settings.channels.comment.like_comments = true;
            })
            .await
            .unwrap();

        let liked = Arc::new(AtomicUsize::new(0));
        let handler = MockCommentHandler {
            source: MsgSource::Comment,
            liked: Arc::clone(&liked),
            messages: vec![
                comment_message("like-1", false),
                comment_message("like-2", false),
            ],
        };

        handler
            .handle_likes_only_in(&test_account(), &state, ScanMode::Newest)
            .await
            .unwrap();

        assert_eq!(2, liked.load(Ordering::SeqCst));
        let _ = std::fs::remove_dir_all(data_dir);
    }
}

use super::models::MsgSource;
use super::state::AutoReplyState;
use crate::bilibili::UserInfo;
use async_trait::async_trait;

/// 消息结构
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    #[allow(dead_code)]
    pub content: Option<String>,
    pub extra_data: serde_json::Value,
}

/// 处理结果
#[derive(Debug, Default)]
pub struct HandleResult {
    pub success_count: u32,
    pub error_count: u32,
    pub stopped_by_rate_limit: bool,
}

/// 消息处理器 trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    fn name(&self) -> &'static str;

    fn source_type(&self) -> MsgSource;

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String>;

    async fn send_reply(&self, account: &UserInfo, message: &Message, reply_msg: &str) -> Result<(), String>;

    fn needs_history_fallback(&self) -> bool {
        matches!(self.source_type(), MsgSource::DirectMessage | MsgSource::Follow)
    }

    async fn handle(&self, account: &UserInfo, state: &AutoReplyState) -> Result<HandleResult, String> {
        let settings = state.get_settings().await;
        let messages = self.fetch_messages(account).await?;

        let mut result = HandleResult::default();

        for message in messages {
            let dedup_key = format!("{}:{}", self.source_type().id(), message.id);

            if settings.reply_only_once && state.is_replied(&dedup_key).await {
                continue;
            }

            // 历史记录回查（私信/关注降级保障）
            if settings.reply_only_once && self.needs_history_fallback()
                && state.is_replied_in_history(&message.user_id, &self.source_type()).await
            {
                log::info!("历史记录回查命中，跳过已回复用户: {}", message.user_id);
                // 同步到 replied_set 避免下次再查历史
                state.mark_replied(dedup_key).await;
                continue;
            }

            let formatted = format_message(&settings.message, &message.user_name);

            match self.send_reply(account, &message, &formatted).await {
                Ok(_) => {
                    if settings.reply_only_once {
                        state.mark_replied(dedup_key).await;
                    }
                    state.add_history(message.user_name.clone(), formatted, self.source_type()).await;
                    result.success_count += 1;
                }
                Err(e) => {
                    log::error!("{}回复失败: {}", self.name(), e);
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
}

/// 格式化消息
pub fn format_message(template: &str, username: &str) -> String {
    use chrono::{FixedOffset, TimeZone};
    let beijing_now = FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .from_utc_datetime(&chrono::Utc::now().naive_utc());
    template
        .replace("{用户名}", username)
        .replace("{时间}", &beijing_now.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// 判断是否为风控错误
pub fn is_rate_limit_error(error: &str) -> bool {
    error.contains("banned") || error.contains("频繁")
}

/// 生成设备ID (dev_id)
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

/// 消息处理器注册表
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

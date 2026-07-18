use chrono::{FixedOffset, TimeZone};
use serde::{Deserialize, Deserializer, Serialize};

const DEFAULT_MESSAGE: &str = "感谢您的留言！我会尽快回复。";
const DEFAULT_AI_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_AI_MODEL: &str = "gpt-4o-mini";

fn default_true() -> bool {
    true
}

fn default_interval() -> u64 {
    60
}

fn default_message() -> String {
    DEFAULT_MESSAGE.to_string()
}

fn default_ai_base_url() -> String {
    DEFAULT_AI_BASE_URL.to_string()
}

fn default_ai_model() -> String {
    DEFAULT_AI_MODEL.to_string()
}

pub(crate) fn beijing_now() -> chrono::DateTime<FixedOffset> {
    FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .from_utc_datetime(&chrono::Utc::now().naive_utc())
}

/// 消息来源类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum MsgSource {
    Comment,
    DirectMessage,
    Follow,
}

impl MsgSource {
    pub const ALL: [MsgSource; 3] = [
        MsgSource::Comment,
        MsgSource::DirectMessage,
        MsgSource::Follow,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            MsgSource::Comment => "评论",
            MsgSource::DirectMessage => "私信",
            MsgSource::Follow => "关注",
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            MsgSource::Comment => "c",
            MsgSource::DirectMessage => "dm",
            MsgSource::Follow => "f",
        }
    }
}

/// AI 服务的请求与响应协议格式。
///
/// 缺失该字段的旧配置会继续使用 OpenAI Chat Completions，保持原有行为。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiApiFormat {
    #[default]
    OpenAiChatCompletions,
    OpenAiCompletions,
    OpenAiResponses,
    AnthropicMessages,
}

/// 共享的 AI 服务连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderConfig {
    #[serde(default)]
    pub api_format: AiApiFormat,
    #[serde(default = "default_ai_base_url")]
    pub base_url: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            api_format: AiApiFormat::default(),
            base_url: default_ai_base_url(),
            model: default_ai_model(),
            api_key: String::new(),
        }
    }
}

/// 每个回复渠道独立的 AI 行为配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub prompt_template: String,
}

/// AI 运行时配置，同时用于读取旧版设置 JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReplyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_format: AiApiFormat,
    #[serde(default = "default_ai_base_url")]
    pub base_url: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub prompt_template: String,
}

impl Default for AiReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_format: AiApiFormat::default(),
            base_url: default_ai_base_url(),
            model: default_ai_model(),
            api_key: String::new(),
            system_prompt: String::new(),
            prompt_template: String::new(),
        }
    }
}

impl AiReplyConfig {
    pub fn from_parts(provider: &AiProviderConfig, channel: &ChannelAiConfig) -> Self {
        Self {
            enabled: channel.enabled,
            api_format: provider.api_format,
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            api_key: provider.api_key.clone(),
            system_prompt: channel.system_prompt.clone(),
            prompt_template: channel.prompt_template.clone(),
        }
    }

    fn provider(&self) -> AiProviderConfig {
        AiProviderConfig {
            api_format: self.api_format,
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            api_key: self.api_key.clone(),
        }
    }

    fn channel(&self) -> ChannelAiConfig {
        ChannelAiConfig {
            enabled: self.enabled,
            system_prompt: self.system_prompt.clone(),
            prompt_template: self.prompt_template.clone(),
        }
    }

    pub fn effective_system_prompt(&self) -> String {
        if self.system_prompt.trim().is_empty() {
            "你是一个友善的B站UP主助手，负责回复粉丝的评论和私信。请根据对方的消息内容生成一条简短、友好、自然的回复。回复应该简洁（不超过50个字），语气亲切。".to_string()
        } else {
            self.system_prompt.clone()
        }
    }

    pub fn effective_prompt_template(&self) -> String {
        if self.prompt_template.trim().is_empty() {
            "用户「{用户名}」通过{来源}给你发了一条消息：「{消息内容}」\n请生成一条合适的回复。"
                .to_string()
        } else {
            self.prompt_template.clone()
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReplyPolicy {
    #[default]
    PerMessage,
    OncePerUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelReplySettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_message")]
    pub message: String,
    #[serde(default)]
    pub reply_policy: ReplyPolicy,
    #[serde(default)]
    pub ai: ChannelAiConfig,
}

impl Default for ChannelReplySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            message: default_message(),
            reply_policy: ReplyPolicy::PerMessage,
            ai: ChannelAiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentReplySettings {
    #[serde(flatten)]
    pub reply: ChannelReplySettings,
    #[serde(default = "default_true")]
    pub like_comments: bool,
}

impl Default for CommentReplySettings {
    fn default() -> Self {
        Self {
            reply: ChannelReplySettings::default(),
            like_comments: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReplyChannels {
    #[serde(default)]
    pub comment: CommentReplySettings,
    #[serde(default = "default_direct_message_settings")]
    pub direct_message: ChannelReplySettings,
    #[serde(default = "default_follow_settings")]
    pub follow: ChannelReplySettings,
}

fn default_direct_message_settings() -> ChannelReplySettings {
    ChannelReplySettings {
        reply_policy: ReplyPolicy::OncePerUser,
        ..ChannelReplySettings::default()
    }
}

fn default_follow_settings() -> ChannelReplySettings {
    ChannelReplySettings {
        reply_policy: ReplyPolicy::OncePerUser,
        ..ChannelReplySettings::default()
    }
}

impl Default for AutoReplyChannels {
    fn default() -> Self {
        Self {
            comment: CommentReplySettings::default(),
            direct_message: default_direct_message_settings(),
            follow: default_follow_settings(),
        }
    }
}

impl AutoReplyChannels {
    pub fn any_enabled(&self) -> bool {
        self.comment.reply.enabled || self.direct_message.enabled || self.follow.enabled
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReplySettings {
    pub enabled: bool,
    pub interval: u64,
    pub ai_provider: AiProviderConfig,
    pub channels: AutoReplyChannels,
    pub history: Vec<ReplyHistory>,
}

impl Default for AutoReplySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: default_interval(),
            ai_provider: AiProviderConfig::default(),
            channels: AutoReplyChannels::default(),
            history: Vec::new(),
        }
    }
}

impl AutoReplySettings {
    pub fn channel(&self, source: MsgSource) -> &ChannelReplySettings {
        match source {
            MsgSource::Comment => &self.channels.comment.reply,
            MsgSource::DirectMessage => &self.channels.direct_message,
            MsgSource::Follow => &self.channels.follow,
        }
    }

    pub fn resolved_ai(&self, source: MsgSource) -> AiReplyConfig {
        AiReplyConfig::from_parts(&self.ai_provider, &self.channel(source).ai)
    }

    pub fn enabled_sources(&self) -> Vec<MsgSource> {
        MsgSource::ALL
            .into_iter()
            .filter(|source| self.channel(*source).enabled)
            .collect()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoReplySettingsWire {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    interval: Option<u64>,
    #[serde(default)]
    ai_provider: Option<AiProviderConfig>,
    #[serde(default)]
    channels: Option<AutoReplyChannels>,
    #[serde(default)]
    history: Vec<ReplyHistory>,

    // Legacy fields from the shared configuration format.
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    reply_only_once: Option<bool>,
    #[serde(default)]
    sources: Option<Vec<MsgSource>>,
    #[serde(default)]
    like_comments: Option<bool>,
    #[serde(default)]
    ai: Option<AiReplyConfig>,
}

impl<'de> Deserialize<'de> for AutoReplySettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AutoReplySettingsWire::deserialize(deserializer)?;

        if let Some(channels) = wire.channels {
            return Ok(Self {
                enabled: wire.enabled.unwrap_or(true),
                interval: wire.interval.unwrap_or_else(default_interval),
                ai_provider: wire.ai_provider.unwrap_or_default(),
                channels,
                history: wire.history,
            });
        }

        let legacy_ai = wire.ai.unwrap_or_default();
        let channel_ai = legacy_ai.channel();
        let message = wire.message.unwrap_or_else(default_message);
        let reply_only_once = wire.reply_only_once.unwrap_or(true);
        let sources = wire.sources.unwrap_or_else(|| MsgSource::ALL.to_vec());
        let legacy_policy = if reply_only_once {
            ReplyPolicy::OncePerUser
        } else {
            ReplyPolicy::PerMessage
        };

        let channel = |source, reply_policy| ChannelReplySettings {
            enabled: sources.contains(&source),
            message: message.clone(),
            reply_policy,
            ai: channel_ai.clone(),
        };

        Ok(Self {
            enabled: wire.enabled.unwrap_or(true),
            interval: wire.interval.unwrap_or_else(default_interval),
            ai_provider: wire.ai_provider.unwrap_or_else(|| legacy_ai.provider()),
            channels: AutoReplyChannels {
                comment: CommentReplySettings {
                    reply: channel(MsgSource::Comment, ReplyPolicy::PerMessage),
                    like_comments: wire.like_comments.unwrap_or(true),
                },
                direct_message: channel(MsgSource::DirectMessage, legacy_policy),
                follow: channel(MsgSource::Follow, legacy_policy),
            },
            history: wire.history,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyHistory {
    pub user: String,
    pub time: String,
    pub message: String,
    pub source: MsgSource,
}

impl ReplyHistory {
    pub fn new(user: String, message: String, source: MsgSource) -> Self {
        Self {
            user,
            time: beijing_now().format("%Y-%m-%d %H:%M:%S").to_string(),
            message,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_shared_settings_into_channels() {
        let legacy = serde_json::json!({
            "enabled": true,
            "message": "旧版回复",
            "interval": 15,
            "replyOnlyOnce": true,
            "sources": ["comment", "directMessage"],
            "history": [],
            "likeComments": false,
            "ai": {
                "enabled": true,
                "baseUrl": "https://example.com/v1",
                "model": "example-model",
                "apiKey": "secret",
                "systemPrompt": "旧系统提示词",
                "promptTemplate": "旧模板 {消息内容}"
            }
        });

        let settings: AutoReplySettings = serde_json::from_value(legacy).unwrap();

        assert_eq!(15, settings.interval);
        assert_eq!(
            AiApiFormat::OpenAiChatCompletions,
            settings.ai_provider.api_format
        );
        assert_eq!("https://example.com/v1", settings.ai_provider.base_url);
        assert!(settings.channels.comment.reply.enabled);
        assert!(settings.channels.direct_message.enabled);
        assert!(!settings.channels.follow.enabled);
        assert_eq!(
            ReplyPolicy::PerMessage,
            settings.channels.comment.reply.reply_policy
        );
        assert_eq!(
            ReplyPolicy::OncePerUser,
            settings.channels.direct_message.reply_policy
        );
        assert_eq!(
            ReplyPolicy::OncePerUser,
            settings.channels.follow.reply_policy
        );
        assert_eq!("旧版回复", settings.channels.direct_message.message);
        assert!(settings.channels.direct_message.ai.enabled);
        assert!(!settings.channels.comment.like_comments);

        let canonical = serde_json::to_value(settings).unwrap();
        assert!(canonical.get("channels").is_some());
        assert!(canonical.get("aiProvider").is_some());
        assert!(canonical.get("sources").is_none());
        assert!(canonical.get("replyOnlyOnce").is_none());
    }

    #[test]
    fn round_trips_channel_settings() {
        let mut settings = AutoReplySettings::default();
        settings.ai_provider.api_format = AiApiFormat::AnthropicMessages;
        settings.channels.comment.reply.message = "评论回复".to_string();
        settings.channels.direct_message.message = "私信回复".to_string();
        settings.channels.follow.message = "关注回复".to_string();

        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AutoReplySettings = serde_json::from_str(&json).unwrap();

        assert_eq!(
            AiApiFormat::AnthropicMessages,
            decoded.ai_provider.api_format
        );
        assert_eq!("评论回复", decoded.channels.comment.reply.message);
        assert_eq!("私信回复", decoded.channels.direct_message.message);
        assert_eq!("关注回复", decoded.channels.follow.message);
    }
}

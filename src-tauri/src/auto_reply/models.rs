use chrono::{FixedOffset, TimeZone};
use serde::{Deserialize, Serialize};

pub(crate) fn beijing_now() -> chrono::DateTime<FixedOffset> {
    FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .from_utc_datetime(&chrono::Utc::now().naive_utc())
}

/// 消息来源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum MsgSource {
    Comment,
    DirectMessage,
    Follow,
}

impl MsgSource {
    #[allow(dead_code)]
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

/// AI 回复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiReplyConfig {
    /// 是否启用 AI 生成回复
    #[serde(default)]
    pub enabled: bool,
    /// API Base URL（兼容 OpenAI 接口）
    #[serde(default)]
    pub base_url: String,
    /// 模型名称
    #[serde(default)]
    pub model: String,
    /// API Key
    #[serde(default)]
    pub api_key: String,
    /// 系统提示词 —— 设定 AI 的角色与回复风格
    #[serde(default)]
    pub system_prompt: String,
    /// 回复提示词模板 —— 每次生成回复时发送给 AI 的用户消息模板
    /// 支持变量：{用户名}、{消息内容}、{来源}
    #[serde(default)]
    pub prompt_template: String,
}

impl Default for AiReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: String::new(),
            system_prompt: String::new(),
            prompt_template: String::new(),
        }
    }
}

impl AiReplyConfig {
    /// 获取系统提示词，若为空则返回默认值
    pub fn effective_system_prompt(&self) -> String {
        if self.system_prompt.trim().is_empty() {
            "你是一个友善的B站UP主助手，负责回复粉丝的评论和私信。请根据对方的消息内容生成一条简短、友好、自然的回复。回复应该简洁（不超过50个字），语气亲切。".to_string()
        } else {
            self.system_prompt.clone()
        }
    }

    /// 获取回复提示词模板，若为空则返回默认模板
    pub fn effective_prompt_template(&self) -> String {
        if self.prompt_template.trim().is_empty() {
            "用户「{用户名}」通过{来源}给你发了一条消息：「{消息内容}」\n请生成一条合适的回复。".to_string()
        } else {
            self.prompt_template.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReplySettings {
    pub enabled: bool,
    pub message: String,
    pub interval: u64,
    pub reply_only_once: bool,
    pub sources: Vec<MsgSource>,
    #[serde(default)]
    pub history: Vec<ReplyHistory>,
    #[serde(default)]
    pub like_comments: bool,
    #[serde(default)]
    pub ai: AiReplyConfig,
}

impl AutoReplySettings {
    pub fn default() -> Self {
        Self {
            enabled: true,
            message: "感谢您的留言！我会尽快回复。".to_string(),
            interval: 60,
            reply_only_once: true,
            sources: vec![MsgSource::Comment, MsgSource::DirectMessage, MsgSource::Follow],
            history: Vec::new(),
            like_comments: true,
            ai: AiReplyConfig::default(),
        }
    }

    #[allow(dead_code)]
    pub fn is_source_enabled(&self, source: &MsgSource) -> bool {
        self.sources.contains(source)
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
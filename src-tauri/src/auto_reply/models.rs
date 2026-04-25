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

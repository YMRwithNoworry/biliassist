use chrono::{FixedOffset, TimeZone};
use serde::{Deserialize, Deserializer, Serialize};

const DEFAULT_MESSAGE: &str = "感谢您的留言！我会尽快回复。";

fn default_true() -> bool {
    true
}

fn default_interval() -> u64 {
    60
}

fn default_message() -> String {
    DEFAULT_MESSAGE.to_string()
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
    Dynamic,
    DirectMessage,
    Follow,
}

impl MsgSource {
    pub const ALL: [MsgSource; 4] = [
        MsgSource::Comment,
        MsgSource::Dynamic,
        MsgSource::DirectMessage,
        MsgSource::Follow,
    ];

    pub const COMMENT_SOURCES: [MsgSource; 2] = [MsgSource::Comment, MsgSource::Dynamic];

    pub fn display_name(&self) -> &'static str {
        match self {
            MsgSource::Comment => "评论",
            MsgSource::Dynamic => "动态评论",
            MsgSource::DirectMessage => "私信",
            MsgSource::Follow => "关注",
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            MsgSource::Comment => "c",
            MsgSource::Dynamic => "dy",
            MsgSource::DirectMessage => "dm",
            MsgSource::Follow => "f",
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
}

impl Default for ChannelReplySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            message: default_message(),
            reply_policy: ReplyPolicy::PerMessage,
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

/// 用户指定的视频及其独立回复配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedVideoSettings {
    #[serde(default)]
    pub bvid: String,
    #[serde(flatten)]
    pub reply: ChannelReplySettings,
    #[serde(default = "default_true")]
    pub like_comments: bool,
}

impl Default for TrackedVideoSettings {
    fn default() -> Self {
        Self {
            bvid: String::new(),
            reply: ChannelReplySettings::default(),
            like_comments: true,
        }
    }
}

impl TrackedVideoSettings {
    pub fn normalized_bvid(&self) -> Option<String> {
        let bvid = self.bvid.trim();
        let prefix = bvid.get(..2)?;
        if !prefix.eq_ignore_ascii_case("bv") || bvid.len() < 3 {
            return None;
        }
        Some(format!("BV{}", &bvid[2..]))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReplyChannels {
    #[serde(default)]
    pub comment: CommentReplySettings,
    #[serde(default)]
    pub dynamic: CommentReplySettings,
    #[serde(default = "default_direct_message_settings")]
    pub direct_message: ChannelReplySettings,
    #[serde(default = "default_follow_settings")]
    pub follow: ChannelReplySettings,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoReplyChannelsWire {
    #[serde(default)]
    comment: CommentReplySettings,
    #[serde(default)]
    dynamic: Option<CommentReplySettings>,
    #[serde(default = "default_direct_message_settings")]
    direct_message: ChannelReplySettings,
    #[serde(default = "default_follow_settings")]
    follow: ChannelReplySettings,
}

impl<'de> Deserialize<'de> for AutoReplyChannels {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AutoReplyChannelsWire::deserialize(deserializer)?;
        let dynamic = wire.dynamic.unwrap_or_else(|| wire.comment.clone());
        Ok(Self {
            comment: wire.comment,
            dynamic,
            direct_message: wire.direct_message,
            follow: wire.follow,
        })
    }
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
            dynamic: CommentReplySettings::default(),
            direct_message: default_direct_message_settings(),
            follow: default_follow_settings(),
        }
    }
}

impl AutoReplyChannels {
    pub fn any_enabled(&self) -> bool {
        self.comment.reply.enabled
            || self.dynamic.reply.enabled
            || self.direct_message.enabled
            || self.follow.enabled
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReplySettings {
    pub enabled: bool,
    pub interval: u64,
    pub channels: AutoReplyChannels,
    pub tracked_videos: Vec<TrackedVideoSettings>,
    pub history: Vec<ReplyHistory>,
}

impl Default for AutoReplySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: default_interval(),
            channels: AutoReplyChannels::default(),
            tracked_videos: Vec::new(),
            history: Vec::new(),
        }
    }
}

impl AutoReplySettings {
    pub fn any_enabled(&self) -> bool {
        self.channels.any_enabled() || self.has_enabled_tracked_videos()
    }

    pub fn channel(&self, source: MsgSource) -> &ChannelReplySettings {
        match source {
            MsgSource::Comment => &self.channels.comment.reply,
            MsgSource::Dynamic => &self.channels.dynamic.reply,
            MsgSource::DirectMessage => &self.channels.direct_message,
            MsgSource::Follow => &self.channels.follow,
        }
    }

    pub fn enabled_sources(&self) -> Vec<MsgSource> {
        MsgSource::ALL
            .into_iter()
            .filter(|source| {
                self.channel(*source).enabled
                    || (*source == MsgSource::Comment && self.has_enabled_tracked_videos())
            })
            .collect()
    }

    pub fn has_enabled_tracked_videos(&self) -> bool {
        self.tracked_videos
            .iter()
            .any(|video| video.reply.enabled && video.normalized_bvid().is_some())
    }

    pub fn likes_enabled_for_source(&self, source: MsgSource) -> bool {
        self.comment_settings(source)
            .map(|channel| channel.like_comments)
            .unwrap_or(false)
            || (source == MsgSource::Comment
                && self
                    .tracked_videos
                    .iter()
                    .any(|video| video.reply.enabled && video.like_comments))
    }

    pub fn comment_settings(&self, source: MsgSource) -> Option<&CommentReplySettings> {
        match source {
            MsgSource::Comment => Some(&self.channels.comment),
            MsgSource::Dynamic => Some(&self.channels.dynamic),
            MsgSource::DirectMessage | MsgSource::Follow => None,
        }
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
    channels: Option<AutoReplyChannels>,
    #[serde(default)]
    tracked_videos: Option<Vec<TrackedVideoSettings>>,
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
                channels,
                tracked_videos: wire.tracked_videos.unwrap_or_default(),
                history: wire.history,
            });
        }

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
        };

        let comment_reply = channel(MsgSource::Comment, ReplyPolicy::PerMessage);
        let like_comments = wire.like_comments.unwrap_or(true);

        Ok(Self {
            enabled: wire.enabled.unwrap_or(true),
            interval: wire.interval.unwrap_or_else(default_interval),
            channels: AutoReplyChannels {
                comment: CommentReplySettings {
                    reply: comment_reply.clone(),
                    like_comments,
                },
                dynamic: CommentReplySettings {
                    reply: comment_reply,
                    like_comments,
                },
                direct_message: channel(MsgSource::DirectMessage, legacy_policy),
                follow: channel(MsgSource::Follow, legacy_policy),
            },
            tracked_videos: Vec::new(),
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
            "aiProvider": { "model": "example-model" },
            "ai": { "enabled": true }
        });

        let settings: AutoReplySettings = serde_json::from_value(legacy).unwrap();

        assert_eq!(15, settings.interval);
        assert!(settings.channels.comment.reply.enabled);
        assert!(settings.channels.dynamic.reply.enabled);
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
        assert!(!settings.channels.comment.like_comments);
        assert!(!settings.channels.dynamic.like_comments);

        let canonical = serde_json::to_value(settings).unwrap();
        assert!(canonical.get("channels").is_some());
        assert!(canonical.get("aiProvider").is_none());
        assert!(canonical.get("sources").is_none());
        assert!(canonical.get("replyOnlyOnce").is_none());
    }

    #[test]
    fn round_trips_channel_settings() {
        let mut settings = AutoReplySettings::default();
        settings.channels.comment.reply.message = "评论回复".to_string();
        settings.channels.dynamic.reply.message = "动态回复".to_string();
        settings.channels.direct_message.message = "私信回复".to_string();
        settings.channels.follow.message = "关注回复".to_string();

        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AutoReplySettings = serde_json::from_str(&json).unwrap();

        assert_eq!("评论回复", decoded.channels.comment.reply.message);
        assert_eq!("动态回复", decoded.channels.dynamic.reply.message);
        assert_eq!("私信回复", decoded.channels.direct_message.message);
        assert_eq!("关注回复", decoded.channels.follow.message);
    }

    #[test]
    fn existing_channel_settings_are_copied_to_new_dynamic_channel() {
        let stored = serde_json::json!({
            "enabled": true,
            "interval": 5,
            "channels": {
                "comment": {
                    "enabled": false,
                    "message": "原评论回复",
                    "replyPolicy": "oncePerUser",
                    "likeComments": false
                }
            },
            "history": []
        });

        let settings: AutoReplySettings = serde_json::from_value(stored).unwrap();

        assert!(!settings.channels.dynamic.reply.enabled);
        assert_eq!("原评论回复", settings.channels.dynamic.reply.message);
        assert_eq!(
            ReplyPolicy::OncePerUser,
            settings.channels.dynamic.reply.reply_policy
        );
        assert!(!settings.channels.dynamic.like_comments);
        assert!(settings.tracked_videos.is_empty());
    }

    #[test]
    fn round_trips_tracked_video_settings_and_enables_comment_source() {
        let stored = serde_json::json!({
            "enabled": true,
            "channels": {
                "comment": {
                    "enabled": false,
                    "message": "默认回复",
                    "likeComments": false
                }
            },
            "trackedVideos": [{
                "bvid": " bv1Test ",
                "enabled": true,
                "message": "指定视频回复 {用户名}",
                "replyPolicy": "oncePerUser",
                "likeComments": false
            }]
        });

        let settings: AutoReplySettings = serde_json::from_value(stored).unwrap();
        assert_eq!(
            Some("BV1Test".to_string()),
            settings.tracked_videos[0].normalized_bvid()
        );
        assert!(settings.has_enabled_tracked_videos());
        assert!(settings.enabled_sources().contains(&MsgSource::Comment));
        assert!(!settings.likes_enabled_for_source(MsgSource::Comment));

        let canonical = serde_json::to_value(&settings).unwrap();
        assert_eq!(" bv1Test ", canonical["trackedVideos"][0]["bvid"]);
        assert_eq!(
            "指定视频回复 {用户名}",
            canonical["trackedVideos"][0]["message"]
        );
        assert_eq!("oncePerUser", canonical["trackedVideos"][0]["replyPolicy"]);
    }

    #[test]
    fn invalid_tracked_video_bvid_is_ignored() {
        let settings = TrackedVideoSettings {
            bvid: "不是BV号".to_string(),
            ..TrackedVideoSettings::default()
        };
        assert!(settings.normalized_bvid().is_none());
    }
}

use crate::auto_reply::direct_message;
use super::handler::{MessageHandler, Message};
use super::http::{get_http_client, resp_to_json};
use super::models::MsgSource;
use crate::bilibili::UserInfo;
use async_trait::async_trait;

/// 关注处理器
pub struct FollowHandler;

impl FollowHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageHandler for FollowHandler {
    fn name(&self) -> &'static str {
        "关注处理器"
    }

    fn source_type(&self) -> MsgSource {
        MsgSource::Follow
    }

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String> {
        let pn = "1".to_string();
        let ps = "10".to_string();

        let resp = get_http_client()
            .get("https://api.bilibili.com/x/relation/followers")
            .header("Cookie", &account.cookie)
            .header("Referer", "https://www.bilibili.com")
            .query(&[("vmid", &account.uid), ("pn", &pn), ("ps", &ps)])
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let json = resp_to_json(resp).await?;

        if json["code"] != 0 {
            return Err(format!("获取粉丝列表失败: {}", json["message"]));
        }

        let empty = vec![];
        let list = json["data"]["list"].as_array().unwrap_or(&empty);
        let mut messages = Vec::new();

        for f in list {
            let mid = f["mid"].as_i64().unwrap_or(0);
            let name = f["uname"].as_str().unwrap_or("").to_string();
            let uid = mid.to_string();

            if mid == 0 { continue; }

            messages.push(Message {
                id: uid.clone(),
                user_id: uid,
                user_name: name,
                content: None,
                extra_data: serde_json::Value::Null,
            });
        }

        Ok(messages)
    }

    async fn send_reply(&self, account: &UserInfo, message: &Message, reply_msg: &str) -> Result<(), String> {
        direct_message::send_dm(account, &message.user_id, reply_msg).await
    }
}

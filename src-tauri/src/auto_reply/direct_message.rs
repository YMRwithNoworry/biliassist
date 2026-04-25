use super::handler::{MessageHandler, Message, generate_dev_id};
use super::http::{get_http_client, resp_to_json, extract_csrf};
use super::models::MsgSource;
use crate::bilibili::UserInfo;
use async_trait::async_trait;

/// 发送私信（公共函数，供关注处理器复用）
pub async fn send_dm(
    account: &UserInfo,
    uid: &str,
    msg: &str,
) -> Result<(), String> {
    let csrf = extract_csrf(&account.cookie);
    if csrf.is_empty() {
        return Err("cookie 中缺少 bili_jct (CSRF token)".to_string());
    }

    let receiver_id = uid.parse::<i64>().unwrap_or(0);
    let sender_id = account.uid.parse::<i64>().unwrap_or(0);
    let dev_id = generate_dev_id();
    let timestamp = chrono::Utc::now().timestamp();

    let content_json = serde_json::json!({
        "content": msg
    });

    let receiver_type = "1".to_string();
    let msg_type = "1".to_string();
    let msg_status = "0".to_string();

    let resp = get_http_client()
        .post("https://api.vc.bilibili.com/web_im/v1/web_im/send_msg")
        .header("Cookie", &account.cookie)
        .header("Referer", "https://message.bilibili.com/")
        .header("Origin", "https://message.bilibili.com")
        .header("Accept", "application/json, text/plain, */*")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .form(&[
            ("msg[sender_uid]", &sender_id.to_string()),
            ("msg[receiver_id]", &receiver_id.to_string()),
            ("msg[receiver_type]", &receiver_type),
            ("msg[msg_type]", &msg_type),
            ("msg[msg_status]", &msg_status),
            ("msg[dev_id]", &dev_id),
            ("msg[timestamp]", &timestamp.to_string()),
            ("msg[content]", &content_json.to_string()),
            ("csrf", &csrf),
            ("csrf_token", &csrf),
        ])
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let text = resp.text().await.map_err(|e| format!("读取失败: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("解析失败: {} | body={}", e, &text[..text.len().min(300)]))?;

    if json["code"] != 0 {
        log::error!("私信API响应(send_msg): {}", &text[..text.len().min(300)]);
        return Err(format!("发送私信失败: {}", json["message"]));
    }
    Ok(())
}

/// 私信处理器
pub struct DirectMessageHandler;

impl DirectMessageHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageHandler for DirectMessageHandler {
    fn name(&self) -> &'static str {
        "私信处理器"
    }

    fn source_type(&self) -> MsgSource {
        MsgSource::DirectMessage
    }

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String> {
        let resp = get_http_client()
            .get("https://api.vc.bilibili.com/session_svr/v1/session_svr/get_sessions")
            .header("Cookie", &account.cookie)
            .header("Referer", "https://message.bilibili.com/")
            .header("Accept", "application/json, text/plain, */*")
            .query(&[
                ("session_type", "1"),
                ("group_fold", "1"),
                ("unfollow_fold", "0"),
                ("sort_rule", "2"),
                ("build", "0"),
                ("mobi_app", "web"),
            ])
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let json = resp_to_json(resp).await?;

        if json["code"] != 0 {
            return Err(format!("获取私信会话失败: {}", json["message"]));
        }

        let empty = vec![];
        let sessions = json["data"]["session_list"].as_array().unwrap_or(&empty);
        let mut messages = Vec::new();

        for session in sessions {
            let talker_id = session["talker_id"].as_i64().unwrap_or(0);
            let unread_count = session["unread_count"].as_i64().unwrap_or(0);
            let name = talker_id.to_string();
            let uid = talker_id.to_string();

            if talker_id == 0 { continue; }
            if unread_count == 0 { continue; }

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
        send_dm(account, &message.user_id, reply_msg).await
    }
}

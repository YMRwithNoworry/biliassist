use super::handler::{generate_dev_id, Message, MessageHandler};
use super::http::{extract_csrf, get_http_client, resp_to_json};
use super::models::MsgSource;
use crate::bilibili::UserInfo;
use async_trait::async_trait;

/// 发送私信（公共函数，供关注处理器复用）
pub async fn send_dm(account: &UserInfo, uid: &str, msg: &str) -> Result<(), String> {
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

async fn mark_session_read(
    account: &UserInfo,
    talker_id: &str,
    ack_seqno: &str,
) -> Result<(), String> {
    let csrf = extract_csrf(&account.cookie);
    if csrf.is_empty() {
        return Err("cookie 中缺少 bili_jct (CSRF token)".to_string());
    }

    let resp = get_http_client()
        .post("https://api.vc.bilibili.com/session_svr/v1/session_svr/update_ack")
        .header("Cookie", &account.cookie)
        .header("Referer", "https://message.bilibili.com/")
        .header("Origin", "https://message.bilibili.com")
        .form(&[
            ("talker_id", talker_id),
            ("session_type", "1"),
            ("ack_seqno", ack_seqno),
            ("csrf", csrf.as_str()),
            ("csrf_token", csrf.as_str()),
            ("build", "0"),
            ("mobi_app", "web"),
        ])
        .send()
        .await
        .map_err(|e| format!("确认私信已读请求失败: {}", e))?;

    let json = resp_to_json(resp).await?;
    if json["code"] != 0 {
        return Err(format!("确认私信已读失败: {}", json["message"]));
    }
    Ok(())
}

fn value_to_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_i64().map(|value| value.to_string()))
}

fn parse_message_content(last_message: &serde_json::Value) -> Option<String> {
    let raw = last_message["content"].as_str()?;
    let msg_type = last_message["msg_type"].as_i64().unwrap_or(0);

    let parsed = serde_json::from_str::<serde_json::Value>(raw).ok();
    let parsed_text = parsed.as_ref().and_then(|value| {
        value["content"]
            .as_str()
            .or_else(|| value["text"].as_str())
            .or_else(|| value["title"].as_str())
    });

    match msg_type {
        1 => Some(parsed_text.unwrap_or(raw).to_string()),
        2 => Some("[图片]".to_string()),
        6 => Some("[表情]".to_string()),
        _ => parsed_text.map(ToOwned::to_owned),
    }
}

fn is_automated_message_source(msg_source: i64) -> bool {
    matches!(msg_source, 5 | 6 | 8..=19)
}

fn session_to_message(session: &serde_json::Value, account_uid: &str) -> Option<Message> {
    if session["session_type"].as_i64().unwrap_or(1) != 1
        || session["system_msg_type"].as_i64().unwrap_or(0) != 0
        || session["unread_count"].as_i64().unwrap_or(0) <= 0
    {
        return None;
    }

    let talker_id = value_to_string(&session["talker_id"])?;
    if talker_id == "0" {
        return None;
    }

    let last_message = session.get("last_msg")?;
    if !last_message.is_object()
        || last_message["msg_status"].as_i64().unwrap_or(0) != 0
        || is_automated_message_source(last_message["msg_source"].as_i64().unwrap_or(0))
    {
        return None;
    }

    let sender_uid = value_to_string(&last_message["sender_uid"])?;
    if sender_uid == account_uid || sender_uid != talker_id {
        return None;
    }

    let msg_key = value_to_string(&last_message["msg_key"])?;
    let msg_seqno = value_to_string(&last_message["msg_seqno"])?;

    Some(Message {
        id: msg_key,
        user_id: talker_id.clone(),
        user_name: talker_id.clone(),
        content: parse_message_content(last_message),
        extra_data: serde_json::json!({
            "talker_id": talker_id,
            "msg_seqno": msg_seqno,
        }),
    })
}

/// 私信处理器
#[derive(Default)]
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
                ("size", "100"),
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

        Ok(json["data"]["session_list"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|session| session_to_message(session, &account.uid))
            .collect())
    }

    async fn send_reply(
        &self,
        account: &UserInfo,
        message: &Message,
        reply_msg: &str,
    ) -> Result<(), String> {
        send_dm(account, &message.user_id, reply_msg).await?;

        let talker_id = message.extra_data["talker_id"].as_str();
        let msg_seqno = message.extra_data["msg_seqno"].as_str();
        if let (Some(talker_id), Some(msg_seqno)) = (talker_id, msg_seqno) {
            if let Err(error) = mark_session_read(account, talker_id, msg_seqno).await {
                log::warn!("私信已发送，但确认会话已读失败: {}", error);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inbound_session() -> serde_json::Value {
        serde_json::json!({
            "talker_id": 2,
            "session_type": 1,
            "system_msg_type": 0,
            "unread_count": 1,
            "last_msg": {
                "sender_uid": 2,
                "msg_type": 1,
                "content": "{\"content\":\"你好\"}",
                "msg_seqno": 1234567890123456u64,
                "msg_key": 7354295169819585966u64,
                "msg_status": 0,
                "msg_source": 7
            }
        })
    }

    #[test]
    fn parses_stable_message_identity_and_content() {
        let message = session_to_message(&inbound_session(), "1").unwrap();

        assert_eq!("7354295169819585966", message.id);
        assert_eq!("2", message.user_id);
        assert_eq!(Some("你好"), message.content.as_deref());
        assert_eq!("1234567890123456", message.extra_data["msg_seqno"]);
    }

    #[test]
    fn ignores_messages_sent_by_the_active_account() {
        let mut session = inbound_session();
        session["last_msg"]["sender_uid"] = serde_json::json!(1);

        assert!(session_to_message(&session, "1").is_none());
    }

    #[test]
    fn ignores_automated_or_read_sessions() {
        let mut automated = inbound_session();
        automated["last_msg"]["msg_source"] = serde_json::json!(9);
        assert!(session_to_message(&automated, "1").is_none());

        let mut read = inbound_session();
        read["unread_count"] = serde_json::json!(0);
        assert!(session_to_message(&read, "1").is_none());
    }
}

use super::handler::{MessageHandler, Message};
use super::http::{get_http_client, resp_to_json, extract_csrf};
use super::models::MsgSource;
use super::wbi;
use crate::bilibili::UserInfo;
use async_trait::async_trait;

/// 评论处理器
pub struct CommentHandler;

impl CommentHandler {
    pub fn new() -> Self {
        Self
    }

    async fn get_videos(&self, account: &UserInfo) -> Result<Vec<u64>, String> {
        let mut videos = Vec::new();
        let mut page = 1u32;

        let (img_key, sub_key) = match wbi::get_wbi_keys(&account.cookie).await {
            Ok(keys) => keys,
            Err(e) => {
                log::warn!("获取WBI密钥失败，尝试使用旧API: {}", e);
                return self.get_videos_fallback(account).await;
            }
        };

        let base_params = vec![
            ("mid".to_string(), account.uid.clone()),
            ("ps".to_string(), "50".to_string()),
            ("order".to_string(), "pubdate".to_string()),
        ];

        while page <= 5 {
            let pn = page.to_string();
            let mut params = base_params.clone();
            params.push(("pn".to_string(), pn));

            let signed = wbi::sign_wbi_params(&params, &img_key, &sub_key);

            let resp = get_http_client()
                .get("https://api.bilibili.com/x/space/wbi/arc/search")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://space.bilibili.com/{}/video", account.uid))
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .query(&signed.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>())
                .send()
                .await
                .map_err(|e| format!("请求视频列表失败: {}", e))?;

            let json = resp_to_json(resp).await?;

            if json["code"] != 0 {
                if page == 1 {
                    log::warn!("获取视频列表返回: code={}, msg={}", json["code"], json["message"]);
                }
                break;
            }

            let empty = vec![];
            let vlist = json["data"]["list"]["vlist"].as_array().or_else(|| {
                json["data"]["list"]["vms"].as_array()
            }).unwrap_or(&empty);

            if vlist.is_empty() { break; }

            for v in vlist {
                if let Some(aid) = v["aid"].as_u64().or_else(|| v["aid"].as_str().and_then(|s| s.parse().ok())) {
                    videos.push(aid);
                }
            }

            let count = json["data"]["page"]["count"].as_u64().unwrap_or(0);
            let pn_val = json["data"]["page"]["pn"].as_u64().unwrap_or(1);
            let ps_val = json["data"]["page"]["ps"].as_u64().unwrap_or(50);
            if pn_val * ps_val >= count { break; }

            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        log::info!("获取到 {} 个视频", videos.len());
        Ok(videos)
    }

    /// 旧API降级方案
    async fn get_videos_fallback(&self, account: &UserInfo) -> Result<Vec<u64>, String> {
        let mut videos = Vec::new();
        let mut page = 1u32;

        while page <= 5 {
            let pn = page.to_string();
            let ps = "50".to_string();
            let order = "pubdate".to_string();

            let resp = get_http_client()
                .get("https://api.bilibili.com/x/space/arc/search")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://space.bilibili.com/{}/video", account.uid))
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .query(&[
                    ("mid", &account.uid),
                    ("pn", &pn),
                    ("ps", &ps),
                    ("order", &order),
                ])
                .send()
                .await
                .map_err(|e| format!("请求视频列表失败: {}", e))?;

            let json = resp_to_json(resp).await?;

            if json["code"] != 0 {
                if page == 1 {
                    log::warn!("获取视频列表返回: code={}, msg={}", json["code"], json["message"]);
                }
                break;
            }

            let empty = vec![];
            let vlist = json["data"]["list"]["vlist"].as_array().unwrap_or(&empty);
            if vlist.is_empty() { break; }

            for v in vlist {
                if let Some(aid) = v["aid"].as_u64() {
                    videos.push(aid);
                }
            }

            let count = json["data"]["page"]["count"].as_u64().unwrap_or(0);
            let pn_val = json["data"]["page"]["pn"].as_u64().unwrap_or(1);
            let ps_val = json["data"]["page"]["ps"].as_u64().unwrap_or(50);
            if pn_val * ps_val >= count { break; }

            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        log::info!("获取到 {} 个视频", videos.len());
        Ok(videos)
    }

    /// 判断该评论是否已被UP主回复
    fn has_up_replied(reply: &serde_json::Value, my_mid: i64) -> bool {
        // 1. up_action.reply 字段：B站官方标记
        if reply["up_action"]["reply"].as_bool().unwrap_or(false) {
            return true;
        }

        // 2. reply_control.reply 字段
        if reply["reply_control"]["reply"].as_bool().unwrap_or(false) {
            return true;
        }

        // 3. 检查楼中楼里是否有自己的回复
        if let Some(subs) = reply["replies"].as_array() {
            if subs.iter().any(|r| r["mid"].as_i64().unwrap_or(0) == my_mid) {
                return true;
            }
        }

        false
    }

    /// 按时间分页获取评论（x/v2/reply，无需WBI）
    async fn get_unreplied_pn(&self, account: &UserInfo, aid: u64, my_mid: i64) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();
        let mut pn = 1u32;

        while pn <= 20 {
            let aid_s = aid.to_string();
            let pn_s = pn.to_string();

            let resp = get_http_client()
                .get("https://api.bilibili.com/x/v2/reply")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://www.bilibili.com/video/av{}", aid))
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .query(&[
                    ("type", "1"),
                    ("oid", &aid_s),
                    ("sort", "0"),
                    ("ps", "20"),
                    ("pn", &pn_s),
                    ("nohot", "1"),
                ])
                .send()
                .await
                .map_err(|e| format!("请求评论失败: {}", e))?;

            let json = resp_to_json(resp).await?;

            if json["code"] != 0 {
                log::warn!("获取评论返回: code={}, msg={}", json["code"], json["message"]);
                break;
            }

            let empty = vec![];
            let replies = json["data"]["replies"].as_array().unwrap_or(&empty);
            log::info!("视频 aid={} 第{}页获取到{}条评论", aid, pn, replies.len());
            if replies.is_empty() { break; }

            for reply in replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();

                if mid == my_mid { continue; }
                if rpid == 0 { continue; }

                if Self::has_up_replied(reply, my_mid) {
                    log::debug!("评论 rpid={} 已回复，跳过", rpid);
                    continue;
                }

                log::info!("找到未回复评论: rpid={}, mid={}, nickname={}", rpid, mid, nickname);
                let extra = serde_json::json!({ "aid": aid, "rpid": rpid });
                messages.push(Message {
                    id: format!("{}:{}", aid, rpid),
                    user_id: mid.to_string(),
                    user_name: nickname.clone(),
                    content: None,
                    extra_data: extra,
                });
            }

            let page_info = &json["data"]["page"];
            let count = page_info["count"].as_u64().unwrap_or(0);
            let size = page_info["size"].as_u64().unwrap_or(20);
            if (pn as u64) * size >= count { break; }

            pn += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(messages)
    }

    /// 按时间游标获取评论（x/v2/reply/main + WBI）
    async fn get_unreplied_cursor(&self, account: &UserInfo, aid: u64, my_mid: i64, wbi_keys: &(String, String)) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();
        let mut next: i64 = 0;
        let mut page = 0u32;
        let (ref img_key, ref sub_key) = wbi_keys;

        while page < 10 {
            let aid_s = aid.to_string();
            let next_s = next.to_string();

            let params = vec![
                ("type".to_string(), "1".to_string()),
                ("oid".to_string(), aid_s),
                ("mode".to_string(), "2".to_string()),
                ("ps".to_string(), "20".to_string()),
                ("next".to_string(), next_s),
            ];

            let signed = wbi::sign_wbi_params(&params, img_key, sub_key);

            let resp = get_http_client()
                .get("https://api.bilibili.com/x/v2/reply/main")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://www.bilibili.com/video/av{}", aid))
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .query(&signed.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>())
                .send()
                .await
                .map_err(|e| format!("请求评论失败: {}", e))?;

            let json = resp_to_json(resp).await?;

            if json["code"] != 0 {
                log::warn!("获取评论返回: code={}, msg={}", json["code"], json["message"]);
                break;
            }

            let empty = vec![];
            let replies = json["data"]["replies"].as_array().unwrap_or(&empty);
            log::info!("视频 aid={} 游标分页{}获取到{}条评论", aid, page, replies.len());
            if replies.is_empty() { break; }

            for reply in replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();

                if mid == my_mid { continue; }
                if rpid == 0 { continue; }

                if Self::has_up_replied(reply, my_mid) {
                    continue;
                }

                log::info!("找到未回复评论: rpid={}, mid={}, nickname={}", rpid, mid, nickname);
                let extra = serde_json::json!({ "aid": aid, "rpid": rpid });
                messages.push(Message {
                    id: format!("{}:{}", aid, rpid),
                    user_id: mid.to_string(),
                    user_name: nickname.clone(),
                    content: None,
                    extra_data: extra,
                });
            }

            let is_end = json["data"]["cursor"]["is_end"].as_bool().unwrap_or(true);
            if is_end { break; }

            next = json["data"]["cursor"]["next"].as_i64().unwrap_or(0);
            if next == 0 { break; }

            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(messages)
    }

    async fn get_unreplied(&self, account: &UserInfo, aid: u64) -> Result<Vec<Message>, String> {
        let my_mid = account.uid.parse::<i64>().unwrap_or(0);
        log::info!("开始获取视频 aid={} 的评论，my_mid={}", aid, my_mid);

        // 主路径：x/v2/reply（pn分页，按时间排序，无需WBI）
        let messages = self.get_unreplied_pn(account, aid, my_mid).await;
        if let Ok(ref m) = messages {
            if !m.is_empty() {
                log::info!("视频 aid={} 按时间API找到 {} 条未回复评论", aid, m.len());
                return messages;
            }
        }

        // 降级路径：x/v2/reply/main（游标分页 + WBI签名）
        if let Ok(wbi_keys) = wbi::get_wbi_keys(&account.cookie).await {
            let cursor_msgs = self.get_unreplied_cursor(account, aid, my_mid, &wbi_keys).await?;
            log::info!("视频 aid={} 按游标API找到 {} 条未回复评论", aid, cursor_msgs.len());
            return Ok(cursor_msgs);
        }

        messages
    }

    async fn reply_to_comment(
        &self,
        account: &UserInfo,
        aid: u64,
        rpid: u64,
        message: &str,
    ) -> Result<(), String> {
        let csrf = extract_csrf(&account.cookie);
        if csrf.is_empty() {
            return Err("未找到 CSRF token (bili_jct)，请重新登录".into());
        }

        let oid_s = aid.to_string();
        let rpid_s = rpid.to_string();

        log::info!("准备回复评论: aid={}, rpid={}, message_len={}", aid, rpid, message.len());

        let resp = get_http_client()
            .post("https://api.bilibili.com/x/v2/reply/add")
            .header("Cookie", &account.cookie)
            .header("Referer", format!("https://www.bilibili.com/video/av{}", aid))
            .header("Origin", "https://www.bilibili.com")
            .header("Accept", "application/json, text/plain, */*")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .form(&[
                ("type", "1"),
                ("oid", &oid_s),
                ("message", message),
                ("root", &rpid_s),
                ("parent", &rpid_s),
                ("plat", "1"),
                ("csrf", &csrf),
                ("csrf_token", &csrf),
            ])
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let json = resp_to_json(resp).await?;
        log::info!("评论回复API响应: code={}", json["code"]);

        if json["code"] != 0 {
            let msg = json["message"].as_str().unwrap_or("未知错误");
            log::error!("评论回复失败: code={}, msg={}", json["code"], msg);
            return Err(format!("回复评论失败: {}", msg));
        }

        log::info!("评论回复成功: aid={}, rpid={}", aid, rpid);
        Ok(())
    }
}

#[async_trait]
impl MessageHandler for CommentHandler {
    fn name(&self) -> &'static str {
        "评论处理器"
    }

    fn source_type(&self) -> MsgSource {
        MsgSource::Comment
    }

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String> {
        let videos = self.get_videos(account).await?;
        if videos.is_empty() {
            log::info!("没有找到发布的视频");
            return Ok(Vec::new());
        }

        log::info!("开始遍历 {} 个视频的评论", videos.len());

        let mut all_messages = Vec::new();
        for aid in videos {
            match self.get_unreplied(account, aid).await {
                Ok(msgs) => {
                    if !msgs.is_empty() {
                        log::info!("视频 aid={} 有 {} 条未回复评论", aid, msgs.len());
                    }
                    all_messages.extend(msgs);
                }
                Err(e) => {
                    log::warn!("获取视频 {} 评论失败: {}", aid, e);
                    continue;
                }
            }
        }

        log::info!("共计找到 {} 条未回复评论", all_messages.len());
        Ok(all_messages)
    }

    async fn send_reply(&self, account: &UserInfo, message: &Message, reply_msg: &str) -> Result<(), String> {
        let aid = message.extra_data["aid"].as_u64().ok_or("缺少aid参数")?;
        let rpid = message.extra_data["rpid"].as_u64().ok_or("缺少rpid参数")?;
        self.reply_to_comment(account, aid, rpid, reply_msg).await
    }
}

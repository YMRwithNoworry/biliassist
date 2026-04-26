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

        // 获取WBI密钥
        let (img_key, sub_key) = match wbi::get_wbi_keys(&account.cookie).await {
            Ok(keys) => keys,
            Err(e) => {
                log::warn!("获取WBI密钥失败，尝试使用旧API: {}", e);
                return self.get_videos_fallback(account).await;
            }
        };

        let base_params = vec![
            ("mid".to_string(), account.uid.clone()),
            ("ps".to_string(), "30".to_string()),
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
            let ps_val = json["data"]["page"]["ps"].as_u64().unwrap_or(30);
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
            let ps = "30".to_string();
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
            let ps_val = json["data"]["page"]["ps"].as_u64().unwrap_or(30);
            if pn_val * ps_val >= count { break; }

            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        log::info!("获取到 {} 个视频", videos.len());
        Ok(videos)
    }

    async fn get_unreplied(&self, account: &UserInfo, aid: u64) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();
        let mut next: i64 = 1;
        let mut page = 0u32;
        let my_mid = account.uid.parse::<i64>().unwrap_or(0);

        log::info!("开始获取视频 aid={} 的评论，my_mid={}", aid, my_mid);

        // 获取WBI密钥用于签名
        let wbi_keys = wbi::get_wbi_keys(&account.cookie).await.ok();

        while page < 10 {
            let aid_s = aid.to_string();
            let next_s = next.to_string();

            let req = get_http_client()
                .get("https://api.bilibili.com/x/v2/reply/main")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://www.bilibili.com/video/av{}", aid))
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

            // 构建基础参数
            let mut params = vec![
                ("type".to_string(), "1".to_string()),
                ("oid".to_string(), aid_s),
                ("mode".to_string(), "3".to_string()),
                ("ps".to_string(), "20".to_string()),
                ("next".to_string(), next_s),
            ];

            // 如果有WBI密钥，对参数进行签名
            if let Some((ref img_key, ref sub_key)) = wbi_keys {
                params = wbi::sign_wbi_params(&params, img_key, sub_key);
            }

            let resp = req
                .query(&params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>())
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
            log::info!("视频 aid={} 第 {} 页获取到 {} 条评论", aid, page, replies.len());
            if replies.is_empty() { break; }

            for reply in replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();

                log::debug!("评论: rpid={}, mid={}, nickname={}, my_mid={}", rpid, mid, nickname, my_mid);

                if mid == my_mid { continue; }

                let has_up_reply = reply["replies"].as_array()
                    .map(|subs| subs.iter().any(|r| r["mid"].as_i64().unwrap_or(0) == my_mid))
                    .unwrap_or(false);

                let up_action_reply = reply["up_action"]["reply"].as_bool().unwrap_or(false);

                let reply_control_reply = reply["reply_control"]["reply"].as_bool().unwrap_or(false);

                log::debug!("评论 rpid={}: has_up_reply={}, up_action_reply={}, reply_control_reply={}",
                    rpid, has_up_reply, up_action_reply, reply_control_reply);

                if !has_up_reply && !up_action_reply && !reply_control_reply {
                    log::info!("找到未回复评论: rpid={}, mid={}, nickname={}", rpid, mid, nickname);
                    let extra = serde_json::json!({
                        "aid": aid,
                        "rpid": rpid,
                    });
                    messages.push(Message {
                        id: format!("{}:{}", aid, rpid),
                        user_id: mid.to_string(),
                        user_name: nickname.clone(),
                        content: None,
                        extra_data: extra,
                    });
                }
            }

            // 检查是否还有下一页
            let is_end = json["data"]["cursor"]["is_end"].as_bool().unwrap_or(true);
            if is_end {
                log::info!("视频 aid={} 评论已全部加载完毕", aid);
                break;
            }

            next = json["data"]["cursor"]["next"].as_i64().unwrap_or(0);
            if next == 0 { break; }

            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        log::info!("视频 aid={} 共找到 {} 条未回复评论", aid, messages.len());
        Ok(messages)
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
        log::info!("评论回复API响应: {}", json.to_string());

        if json["code"] != 0 {
            log::error!("评论回复失败: code={}, msg={}", json["code"], json["message"]);
            return Err(format!("回复评论失败: {}", json["message"]));
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

        let mut all_messages = Vec::new();
        for aid in videos {
            match self.get_unreplied(account, aid).await {
                Ok(msgs) => {
                    log::info!("视频 aid={} 有 {} 条未回复评论", aid, msgs.len());
                    all_messages.extend(msgs);
                }
                Err(e) => {
                    log::warn!("获取视频 {} 评论失败: {}", aid, e);
                    continue;
                }
            }
        }

        Ok(all_messages)
    }

    async fn send_reply(&self, account: &UserInfo, message: &Message, reply_msg: &str) -> Result<(), String> {
        let aid = message.extra_data["aid"].as_u64().ok_or("缺少aid参数")?;
        let rpid = message.extra_data["rpid"].as_u64().ok_or("缺少rpid参数")?;
        self.reply_to_comment(account, aid, rpid, reply_msg).await
    }
}

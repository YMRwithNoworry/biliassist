use super::handler::{MessageHandler, Message};
use super::http::{get_http_client, resp_to_json, extract_csrf};
use super::models::MsgSource;
use super::wbi;
use crate::bilibili::UserInfo;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

const ACCEPT_JSON: &str = "application/json, text/plain, */*";

pub struct CommentHandler {
    wbi_cache: Arc<Mutex<Option<(String, String)>>>,
}

impl CommentHandler {
    pub fn new() -> Self {
        Self {
            wbi_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn browser_headers(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Accept", ACCEPT_JSON)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    }

    async fn ensure_wbi_keys(&self, account: &UserInfo) -> Result<(String, String), String> {
        {
            let cache = self.wbi_cache.lock().await;
            if let Some(ref keys) = *cache {
                return Ok(keys.clone());
            }
        }
        let keys = wbi::get_wbi_keys(&account.cookie).await?;
        let mut cache = self.wbi_cache.lock().await;
        *cache = Some(keys.clone());
        Ok(keys)
    }

    async fn get_videos(&self, account: &UserInfo) -> Result<Vec<u64>, String> {
        let wbi_keys = self.ensure_wbi_keys(account).await;
        if let Ok(ref keys) = wbi_keys {
            let result = self.get_videos_wbi(account, keys).await;
            if let Ok(ref videos) = result {
                if !videos.is_empty() {
                    return result;
                }
                log::warn!("WBI签名API返回空视频列表，降级到旧API");
            }
        }
        self.get_videos_fallback(account).await
    }

    async fn get_videos_wbi(&self, account: &UserInfo, keys: &(String, String)) -> Result<Vec<u64>, String> {
        let mut videos = Vec::new();
        let (ref img_key, ref sub_key) = keys;
        let mut page = 1u32;

        while page <= 5 {
            let mut params = vec![
                ("mid".to_string(), account.uid.clone()),
                ("ps".to_string(), "50".to_string()),
                ("order".to_string(), "pubdate".to_string()),
                ("pn".to_string(), page.to_string()),
            ];
            params = wbi::sign_wbi_params(&params, img_key, sub_key);

            let resp = Self::browser_headers(get_http_client()
                .get("https://api.bilibili.com/x/space/wbi/arc/search")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://space.bilibili.com/{}/video", account.uid)))
                .query(&params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>())
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

            let vlist = json["data"]["list"]["vlist"].as_array()
                .or_else(|| json["data"]["list"]["vms"].as_array())
                .map(|a| a.to_vec())
                .unwrap_or_default();

            if vlist.is_empty() { break; }

            for v in &vlist {
                if let Some(aid) = v["aid"].as_u64().or_else(|| v["aid"].as_str().and_then(|s| s.parse().ok())) {
                    videos.push(aid);
                }
            }

            let page_info = &json["data"]["page"];
            let count = page_info["count"].as_u64().unwrap_or(0);
            let pn_val = page_info["pn"].as_u64().unwrap_or(1);
            let ps_val = page_info["ps"].as_u64().unwrap_or(50);
            if pn_val * ps_val >= count { break; }

            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        log::info!("获取到 {} 个视频", videos.len());
        Ok(videos)
    }

    async fn get_videos_fallback(&self, account: &UserInfo) -> Result<Vec<u64>, String> {
        let mut videos = Vec::new();
        let mut page = 1u32;

        while page <= 5 {
            let pn = page.to_string();

            let resp = Self::browser_headers(get_http_client()
                .get("https://api.bilibili.com/x/space/arc/search")
                .header("Cookie", &account.cookie)
                .header("Referer", format!("https://space.bilibili.com/{}/video", account.uid)))
                .query(&[("mid", account.uid.as_str()), ("pn", &pn), ("ps", "50"), ("order", "pubdate")])
                .send()
                .await
                .map_err(|e| format!("请求视频列表失败: {}", e))?;

            let json = resp_to_json(resp).await?;
            if json["code"] != 0 { break; }

            let vlist = json["data"]["list"]["vlist"].as_array()
                .map(|a| a.to_vec())
                .unwrap_or_default();

            if vlist.is_empty() { break; }

            for v in &vlist {
                if let Some(aid) = v["aid"].as_u64() {
                    videos.push(aid);
                }
            }

            let count = json["data"]["page"]["count"].as_u64().unwrap_or(0);
            if (page as u64) * 50 >= count { break; }
            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        Ok(videos)
    }

    fn is_replied(reply: &serde_json::Value, my_mid: i64) -> bool {
        if let Some(subs) = reply["replies"].as_array() {
            if subs.iter().any(|r| r["mid"].as_i64().unwrap_or(0) == my_mid) { return true; }
        }
        false
    }

    async fn get_comments(&self, account: &UserInfo, aid: u64, my_mid: i64) -> Result<Vec<Message>, String> {
        log::info!("获取视频 aid={} 的评论", aid);

        if let Ok(wbi_keys) = self.ensure_wbi_keys(account).await {
            let msg = self.get_comments_cursor(account, aid, my_mid, &wbi_keys).await;
            if let Ok(ref m) = msg {
                if !m.is_empty() {
                    log::info!("aid={} 找到 {} 条未回复评论", aid, m.len());
                    return msg;
                }
            }
        }

        self.get_comments_pn(account, aid, my_mid).await
    }

    async fn fetch_json(&self, url: &str, params: &[(&str, &str)], cookie: &str, referer: &str) -> Result<serde_json::Value, String> {
        let try_fetch = |delay: u64| async move {
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
            let resp = Self::browser_headers(get_http_client()
                .get(url)
                .header("Cookie", cookie)
                .header("Referer", referer))
                .query(params)
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;
            let text = resp.text().await.map_err(|e| format!("读取失败: {}", e))?;
            if text.trim_start().starts_with("<!DOCTYPE") || text.trim_start().starts_with("<html") {
                return Err("HTML_RESPONSE".to_string());
            }
            serde_json::from_str(&text).map_err(|e| format!("解析JSON失败: {}", e))
        };

        let mut result = try_fetch(0).await;
        if let Err(ref e) = result {
            if e == "HTML_RESPONSE" {
                log::warn!("收到限流HTML，等待3秒重试");
                result = try_fetch(3000).await;
                if let Err(ref e2) = result {
                    if e2 == "HTML_RESPONSE" {
                        return Err("B站限流返回HTML".to_string());
                    }
                }
            }
        }
        result
    }

    async fn get_comments_cursor(
        &self, account: &UserInfo, aid: u64, my_mid: i64, keys: &(String, String),
    ) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();
        let (ref img_key, ref sub_key) = keys;
        let mut next: i64 = 0;

        for page in 0..30u32 {
            let mut params = vec![
                ("type".to_string(), "1".to_string()),
                ("oid".to_string(), aid.to_string()),
                ("mode".to_string(), "2".to_string()),
                ("ps".to_string(), "30".to_string()),
                ("next".to_string(), next.to_string()),
            ];
            params = wbi::sign_wbi_params(&params, img_key, sub_key);
            let param_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let url = "https://api.bilibili.com/x/v2/reply/main";
            let referer = format!("https://www.bilibili.com/video/av{}", aid);

            let json = self.fetch_json(url, &param_refs, &account.cookie, &referer).await?;

            if json["code"] != 0 {
                log::info!("评论API code={}, msg={}", json["code"], json["message"]);
                break;
            }

            let replies = json["data"]["replies"].as_array()
                .map(|a| a.to_vec())
                .unwrap_or_default();
            let count = replies.len();
            log::info!("aid={} 游标第{}页: {}条", aid, page, count);
            if count == 0 { break; }

            let mut filtered = 0u32;
            for reply in &replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();
                if mid == my_mid || rpid == 0 { continue; }
                if Self::is_replied(reply, my_mid) { filtered += 1; continue; }

                messages.push(Message {
                    id: format!("{}:{}", aid, rpid),
                    user_id: mid.to_string(),
                    user_name: nickname,
                    content: None,
                    extra_data: serde_json::json!({ "aid": aid, "rpid": rpid }),
                });
            }
            log::info!("aid={} 第{}页: {}条通过过滤, {}条已回复", aid, page, count.saturating_sub(filtered as usize), filtered);

            if json["data"]["cursor"]["is_end"].as_bool().unwrap_or(true) { break; }
            next = json["data"]["cursor"]["next"].as_i64().unwrap_or(0);
            if next == 0 { break; }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        log::info!("aid={} 游标API共找到 {} 条未回复评论", aid, messages.len());
        Ok(messages)
    }

    async fn get_comments_pn(&self, account: &UserInfo, aid: u64, my_mid: i64) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();

        for pn in 1..=30u32 {
            let aid_s = aid.to_string();
            let pn_s = pn.to_string();
            let params = &[("type", "1"), ("oid", aid_s.as_str()), ("sort", "0"), ("ps", "20"), ("pn", pn_s.as_str()), ("nohot", "1")];
            let referer = format!("https://www.bilibili.com/video/av{}", aid);

            let json = self.fetch_json("https://api.bilibili.com/x/v2/reply", params, &account.cookie, &referer).await?;

            if json["code"] != 0 { break; }

            let replies = json["data"]["replies"].as_array()
                .map(|a| a.to_vec())
                .unwrap_or_default();
            if replies.is_empty() { break; }

            let mut filtered = 0u32;
            for reply in &replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();
                if mid == my_mid || rpid == 0 { continue; }
                if Self::is_replied(reply, my_mid) { filtered += 1; continue; }

                messages.push(Message {
                    id: format!("{}:{}", aid, rpid),
                    user_id: mid.to_string(),
                    user_name: nickname,
                    content: None,
                    extra_data: serde_json::json!({ "aid": aid, "rpid": rpid }),
                });
            }
            if filtered > 0 {
                log::debug!("aid={} pn第{}页: 过滤{}条已回复评论", aid, pn, filtered);
            }

            let count = json["data"]["page"]["count"].as_u64().unwrap_or(0);
            if (pn as u64) * 20 >= count { break; }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(messages)
    }

    async fn reply_to_comment(&self, account: &UserInfo, aid: u64, rpid: u64, message: &str) -> Result<(), String> {
        let csrf = extract_csrf(&account.cookie);
        if csrf.is_empty() {
            return Err("未找到 CSRF token".into());
        }

        let resp = get_http_client()
            .post("https://api.bilibili.com/x/v2/reply/add")
            .header("Cookie", &account.cookie)
            .header("Referer", format!("https://www.bilibili.com/video/av{}", aid))
            .header("Origin", "https://www.bilibili.com")
            .header("Accept", ACCEPT_JSON)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .form(&[
                ("type", "1"),
                ("oid", &aid.to_string()),
                ("message", message),
                ("root", &rpid.to_string()),
                ("parent", &rpid.to_string()),
                ("plat", "1"),
                ("csrf", &csrf),
                ("csrf_token", &csrf),
            ])
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let json = resp_to_json(resp).await?;
        log::info!("回复API: code={}", json["code"]);

        if json["code"] != 0 {
            let msg = json["message"].as_str().unwrap_or("未知");
            return Err(format!("回复评论失败: {}", msg));
        }
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
        {
            let mut cache = self.wbi_cache.lock().await;
            *cache = None;
        }

        let videos = self.get_videos(account).await?;
        log::info!("共获取到 {} 个视频，开始检查评论", videos.len());
        if videos.is_empty() {
            return Ok(Vec::new());
        }

        let my_mid = account.uid.parse::<i64>().unwrap_or(0);
        let mut all = Vec::new();
        // 每次最多处理10个视频，避免超时
        let max_videos = videos.len().min(10);
        let mut processed = 0u32;

        for aid in &videos[..max_videos] {
            match self.get_comments(account, *aid, my_mid).await {
                Ok(msgs) => {
                    if !msgs.is_empty() {
                        log::info!("视频 aid={} 有 {} 条未回复评论", aid, msgs.len());
                    }
                    all.extend(msgs);
                    processed += 1;
                }
                Err(e) => log::warn!("获取视频 {} 评论失败: {}", aid, e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }

        log::info!("本轮处理了 {} 个视频，共计 {} 条未回复评论", processed, all.len());
        Ok(all)
    }

    async fn send_reply(&self, account: &UserInfo, message: &Message, reply_msg: &str) -> Result<(), String> {
        let aid = message.extra_data["aid"].as_u64().ok_or("缺少aid")?;
        let rpid = message.extra_data["rpid"].as_u64().ok_or("缺少rpid")?;
        self.reply_to_comment(account, aid, rpid, reply_msg).await
    }
}

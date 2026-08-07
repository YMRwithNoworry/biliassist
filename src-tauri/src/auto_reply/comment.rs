use super::handler::{Message, MessageHandler};
use super::http::{extract_csrf, get_http_client, resp_to_json};
use super::models::MsgSource;
use super::state::AutoReplyState;
use super::wbi;
use crate::bilibili::UserInfo;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const ACCEPT_JSON: &str = "application/json, text/plain, */*";
const WBI_CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

type WbiKeys = (String, String);
type WbiCacheEntry = (WbiKeys, Instant);

#[derive(Debug, Clone, Copy)]
enum CommentKind {
    Video,
    Dynamic,
}

#[derive(Debug, Clone)]
struct CommentTarget {
    oid: u64,
    reply_type: u32,
    referer: String,
    label: &'static str,
}

pub struct CommentHandler {
    kind: CommentKind,
    wbi_cache: Arc<Mutex<Option<WbiCacheEntry>>>,
}

impl CommentHandler {
    pub fn new() -> Self {
        Self::with_kind(CommentKind::Video)
    }

    pub fn dynamic() -> Self {
        Self::with_kind(CommentKind::Dynamic)
    }

    fn with_kind(kind: CommentKind) -> Self {
        Self {
            kind,
            wbi_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn browser_headers(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("Accept", ACCEPT_JSON)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    }

    async fn ensure_wbi_keys(&self, account: &UserInfo) -> Result<WbiKeys, String> {
        {
            let cache = self.wbi_cache.lock().await;
            if let Some((keys, cached_at)) = cache.as_ref() {
                if cached_at.elapsed() < WBI_CACHE_TTL {
                    return Ok(keys.clone());
                }
            }
        }
        let keys = wbi::get_wbi_keys(&account.cookie).await?;
        let mut cache = self.wbi_cache.lock().await;
        *cache = Some((keys.clone(), Instant::now()));
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

    async fn get_videos_wbi(&self, account: &UserInfo, keys: &WbiKeys) -> Result<Vec<u64>, String> {
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

            let resp = Self::browser_headers(
                get_http_client()
                    .get("https://api.bilibili.com/x/space/wbi/arc/search")
                    .header("Cookie", &account.cookie)
                    .header(
                        "Referer",
                        format!("https://space.bilibili.com/{}/video", account.uid),
                    ),
            )
            .query(
                &params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<_>>(),
            )
            .send()
            .await
            .map_err(|e| format!("请求视频列表失败: {}", e))?;

            let json = resp_to_json(resp).await?;
            if json["code"] != 0 {
                if page == 1 {
                    log::warn!(
                        "获取视频列表返回: code={}, msg={}",
                        json["code"],
                        json["message"]
                    );
                }
                break;
            }

            let vlist = json["data"]["list"]["vlist"]
                .as_array()
                .or_else(|| json["data"]["list"]["vms"].as_array())
                .map(|a| a.to_vec())
                .unwrap_or_default();

            if vlist.is_empty() {
                break;
            }

            for v in &vlist {
                if let Some(aid) = v["aid"]
                    .as_u64()
                    .or_else(|| v["aid"].as_str().and_then(|s| s.parse().ok()))
                {
                    videos.push(aid);
                }
            }

            let page_info = &json["data"]["page"];
            let count = page_info["count"].as_u64().unwrap_or(0);
            let pn_val = page_info["pn"].as_u64().unwrap_or(1);
            let ps_val = page_info["ps"].as_u64().unwrap_or(50);
            if pn_val * ps_val >= count {
                break;
            }

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

            let resp = Self::browser_headers(
                get_http_client()
                    .get("https://api.bilibili.com/x/space/arc/search")
                    .header("Cookie", &account.cookie)
                    .header(
                        "Referer",
                        format!("https://space.bilibili.com/{}/video", account.uid),
                    ),
            )
            .query(&[
                ("mid", account.uid.as_str()),
                ("pn", &pn),
                ("ps", "50"),
                ("order", "pubdate"),
            ])
            .send()
            .await
            .map_err(|e| format!("请求视频列表失败: {}", e))?;

            let json = resp_to_json(resp).await?;
            if json["code"] != 0 {
                break;
            }

            let vlist = json["data"]["list"]["vlist"]
                .as_array()
                .map(|a| a.to_vec())
                .unwrap_or_default();

            if vlist.is_empty() {
                break;
            }

            for v in &vlist {
                if let Some(aid) = v["aid"].as_u64() {
                    videos.push(aid);
                }
            }

            let count = json["data"]["page"]["count"].as_u64().unwrap_or(0);
            if (page as u64) * 50 >= count {
                break;
            }
            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        Ok(videos)
    }

    fn value_as_u64(value: &serde_json::Value) -> Option<u64> {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    }

    fn dynamic_id(item: &serde_json::Value, my_mid: i64) -> Option<u64> {
        let author_mid = Self::value_as_u64(&item["modules"]["module_author"]["mid"])
            .or_else(|| Self::value_as_u64(&item["card"]["desc"]["user_id"]))
            .or_else(|| Self::value_as_u64(&item["card"]["desc"]["uid"]))
            .or_else(|| Self::value_as_u64(&item["desc"]["user_id"]))
            .or_else(|| Self::value_as_u64(&item["desc"]["uid"]));
        if author_mid.is_some() && author_mid != u64::try_from(my_mid).ok() {
            return None;
        }

        Self::value_as_u64(&item["id"])
            .or_else(|| Self::value_as_u64(&item["id_str"]))
            .or_else(|| Self::value_as_u64(&item["dynamic_id"]))
            .or_else(|| Self::value_as_u64(&item["card"]["desc"]["dynamic_id"]))
            .or_else(|| Self::value_as_u64(&item["desc"]["dynamic_id"]))
    }

    async fn get_dynamics(&self, account: &UserInfo) -> Result<Vec<u64>, String> {
        let my_mid = account.uid.parse::<i64>().unwrap_or(0);
        let referer = format!("https://space.bilibili.com/{}/dynamic", account.uid);
        let params = [
            ("host_mid", account.uid.as_str()),
            ("page", "1"),
            ("offset", ""),
            ("features", "itemOpusStyle"),
        ];
        let mut dynamics = Vec::new();

        match Self::browser_headers(
            get_http_client()
                .get("https://api.bilibili.com/x/polymer/web-dynamic/v1/feed/all")
                .header("Cookie", &account.cookie)
                .header("Referer", &referer),
        )
        .query(&params)
        .send()
        .await
        {
            Ok(resp) => match resp_to_json(resp).await {
                Ok(json) if json["code"] == 0 => {
                    if let Some(items) = json["data"]["items"].as_array() {
                        for item in items {
                            if let Some(id) = Self::dynamic_id(item, my_mid) {
                                dynamics.push(id);
                            }
                        }
                    }
                }
                Ok(json) => log::warn!(
                    "获取动态列表返回: code={}, msg={}",
                    json["code"],
                    json["message"]
                ),
                Err(error) => log::warn!("解析动态列表失败: {}", error),
            },
            Err(error) => log::warn!("请求动态列表失败: {}", error),
        }

        if dynamics.is_empty() {
            let old_params = [
                ("host_uid", account.uid.as_str()),
                ("offset_dynamic_id", "0"),
                ("need_top", "1"),
                ("platform", "web"),
            ];
            let old_url = "https://api.vc.bilibili.com/dynamic_svr/v1/dynamic_svr/space_history";
            let json = self
                .fetch_json(old_url, &old_params, &account.cookie, &referer)
                .await?;
            if json["code"] == 0 {
                if let Some(cards) = json["data"]["cards"].as_array() {
                    for card in cards {
                        let item = if let Some(card_text) = card["card"].as_str() {
                            serde_json::from_str(card_text).unwrap_or_else(|_| card.clone())
                        } else {
                            card.clone()
                        };
                        if let Some(id) = Self::dynamic_id(card, my_mid)
                            .or_else(|| Self::dynamic_id(&item, my_mid))
                        {
                            dynamics.push(id);
                        }
                    }
                }
            }
        }

        dynamics.sort_unstable_by(|left, right| right.cmp(left));
        dynamics.dedup();
        Ok(dynamics)
    }

    fn is_replied(reply: &serde_json::Value, my_mid: i64) -> bool {
        if let Some(subs) = reply["replies"].as_array() {
            if subs
                .iter()
                .any(|r| r["mid"].as_i64().unwrap_or(0) == my_mid)
            {
                return true;
            }
        }
        false
    }

    async fn get_comments(
        &self,
        account: &UserInfo,
        target: &CommentTarget,
        my_mid: i64,
    ) -> Result<Vec<Message>, String> {
        log::info!("获取{} oid={} 的评论", target.label, target.oid);

        if let Ok(wbi_keys) = self.ensure_wbi_keys(account).await {
            match self
                .get_comments_cursor(account, target, my_mid, &wbi_keys)
                .await
            {
                Ok(messages) => {
                    if !messages.is_empty() {
                        log::info!("oid={} 找到 {} 条未回复评论", target.oid, messages.len());
                        return Ok(messages);
                    }
                    // 部分账号下 WBI 接口会错误地返回空列表，需用旧接口补抓。
                    log::warn!("oid={} WBI接口返回空评论，降级到旧分页接口", target.oid);
                }
                Err(error) => log::warn!("WBI评论接口失败，降级到旧接口: {}", error),
            }
        }

        self.get_comments_pn(account, target, my_mid).await
    }

    async fn fetch_json(
        &self,
        url: &str,
        params: &[(&str, &str)],
        cookie: &str,
        referer: &str,
    ) -> Result<serde_json::Value, String> {
        let try_fetch = |delay: u64| async move {
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
            let resp = Self::browser_headers(
                get_http_client()
                    .get(url)
                    .header("Cookie", cookie)
                    .header("Referer", referer),
            )
            .query(params)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
            let text = resp.text().await.map_err(|e| format!("读取失败: {}", e))?;
            if text.trim_start().starts_with("<!DOCTYPE") || text.trim_start().starts_with("<html")
            {
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
        &self,
        account: &UserInfo,
        target: &CommentTarget,
        my_mid: i64,
        keys: &WbiKeys,
    ) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();
        let (ref img_key, ref sub_key) = keys;
        let mut next: i64 = 0;

        for page in 0..30u32 {
            let mut params = vec![
                ("type".to_string(), target.reply_type.to_string()),
                ("oid".to_string(), target.oid.to_string()),
                ("mode".to_string(), "2".to_string()),
                ("ps".to_string(), "30".to_string()),
                ("next".to_string(), next.to_string()),
            ];
            params = wbi::sign_wbi_params(&params, img_key, sub_key);
            let param_refs: Vec<(&str, &str)> = params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let url = "https://api.bilibili.com/x/v2/reply/main";
            let json = self
                .fetch_json(url, &param_refs, &account.cookie, &target.referer)
                .await?;

            if json["code"] != 0 {
                log::info!("评论API code={}, msg={}", json["code"], json["message"]);
                if messages.is_empty() {
                    return Err(format!(
                        "评论API code={}, msg={}",
                        json["code"], json["message"]
                    ));
                }
                break;
            }

            let replies = json["data"]["replies"]
                .as_array()
                .map(|a| a.to_vec())
                .unwrap_or_default();
            let count = replies.len();
            log::info!("oid={} 游标第{}页: {}条", target.oid, page, count);
            if count == 0 {
                break;
            }

            let mut filtered = 0u32;
            for reply in &replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();
                if mid == my_mid || rpid == 0 {
                    continue;
                }
                let already_replied = Self::is_replied(reply, my_mid);
                if already_replied {
                    filtered += 1;
                }

                let comment_text = reply["content"]["message"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                messages.push(Message {
                    id: format!("{}:{}", target.oid, rpid),
                    user_id: mid.to_string(),
                    user_name: nickname,
                    content: if comment_text.is_empty() { None } else { Some(comment_text) },
                    extra_data: serde_json::json!({ "oid": target.oid, "aid": target.oid, "reply_type": target.reply_type, "referer": target.referer, "rpid": rpid, "already_replied": already_replied }),
                });
            }
            log::info!(
                "oid={} 第{}页: {}条通过过滤, {}条已回复",
                target.oid,
                page,
                count.saturating_sub(filtered as usize),
                filtered
            );

            if json["data"]["cursor"]["is_end"].as_bool().unwrap_or(true) {
                break;
            }
            next = json["data"]["cursor"]["next"].as_i64().unwrap_or(0);
            if next == 0 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        log::info!(
            "oid={} 游标API共找到 {} 条未回复评论",
            target.oid,
            messages.len()
        );
        Ok(messages)
    }

    async fn get_comments_pn(
        &self,
        account: &UserInfo,
        target: &CommentTarget,
        my_mid: i64,
    ) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();

        for pn in 1..=30u32 {
            let oid_s = target.oid.to_string();
            let reply_type_s = target.reply_type.to_string();
            let pn_s = pn.to_string();
            let params: &[(&str, &str)] = &[
                ("type", reply_type_s.as_str()),
                ("oid", oid_s.as_str()),
                ("sort", "0"),
                ("ps", "20"),
                ("pn", pn_s.as_str()),
                ("nohot", "1"),
            ];
            let json = self
                .fetch_json(
                    "https://api.bilibili.com/x/v2/reply",
                    params,
                    &account.cookie,
                    &target.referer,
                )
                .await?;

            if json["code"] != 0 {
                break;
            }

            let replies = json["data"]["replies"]
                .as_array()
                .map(|a| a.to_vec())
                .unwrap_or_default();
            if replies.is_empty() {
                break;
            }

            let mut filtered = 0u32;
            for reply in &replies {
                let rpid = reply["rpid"].as_u64().unwrap_or(0);
                let mid = reply["mid"].as_i64().unwrap_or(0);
                let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();
                if mid == my_mid || rpid == 0 {
                    continue;
                }
                let already_replied = Self::is_replied(reply, my_mid);
                if already_replied {
                    filtered += 1;
                }

                let comment_text = reply["content"]["message"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                messages.push(Message {
                    id: format!("{}:{}", target.oid, rpid),
                    user_id: mid.to_string(),
                    user_name: nickname,
                    content: if comment_text.is_empty() { None } else { Some(comment_text) },
                    extra_data: serde_json::json!({ "oid": target.oid, "aid": target.oid, "reply_type": target.reply_type, "referer": target.referer, "rpid": rpid, "already_replied": already_replied }),
                });
            }
            if filtered > 0 {
                log::debug!(
                    "oid={} pn第{}页: 过滤{}条已回复评论",
                    target.oid,
                    pn,
                    filtered
                );
            }

            let count = json["data"]["page"]["count"].as_u64().unwrap_or(0);
            if (pn as u64) * 20 >= count {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(messages)
    }

    async fn reply_to_comment(
        &self,
        account: &UserInfo,
        oid: u64,
        reply_type: u32,
        referer: &str,
        rpid: u64,
        message: &str,
    ) -> Result<(), String> {
        let csrf = extract_csrf(&account.cookie);
        if csrf.is_empty() {
            return Err("未找到 CSRF token".into());
        }

        let reply_type_s = reply_type.to_string();
        let oid_s = oid.to_string();
        let rpid_s = rpid.to_string();
        let csrf_s = csrf.clone();
        let resp = get_http_client()
            .post("https://api.bilibili.com/x/v2/reply/add")
            .header("Cookie", &account.cookie)
            .header("Referer", referer)
            .header("Origin", "https://www.bilibili.com")
            .header("Accept", ACCEPT_JSON)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .form(&[
                ("type", reply_type_s.as_str()),
                ("oid", oid_s.as_str()),
                ("message", message),
                ("root", rpid_s.as_str()),
                ("parent", rpid_s.as_str()),
                ("plat", "1"),
                ("csrf", csrf_s.as_str()),
                ("csrf_token", csrf_s.as_str()),
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

    async fn like_comment(
        &self,
        account: &UserInfo,
        oid: u64,
        reply_type: u32,
        referer: &str,
        rpid: u64,
    ) -> Result<(), String> {
        let csrf = extract_csrf(&account.cookie);
        if csrf.is_empty() {
            return Err("未找到 CSRF token".into());
        }

        let cookie = account.cookie.clone();

        let try_like = |delay: u64| {
            let csrf = csrf.clone();
            let cookie = cookie.clone();
            let reply_type_s = reply_type.to_string();
            let oid_s = oid.to_string();
            let rpid_s = rpid.to_string();
            async move {
                if delay > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
                let resp = get_http_client()
                .post("https://api.bilibili.com/x/v2/reply/action")
                .header("Cookie", &cookie)
                .header("Referer", referer)
                .header("Origin", "https://www.bilibili.com")
                .header("Accept", ACCEPT_JSON)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .form(&[
                    ("type", reply_type_s.as_str()),
                    ("oid", oid_s.as_str()),
                    ("rpid", rpid_s.as_str()),
                    ("action", "1"),
                    ("csrf", &csrf),
                    ("csrf_token", &csrf),
                ])
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;
                let text = resp
                    .text()
                    .await
                    .map_err(|e| format!("读取响应失败: {}", e))?;
                if text.trim_start().starts_with("<!DOCTYPE")
                    || text.trim_start().starts_with("<html")
                {
                    return Err("HTML_RESPONSE".to_string());
                }
                let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                    format!(
                        "解析JSON失败: {} | body={}",
                        e,
                        &text[..text.len().min(200)]
                    )
                })?;
                let code = json["code"].as_i64().unwrap_or(-1);
                if code != 0 {
                    let msg = json["message"].as_str().unwrap_or("未知");
                    if code == 65006 || msg.contains("重复") || msg.contains("已赞") {
                        log::info!("评论 rpid={} 已处于点赞状态", rpid);
                        return Ok(());
                    }
                    return Err(format!("点赞失败: code={}, msg={}", code, msg));
                }
                Ok(())
            }
        };

        let mut result = try_like(0).await;
        if let Err(ref e) = result {
            if e == "HTML_RESPONSE" {
                log::warn!(
                    "点赞API收到限流HTML，等待3秒重试 (oid={}, rpid={})",
                    oid,
                    rpid
                );
                result = try_like(3000).await;
                if let Err(ref e2) = result {
                    if e2 == "HTML_RESPONSE" {
                        return Err("B站限流返回HTML".to_string());
                    }
                }
            }
        }

        if let Err(ref e) = result {
            log::warn!("点赞评论 rpid={} 失败: {}", rpid, e);
            return result;
        }
        log::info!("已点赞评论 rpid={}", rpid);
        Ok(())
    }
}

#[async_trait]
impl MessageHandler for CommentHandler {
    fn name(&self) -> &'static str {
        match self.kind {
            CommentKind::Video => "视频评论处理器",
            CommentKind::Dynamic => "动态评论处理器",
        }
    }

    fn source_type(&self) -> MsgSource {
        match self.kind {
            CommentKind::Video => MsgSource::Comment,
            CommentKind::Dynamic => MsgSource::Dynamic,
        }
    }

    async fn fetch_messages(&self, account: &UserInfo) -> Result<Vec<Message>, String> {
        let targets: Vec<CommentTarget> = match self.kind {
            CommentKind::Video => self
                .get_videos(account)
                .await?
                .into_iter()
                .take(10)
                .map(|aid| CommentTarget {
                    oid: aid,
                    reply_type: 1,
                    referer: format!("https://www.bilibili.com/video/av{}", aid),
                    label: "视频",
                })
                .collect(),
            CommentKind::Dynamic => self
                .get_dynamics(account)
                .await?
                .into_iter()
                .take(10)
                .map(|dynamic_id| CommentTarget {
                    oid: dynamic_id,
                    reply_type: 11,
                    referer: format!("https://t.bilibili.com/{}", dynamic_id),
                    label: "动态",
                })
                .collect(),
        };
        log::info!("共获取到 {} 个{}，开始检查评论", targets.len(), self.name());
        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let my_mid = account.uid.parse::<i64>().unwrap_or(0);
        let mut all = Vec::new();
        // 每轮最多处理当前渠道最近10个目标，避免扫描时间过长。
        let max_targets = targets.len();
        let mut processed = 0u32;

        for (index, target) in targets[..max_targets].iter().enumerate() {
            match self.get_comments(account, target, my_mid).await {
                Ok(msgs) => {
                    if !msgs.is_empty() {
                        log::info!(
                            "{} oid={} 有 {} 条未回复评论",
                            target.label,
                            target.oid,
                            msgs.len()
                        );
                    }
                    all.extend(msgs);
                    processed += 1;
                }
                Err(e) => log::warn!("获取{} oid={} 评论失败: {}", target.label, target.oid, e),
            }
            if index + 1 < max_targets {
                // 请求之间保留短暂间隔，避免连续访问触发风控。
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            }
        }

        log::info!(
            "本轮处理了 {} 个评论目标，共计 {} 条未回复评论",
            processed,
            all.len()
        );
        Ok(all)
    }

    async fn send_reply(
        &self,
        account: &UserInfo,
        message: &Message,
        reply_msg: &str,
    ) -> Result<(), String> {
        let oid = message.extra_data["oid"]
            .as_u64()
            .or_else(|| message.extra_data["aid"].as_u64())
            .ok_or("缺少评论目标 oid")?;
        let reply_type = message.extra_data["reply_type"].as_u64().unwrap_or(1) as u32;
        let referer = message.extra_data["referer"].as_str().unwrap_or_else(|| {
            if reply_type == 11 {
                "https://t.bilibili.com/"
            } else {
                "https://www.bilibili.com/"
            }
        });
        let rpid = message.extra_data["rpid"].as_u64().ok_or("缺少rpid")?;
        self.reply_to_comment(account, oid, reply_type, referer, rpid, reply_msg)
            .await
    }

    async fn on_reply_success(
        &self,
        account: &UserInfo,
        message: &Message,
        state: &AutoReplyState,
    ) -> Result<(), String> {
        let settings = state.get_settings().await;
        let like_comments = settings
            .comment_settings(self.source_type())
            .map(|channel| channel.like_comments)
            .unwrap_or(false);
        if !like_comments {
            return Ok(());
        }
        let oid = message.extra_data["oid"]
            .as_u64()
            .or_else(|| message.extra_data["aid"].as_u64())
            .ok_or("缺少评论目标 oid")?;
        let reply_type = message.extra_data["reply_type"].as_u64().unwrap_or(1) as u32;
        let referer = message.extra_data["referer"].as_str().unwrap_or_else(|| {
            if reply_type == 11 {
                "https://t.bilibili.com/"
            } else {
                "https://www.bilibili.com/"
            }
        });
        let rpid = message.extra_data["rpid"].as_u64().ok_or("缺少rpid")?;
        self.like_comment(account, oid, reply_type, referer, rpid)
            .await
    }
}

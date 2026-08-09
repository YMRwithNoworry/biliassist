use super::handler::{Message, MessageHandler};
use super::http::{extract_csrf, get_http_client, resp_to_json};
use super::models::{ChannelReplySettings, MsgSource};
use super::state::{get_global_state, AutoReplyState};
use super::wbi;
use crate::bilibili::UserInfo;
use async_trait::async_trait;
use std::collections::HashSet;
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
    custom_reply: Option<ChannelReplySettings>,
    custom_like_comments: Option<bool>,
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

    async fn get_video_aid_by_bvid(&self, account: &UserInfo, bvid: &str) -> Result<u64, String> {
        let referer = format!("https://www.bilibili.com/video/{bvid}");
        let json = self
            .fetch_json(
                "https://api.bilibili.com/x/web-interface/view",
                &[("bvid", bvid)],
                &account.cookie,
                &referer,
            )
            .await?;
        if json["code"] != 0 {
            return Err(format!(
                "获取视频 {} 信息失败: {}",
                bvid,
                json["message"].as_str().unwrap_or("未知错误")
            ));
        }
        json["data"]["aid"]
            .as_u64()
            .or_else(|| {
                json["data"]["aid"]
                    .as_str()
                    .and_then(|value| value.parse().ok())
            })
            .ok_or_else(|| format!("视频 {} 缺少 aid", bvid))
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

    fn reply_id(reply: &serde_json::Value) -> u64 {
        Self::value_as_u64(&reply["rpid"])
            .or_else(|| Self::value_as_u64(&reply["rpid_str"]))
            .unwrap_or(0)
    }

    fn reply_mid(reply: &serde_json::Value) -> u64 {
        Self::value_as_u64(&reply["mid"])
            .or_else(|| Self::value_as_u64(&reply["member"]["mid"]))
            .unwrap_or(0)
    }

    fn reply_parent(reply: &serde_json::Value) -> u64 {
        Self::value_as_u64(&reply["parent"])
            .or_else(|| Self::value_as_u64(&reply["parent_str"]))
            .unwrap_or(0)
    }

    fn reply_dialog(reply: &serde_json::Value) -> u64 {
        Self::value_as_u64(&reply["dialog"])
            .or_else(|| Self::value_as_u64(&reply["dialog_str"]))
            .unwrap_or(0)
    }

    fn embedded_sub_comments(reply: &serde_json::Value) -> Vec<serde_json::Value> {
        reply["replies"]
            .as_array()
            .map(|replies| replies.to_vec())
            .unwrap_or_default()
    }

    fn message_from_reply(
        target: &CommentTarget,
        reply: &serde_json::Value,
        root_rpid: u64,
        parent_rpid: u64,
        my_mid: u64,
        already_replied: bool,
    ) -> Option<Message> {
        let rpid = Self::reply_id(reply);
        let mid = Self::reply_mid(reply);
        if rpid == 0 || mid == 0 || mid == my_mid {
            return None;
        }

        let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();
        let comment_text = reply["content"]["message"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Some(Message {
            id: format!("{}:{}", target.oid, rpid),
            user_id: mid.to_string(),
            user_name: nickname,
            content: if comment_text.is_empty() {
                None
            } else {
                Some(comment_text)
            },
            extra_data: serde_json::json!({
                "oid": target.oid,
                "aid": target.oid,
                "reply_type": target.reply_type,
                "referer": target.referer,
                "rpid": rpid,
                "root_rpid": root_rpid,
                "parent_rpid": parent_rpid,
                "already_replied": already_replied,
                "custom_message": target.custom_reply.as_ref().map(|channel| channel.message.clone()),
                "custom_reply_policy": target.custom_reply.as_ref().map(|channel| match channel.reply_policy {
                    super::models::ReplyPolicy::OncePerUser => "oncePerUser",
                    super::models::ReplyPolicy::PerMessage => "perMessage",
                }),
                "custom_like_comments": target.custom_like_comments,
            }),
        })
    }

    fn thread_messages(
        target: &CommentTarget,
        root: &serde_json::Value,
        sub_comments: &[serde_json::Value],
        my_mid: i64,
    ) -> Vec<Message> {
        let Some(my_mid) = u64::try_from(my_mid).ok() else {
            return Vec::new();
        };
        let root_rpid = Self::reply_id(root);
        if root_rpid == 0 {
            return Vec::new();
        }

        let mut messages = Vec::new();
        let root_replied = sub_comments
            .iter()
            .any(|reply| Self::reply_mid(reply) == my_mid);
        if let Some(message) =
            Self::message_from_reply(target, root, root_rpid, root_rpid, my_mid, root_replied)
        {
            messages.push(message);
        }

        for sub_comment in sub_comments {
            let child_rpid = Self::reply_id(sub_comment);
            if child_rpid == 0 {
                continue;
            }
            let child_replied = sub_comments.iter().any(|candidate| {
                Self::reply_mid(candidate) == my_mid
                    && (Self::reply_parent(candidate) == child_rpid
                        || Self::reply_dialog(candidate) == child_rpid)
            });
            if let Some(message) = Self::message_from_reply(
                target,
                sub_comment,
                root_rpid,
                child_rpid,
                my_mid,
                child_replied,
            ) {
                messages.push(message);
            }
        }

        messages
    }

    async fn get_sub_comments(
        &self,
        account: &UserInfo,
        target: &CommentTarget,
        root: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String> {
        let embedded = Self::embedded_sub_comments(root);
        let expected = Self::value_as_u64(&root["rcount"]).unwrap_or(embedded.len() as u64);
        if expected <= embedded.len() as u64 {
            return Ok(embedded);
        }

        let root_rpid = Self::reply_id(root);
        if root_rpid == 0 {
            return Ok(embedded);
        }

        let mut comments = Vec::new();
        for pn in 1..=30u32 {
            let oid_s = target.oid.to_string();
            let reply_type_s = target.reply_type.to_string();
            let root_s = root_rpid.to_string();
            let pn_s = pn.to_string();
            let params: &[(&str, &str)] = &[
                ("type", reply_type_s.as_str()),
                ("oid", oid_s.as_str()),
                ("root", root_s.as_str()),
                ("ps", "20"),
                ("pn", pn_s.as_str()),
            ];
            let json = self
                .fetch_json(
                    "https://api.bilibili.com/x/v2/reply/reply",
                    params,
                    &account.cookie,
                    &target.referer,
                )
                .await?;
            if json["code"] != 0 {
                return Err(format!(
                    "子评论API code={}, msg={}",
                    json["code"], json["message"]
                ));
            }

            let page_comments = json["data"]["replies"]
                .as_array()
                .map(|replies| replies.to_vec())
                .unwrap_or_default();
            let page_count = page_comments.len();
            if page_count == 0 {
                break;
            }
            comments.extend(page_comments);
            if comments.len() as u64 >= expected || page_count < 20 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

        if comments.is_empty() {
            return Ok(embedded);
        }
        let mut seen = HashSet::new();
        comments.retain(|reply| {
            let rpid = Self::reply_id(reply);
            rpid != 0 && seen.insert(rpid)
        });
        Ok(comments)
    }

    async fn messages_for_thread(
        &self,
        account: &UserInfo,
        target: &CommentTarget,
        root: &serde_json::Value,
        my_mid: i64,
    ) -> Vec<Message> {
        let embedded = Self::embedded_sub_comments(root);
        let sub_comments = match self.get_sub_comments(account, target, root).await {
            Ok(comments) => comments,
            Err(error) => {
                log::warn!(
                    "获取 oid={} 根评论 rpid={} 的子评论失败，使用接口内嵌数据: {}",
                    target.oid,
                    Self::reply_id(root),
                    error
                );
                embedded
            }
        };
        Self::thread_messages(target, root, &sub_comments, my_mid)
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
            let mut added = 0usize;
            for reply in &replies {
                let thread_messages = self
                    .messages_for_thread(account, target, reply, my_mid)
                    .await;
                let thread_filtered = thread_messages
                    .iter()
                    .filter(|message| {
                        message.extra_data["already_replied"]
                            .as_bool()
                            .unwrap_or(false)
                    })
                    .count();
                filtered += thread_filtered as u32;
                added += thread_messages.len().saturating_sub(thread_filtered);
                messages.extend(thread_messages);
            }
            log::info!(
                "oid={} 第{}页: {}个评论线程, {}条待处理, {}条已回复",
                target.oid,
                page,
                count,
                added,
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
            let mut added = 0usize;
            for reply in &replies {
                let thread_messages = self
                    .messages_for_thread(account, target, reply, my_mid)
                    .await;
                let thread_filtered = thread_messages
                    .iter()
                    .filter(|message| {
                        message.extra_data["already_replied"]
                            .as_bool()
                            .unwrap_or(false)
                    })
                    .count();
                filtered += thread_filtered as u32;
                added += thread_messages.len().saturating_sub(thread_filtered);
                messages.extend(thread_messages);
            }
            if filtered > 0 || added > 0 {
                log::debug!(
                    "oid={} pn第{}页: {}条待处理, {}条已回复",
                    target.oid,
                    pn,
                    added,
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
        root_rpid: u64,
        parent_rpid: u64,
        message: &str,
    ) -> Result<(), String> {
        let csrf = extract_csrf(&account.cookie);
        if csrf.is_empty() {
            return Err("未找到 CSRF token".into());
        }

        let reply_type_s = reply_type.to_string();
        let oid_s = oid.to_string();
        let root_s = root_rpid.to_string();
        let parent_s = parent_rpid.to_string();
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
                ("root", root_s.as_str()),
                ("parent", parent_s.as_str()),
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
            CommentKind::Video => {
                let settings = get_global_state().get_settings().await;
                let mut targets = Vec::new();

                if settings.channels.comment.reply.enabled {
                    match self.get_videos(account).await {
                        Ok(aids) => {
                            for aid in aids.into_iter().take(10) {
                                targets.push(CommentTarget {
                                    oid: aid,
                                    reply_type: 1,
                                    referer: format!("https://www.bilibili.com/video/av{aid}"),
                                    label: "视频",
                                    custom_reply: None,
                                    custom_like_comments: None,
                                });
                            }
                        }
                        Err(error) => {
                            log::warn!("获取自己的视频列表失败，继续处理指定视频: {}", error)
                        }
                    }
                }

                for tracked in settings.tracked_videos.iter() {
                    if !tracked.reply.enabled {
                        continue;
                    }
                    let Some(bvid) = tracked.normalized_bvid() else {
                        log::warn!("忽略无效 BV 号: {}", tracked.bvid);
                        continue;
                    };
                    match self.get_video_aid_by_bvid(account, &bvid).await {
                        Ok(aid) => {
                            let target = CommentTarget {
                                oid: aid,
                                reply_type: 1,
                                referer: format!("https://www.bilibili.com/video/{bvid}"),
                                label: "指定视频",
                                custom_reply: Some(tracked.reply.clone()),
                                custom_like_comments: Some(tracked.like_comments),
                            };
                            if let Some(existing) = targets.iter_mut().find(|item| item.oid == aid)
                            {
                                *existing = target;
                            } else {
                                targets.push(target);
                            }
                        }
                        Err(error) => log::warn!("获取指定视频 {} 失败: {}", bvid, error),
                    }
                }
                targets
            }
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
                    custom_reply: None,
                    custom_like_comments: None,
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
        let root_rpid = message.extra_data["root_rpid"].as_u64().unwrap_or(rpid);
        let parent_rpid = message.extra_data["parent_rpid"].as_u64().unwrap_or(rpid);
        self.reply_to_comment(
            account,
            oid,
            reply_type,
            referer,
            root_rpid,
            parent_rpid,
            reply_msg,
        )
        .await
    }

    async fn on_reply_success(
        &self,
        account: &UserInfo,
        message: &Message,
        state: &AutoReplyState,
    ) -> Result<(), String> {
        let settings = state.get_settings().await;
        let like_comments = message.extra_data["custom_like_comments"]
            .as_bool()
            .or_else(|| {
                settings
                    .comment_settings(self.source_type())
                    .map(|channel| channel.like_comments)
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn video_target() -> CommentTarget {
        CommentTarget {
            oid: 42,
            reply_type: 1,
            referer: "https://www.bilibili.com/video/av42".to_string(),
            label: "视频",
            custom_reply: None,
            custom_like_comments: None,
        }
    }

    #[test]
    fn builds_messages_for_root_and_sub_comments_with_correct_reply_targets() {
        let root = serde_json::json!({
            "rpid": 100,
            "mid": 2,
            "member": { "uname": "一级用户" },
            "content": { "message": "一级评论" }
        });
        let sub_comments = vec![
            serde_json::json!({
                "rpid": 101,
                "root": 100,
                "parent": 100,
                "mid": 3,
                "member": { "uname": "子评论用户" },
                "content": { "message": "子评论" }
            }),
            serde_json::json!({
                "rpid": 102,
                "root": 100,
                "parent": 101,
                "dialog": 101,
                "mid": 1,
                "member": { "uname": "UP主" },
                "content": { "message": "已经回复子评论" }
            }),
            serde_json::json!({
                "rpid": 103,
                "root": 100,
                "parent": 100,
                "mid": 4,
                "member": { "uname": "另一用户" },
                "content": { "message": "另一条子评论" }
            }),
        ];

        let messages = CommentHandler::thread_messages(&video_target(), &root, &sub_comments, 1);

        assert_eq!(3, messages.len());
        assert!(messages.iter().all(|message| message.user_id != "1"));

        let root_message = messages
            .iter()
            .find(|message| message.extra_data["rpid"] == 100)
            .unwrap();
        assert!(root_message.extra_data["already_replied"]
            .as_bool()
            .unwrap());
        assert_eq!(100, root_message.extra_data["root_rpid"]);
        assert_eq!(100, root_message.extra_data["parent_rpid"]);

        let replied_child = messages
            .iter()
            .find(|message| message.extra_data["rpid"] == 101)
            .unwrap();
        assert!(replied_child.extra_data["already_replied"]
            .as_bool()
            .unwrap());
        assert_eq!(100, replied_child.extra_data["root_rpid"]);
        assert_eq!(101, replied_child.extra_data["parent_rpid"]);

        let new_child = messages
            .iter()
            .find(|message| message.extra_data["rpid"] == 103)
            .unwrap();
        assert!(!new_child.extra_data["already_replied"].as_bool().unwrap());
    }

    #[test]
    fn carries_tracked_video_overrides_to_messages() {
        let target = CommentTarget {
            custom_reply: Some(ChannelReplySettings {
                message: "指定回复".to_string(),
                reply_policy: super::super::models::ReplyPolicy::OncePerUser,
                ..ChannelReplySettings::default()
            }),
            custom_like_comments: Some(false),
            ..video_target()
        };
        let reply = serde_json::json!({
            "rpid": 200,
            "mid": 2,
            "member": { "uname": "用户" },
            "content": { "message": "评论" }
        });
        let message =
            CommentHandler::message_from_reply(&target, &reply, 200, 200, 1, false).unwrap();
        assert_eq!("指定回复", message.extra_data["custom_message"]);
        assert_eq!("oncePerUser", message.extra_data["custom_reply_policy"]);
        assert!(!message.extra_data["custom_like_comments"]
            .as_bool()
            .unwrap());
    }
}

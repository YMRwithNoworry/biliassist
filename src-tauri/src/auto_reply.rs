use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use reqwest::Client;

use crate::storage::get_active_account;

// ============================================================
//  数据结构
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum MsgSource {
    Comment,
    DirectMessage,
    Follow,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyHistory {
    pub user: String,
    pub time: String,
    pub message: String,
    pub source: MsgSource,
}

// ============================================================
//  全局状态
// ============================================================

static SETTINGS: std::sync::OnceLock<Arc<RwLock<AutoReplySettings>>> = std::sync::OnceLock::new();
static REPLIED_SET: std::sync::OnceLock<Arc<RwLock<HashSet<String>>>> = std::sync::OnceLock::new();
static HTTP_CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();

fn get_settings_lock() -> &'static Arc<RwLock<AutoReplySettings>> {
    SETTINGS.get_or_init(|| {
        let default_settings = AutoReplySettings {
            enabled: true,
            message: "感谢您的留言！我会尽快回复。".to_string(),
            interval: 60,
            reply_only_once: true,
            sources: vec![MsgSource::Comment, MsgSource::DirectMessage, MsgSource::Follow],
            history: Vec::new(),
        };
        Arc::new(RwLock::new(default_settings))
    })
}

/// 从文件加载设置（包含历史记录）
pub async fn init_settings() {
    let data_dir = match dirs::home_dir() {
        Some(dir) => dir.join(".bilibili_account_manager"),
        None => return,
    };

    let file_path = data_dir.join("auto_reply_settings.json");
    if !file_path.exists() {
        return;
    }

    let json = match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => content,
        Err(e) => {
            log::warn!("读取自动回复设置失败: {}", e);
            return;
        }
    };

    let loaded: AutoReplySettings = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("解析自动回复设置失败: {}", e);
            return;
        }
    };

    let mut settings = get_settings_lock().write().await;
    settings.enabled = loaded.enabled;
    settings.message = loaded.message;
    settings.interval = loaded.interval;
    settings.reply_only_once = loaded.reply_only_once;
    settings.sources = loaded.sources;
    // 合并历史记录，保留已有的
    if !loaded.history.is_empty() {
        settings.history = loaded.history;
    }
    log::info!("已加载自动回复设置，历史记录 {} 条", settings.history.len());
}

fn get_replied_set() -> &'static Arc<RwLock<HashSet<String>>> {
    REPLIED_SET.get_or_init(|| Arc::new(RwLock::new(HashSet::new())))
}

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create HTTP client")
    })
}

// ============================================================
//  工具：从 cookie 提取 CSRF token
// ============================================================

fn extract_csrf(cookie: &str) -> String {
    for pair in cookie.split(';') {
        let pair = pair.trim();
        if pair.starts_with("bili_jct=") {
            return pair.trim_start_matches("bili_jct=").to_string();
        }
    }
    log::warn!("cookie 中未找到 bili_jct，cookie 长度: {}, 前100字符: {}", cookie.len(), &cookie[..cookie.len().min(100)]);
    String::new()
}

// ============================================================
//  工具：安全获取 JSON（先 text 再 parse，避免解码失败）
// ============================================================

async fn resp_to_json(resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("解析JSON失败: {} | body={}", e, &text[..text.len().min(200)]))
}

// ============================================================
//  WBI 签名（B站风控签名）
// ============================================================

/// WBI 混淆密钥表
const MIXIN_KEY_ENC_TAB: [u8; 32] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35,
    27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
];

fn get_mixin_key(orig: &str) -> String {
    let mut str = orig.to_string();
    str.truncate(32);
    let bytes: Vec<char> = str.chars().collect();
    let mut result = String::with_capacity(32);
    for &i in &MIXIN_KEY_ENC_TAB {
        if (i as usize) < bytes.len() {
            result.push(bytes[i as usize]);
        }
    }
    result
}

/// 从 WBI URL 中提取 key（URL 格式: https://i0.hdslb.com/bfs/wbi/{key}.png）
fn extract_wbi_key(url: &str) -> String {
    url.rsplit('/')
        .next()
        .and_then(|s| s.split('.').next())
        .unwrap_or("")
        .to_string()
}

/// 获取 WBI keys（img_key + sub_key）
async fn get_wbi_keys(cookie: &str) -> Result<(String, String), String> {
    let url = "https://api.bilibili.com/x/web-interface/nav";

    let resp = get_http_client()
        .get(url)
        .header("Cookie", cookie)
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| format!("请求nav失败: {}", e))?;

    let json = resp_to_json(resp).await?;

    if json["code"] != 0 {
        log::error!("获取WBI keys失败: code={}, msg={}", json["code"], json["message"]);
        return Err(format!("获取WBI keys失败: {}", json["message"]));
    }

    let img_url = json["data"]["wbi_img"]["img_url"]
        .as_str()
        .unwrap_or("");
    let sub_url = json["data"]["wbi_img"]["sub_url"]
        .as_str()
        .unwrap_or("");

    // URL 格式: https://i0.hdslb.com/bfs/wbi/{key}.png
    let img_key = extract_wbi_key(img_url);
    let sub_key = extract_wbi_key(sub_url);

    log::info!("获取WBI keys成功: img_key={}, sub_key={}", 
        if img_key.len() > 8 { &img_key[..8] } else { &img_key },
        if sub_key.len() > 8 { &sub_key[..8] } else { &sub_key });

    Ok((img_key, sub_key))
}

/// 对参数进行 WBI 签名
fn sign_wbi_params(
    params: &[(String, String)],
    img_key: &str,
    sub_key: &str,
) -> Vec<(String, String)> {
    let mixin_key = get_mixin_key(&format!("{}{}", img_key, sub_key));
    let mut map: Vec<(String, String)> = params.to_vec();
    let wts = Utc::now().timestamp().to_string();
    map.push(("wts".to_string(), wts));

    // 按 key 排序
    map.sort_by(|a, b| a.0.cmp(&b.0));

    // URL 编码函数（空格变成 %20）
    fn encode_value(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                '!' | '\'' | '(' | ')' | '*' => {
                    // 这些字符不编码，直接保留
                    result.push(c);
                }
                ' ' => result.push_str("%20"),
                c if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' => {
                    result.push(c);
                }
                _ => {
                    // 其他字符进行 URL 编码
                    for byte in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        result
    }

    // 拼接查询字符串
    let query: String = map.iter()
        .map(|(k, v)| format!("{}={}", k, encode_value(v)))
        .collect::<Vec<_>>()
        .join("&");

    // 计算 MD5
    let hash = md5_hex(&format!("{}{}", query, mixin_key));

    map.push(("w_rid".to_string(), hash));

    map
}

/// 标准 MD5 哈希（B站 WBI 签名要求）
fn md5_hex(input: &str) -> String {
    use md5::{Md5, Digest};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============================================================
//  Settings API
// ============================================================

pub async fn get_settings() -> Result<AutoReplySettings, String> {
    let settings = get_settings_lock().read().await;
    Ok(settings.clone())
}

pub async fn save_settings(new_settings: AutoReplySettings) -> Result<(), String> {
    let mut settings = get_settings_lock().write().await;
    *settings = new_settings.clone();

    let data_dir = dirs::home_dir()
        .ok_or("无法获取用户目录")?
        .join(".bilibili_account_manager");

    let json = serde_json::to_string(&*settings)
        .map_err(|e| format!("序列化失败: {}", e))?;

    tokio::fs::write(data_dir.join("auto_reply_settings.json"), json)
        .await
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(())
}

// ============================================================
//  自动回复服务主循环
// ============================================================

pub async fn start_auto_reply_service() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let (enabled, interval, message, reply_only_once, sources) = {
            let s = get_settings_lock().read().await;
            (s.enabled, s.interval, s.message.clone(), s.reply_only_once, s.sources.clone())
        };

        if !enabled {
            continue;
        }

        if let Err(e) = run_all(&message, reply_only_once, &sources).await {
            log::error!("自动回复失败: {}", e);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
}

async fn run_all(msg: &str, once: bool, sources: &[MsgSource]) -> Result<(), String> {
    let account = get_active_account().await.ok_or("没有激活的账号")?;

    // 诊断 cookie
    let has_sessdata = account.cookie.contains("SESSDATA=");
    let has_bili_jct = account.cookie.contains("bili_jct=");
    let has_dede = account.cookie.contains("DedeUserID=");
    log::info!(
        "账号 cookie 诊断: len={}, SESSDATA={}, bili_jct={}, DedeUserID={}",
        account.cookie.len(), has_sessdata, has_bili_jct, has_dede
    );

    if !has_sessdata || !has_bili_jct {
        return Err("cookie 不完整（缺少 SESSDATA 或 bili_jct），请删除账号重新扫码登录".to_string());
    }

    for src in sources {
        let result = match src {
            MsgSource::Comment => reply_comments(&account, msg, once).await,
            MsgSource::DirectMessage => reply_dm(&account, msg, once).await,
            MsgSource::Follow => reply_follows(&account, msg, once).await,
        };
        if let Err(e) = result {
            log::error!("{:?}回复失败: {}", src, e);
        }
    }
    Ok(())
}

// ============================================================
//  视频评论：获取视频列表 → 获取未回复评论 → 评论区直接回复
// ============================================================

async fn get_my_videos(account: &crate::bilibili::UserInfo) -> Result<Vec<u64>, String> {
    // 使用不需要 WBI 签名的 API
    let mut videos = Vec::new();
    let mut page = 1u32;

    while page <= 5 {
        let pn = page.to_string();
        let ps = "30".to_string();

        // 使用 x/space/arc/search API（不需要 WBI 签名）
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

async fn get_unreplied_comments(
    account: &crate::bilibili::UserInfo,
    aid: u64,
) -> Result<Vec<(u64, u64, String)>, String> {
    let mut unreplied = Vec::new();
    let mut next_offset: i64 = 0;
    let mut page = 0u32;
    let my_mid = account.uid.parse::<i64>().unwrap_or(0);

    log::info!("开始获取视频 aid={} 的评论，my_mid={}", aid, my_mid);

    while page < 3 {
        let aid_s = aid.to_string();
        let offset_s = next_offset.to_string();

        let resp = get_http_client()
            .get("https://api.bilibili.com/x/v2/reply")
            .header("Cookie", &account.cookie)
            .header("Referer", format!("https://www.bilibili.com/video/av{}", aid))
            .query(&[
                ("type", "1"),
                ("oid", &aid_s),
                ("sort", "2"),
                ("offset", &offset_s),
                ("mode", "3"),
                ("plat", "1"),
                ("web_location", "1315875"),
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
        log::info!("视频 aid={} 第 {} 页获取到 {} 条评论", aid, page, replies.len());
        if replies.is_empty() { break; }

        for reply in replies {
            let rpid = reply["rpid"].as_u64().unwrap_or(0);
            let mid = reply["mid"].as_i64().unwrap_or(0);
            let nickname = reply["member"]["uname"].as_str().unwrap_or("").to_string();

            log::debug!("评论: rpid={}, mid={}, nickname={}, my_mid={}", rpid, mid, nickname, my_mid);

            if mid == my_mid { continue; }

            // 检查 UP 是否已回复（检查楼中楼中是否有自己的回复）
            let has_up_reply = reply["replies"].as_array()
                .map(|subs| subs.iter().any(|r| r["mid"].as_i64().unwrap_or(0) == my_mid))
                .unwrap_or(false);

            // 同时检查 up_action.reply 字段（B站标记UP主是否回复过）
            let up_action_reply = reply["up_action"]["reply"].as_bool().unwrap_or(false);

            log::debug!("评论 rpid={}: has_up_reply={}, up_action_reply={}", rpid, has_up_reply, up_action_reply);

            if !has_up_reply && !up_action_reply {
                log::info!("找到未回复评论: rpid={}, mid={}, nickname={}", rpid, mid, nickname);
                unreplied.push((rpid, mid as u64, nickname));
            }
        }

        next_offset = json["data"]["offset"].as_i64().unwrap_or(0);
        if next_offset == 0 { break; }
        page += 1;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    log::info!("视频 aid={} 共找到 {} 条未回复评论", aid, unreplied.len());
    Ok(unreplied)
}

async fn reply_to_comment(
    account: &crate::bilibili::UserInfo,
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

async fn reply_comments(
    account: &crate::bilibili::UserInfo,
    reply_msg: &str,
    once: bool,
) -> Result<(), String> {
    log::info!("开始处理视频评论自动回复，账号: {}", account.name);

    let videos = get_my_videos(account).await?;
    if videos.is_empty() {
        log::info!("没有找到发布的视频");
        return Ok(());
    }

    log::info!("找到 {} 个视频，检查未回复评论", videos.len());

    let mut total_replied = 0u32;

    for aid in videos {
        let comments = match get_unreplied_comments(account, aid).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("获取视频 {} 评论失败: {}", aid, e);
                continue;
            }
        };

        if comments.is_empty() { continue; }

        log::info!("视频 aid={} 有 {} 条未回复评论", aid, comments.len());

        for (rpid, _mid, nickname) in comments {
            let dedup = format!("c:{}:{}", aid, rpid);

            if once {
                let set = get_replied_set().read().await;
                if set.contains(&dedup) {
                    log::debug!("已回复过，跳过: {}", dedup);
                    continue;
                }
                drop(set);
            }

            let formatted = format_msg(reply_msg, &nickname);

            match reply_to_comment(account, aid, rpid, &formatted).await {
                Ok(_) => {
                    if once { get_replied_set().write().await.insert(dedup); }
                    add_history(&nickname, &formatted, MsgSource::Comment).await;
                    total_replied += 1;
                }
                Err(e) => log::error!("回复评论失败 aid={} rpid={}: {}", aid, rpid, e),
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    log::info!("视频评论回复完成，共回复 {} 条", total_replied);
    Ok(())
}

// ============================================================
//  私信
// ============================================================

/// 生成设备ID (dev_id)
fn generate_dev_id() -> String {
    let mut result = String::with_capacity(36);
    let mut rng = rand::thread_rng();
    use rand::Rng;
    let template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx";
    for c in template.chars() {
        match c {
            'x' => {
                let r: u8 = rng.gen_range(0..16);
                result.push_str(&format!("{:X}", r));
            }
            'y' => {
                let r: u8 = rng.gen_range(0..16);
                result.push_str(&format!("{:X}", 3 & r | 8));
            }
            _ => result.push(c),
        }
    }
    result
}

async fn reply_dm(
    account: &crate::bilibili::UserInfo,
    reply_msg: &str,
    once: bool,
) -> Result<(), String> {
    // 使用 B站私信会话接口获取有未读消息的会话列表
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

    // 新接口的数据结构: data.session_list[]
    let empty = vec![];
    let sessions = json["data"]["session_list"].as_array().unwrap_or(&empty);

    for session in sessions {
        let talker_id = session["talker_id"].as_i64().unwrap_or(0);
        let unread_count = session["unread_count"].as_i64().unwrap_or(0);
        let last_msg = &session["last_msg"];
        let name = last_msg["sender_uid"].as_i64().unwrap_or(0).to_string();
        let uid = talker_id.to_string();
        let dedup = format!("dm:{}", uid);

        if talker_id == 0 { continue; }
        // 只回复有未读消息的会话
        if unread_count == 0 { continue; }

        if once {
            let set = get_replied_set().read().await;
            if set.contains(&dedup) { continue; }
            drop(set);
        }

        match send_dm(account, &uid, reply_msg).await {
            Ok(_) => {
                if once { get_replied_set().write().await.insert(dedup); }
                let formatted = format_msg(reply_msg, &name);
                add_history(&name, &formatted, MsgSource::DirectMessage).await;
                // 私信发送间隔 3 秒，避免触发风控
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Err(e) => {
                log::error!("私信回复 {} 失败: {}", uid, e);
                // 遇到风控限制立即停止
                if e.contains("banned") || e.contains("频繁") || e.contains("风控") {
                    return Err("触发B站风控限制，停止发送私信".to_string());
                }
            }
        }
    }

    Ok(())
}

async fn send_dm(
    account: &crate::bilibili::UserInfo,
    uid: &str,
    msg: &str,
) -> Result<(), String> {
    let formatted = format_msg(msg, &format!("用户{}", uid));
    let csrf = extract_csrf(&account.cookie);
    if csrf.is_empty() {
        return Err("cookie 中缺少 bili_jct (CSRF token)".to_string());
    }

    let receiver_id = uid.parse::<i64>().unwrap_or(0);
    let sender_id = account.uid.parse::<i64>().unwrap_or(0);
    let dev_id = generate_dev_id();
    let timestamp = chrono::Utc::now().timestamp();

    // content 需要是 JSON 格式
    let content_json = serde_json::json!({
        "content": formatted
    });

    let receiver_type = "1".to_string();
    let msg_type = "1".to_string();
    let msg_status = "0".to_string();

    // B站正确的私信发送接口
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

// ============================================================
//  粉丝关注
// ============================================================

async fn reply_follows(
    account: &crate::bilibili::UserInfo,
    reply_msg: &str,
    once: bool,
) -> Result<(), String> {
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

    for f in list {
        let mid = f["mid"].as_i64().unwrap_or(0);
        let name = f["uname"].as_str().unwrap_or("").to_string();
        let uid = mid.to_string();
        let dedup = format!("f:{}", uid);

        if mid == 0 { continue; }
        if once {
            let set = get_replied_set().read().await;
            if set.contains(&dedup) { continue; }
            drop(set);
        }

        match send_dm(account, &uid, reply_msg).await {
            Ok(_) => {
                if once { get_replied_set().write().await.insert(dedup); }
                let formatted = format_msg(reply_msg, &name);
                add_history(&name, &formatted, MsgSource::Follow).await;
                // 私信发送间隔 3 秒，避免触发风控
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Err(e) => {
                log::error!("关注回复 {} 失败: {}", uid, e);
                // 遇到风控限制立即停止
                if e.contains("banned") || e.contains("频繁") {
                    return Err("触发B站风控限制，停止发送私信".to_string());
                }
            }
        }
    }

    Ok(())
}

// ============================================================
//  工具
// ============================================================

fn format_msg(msg: &str, username: &str) -> String {
    msg.replace("{用户名}", username)
       .replace("{时间}", &Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
}

async fn add_history(user: &str, msg: &str, source: MsgSource) {
    let mut s = get_settings_lock().write().await;
    s.history.insert(0, ReplyHistory {
        user: user.into(),
        time: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        message: msg.into(),
        source,
    });
    if s.history.len() > 100 { s.history.truncate(100); }

    // 保存到文件，确保历史记录持久化
    let data_dir = dirs::home_dir().map(|d| d.join(".bilibili_account_manager"));
    if let Some(dir) = data_dir {
        let json = match serde_json::to_string(&*s) {
            Ok(j) => j,
            Err(e) => {
                log::error!("序列化设置失败: {}", e);
                return;
            }
        };
        let file_path = dir.join("auto_reply_settings.json");
        if let Err(e) = tokio::fs::write(&file_path, json).await {
            log::error!("保存设置到文件失败: {}", e);
        }
    }
}

pub async fn test_reply() -> Result<String, String> {
    let s = get_settings_lock().read().await;
    Ok(format!("测试回复内容:\n{}", format_msg(&s.message, "测试用户")))
}

/// 手动触发一次视频评论回复（用于测试和诊断）
pub async fn manual_reply_comments() -> Result<String, String> {
    let account = crate::storage::get_active_account().await.ok_or("没有激活的账号")?;
    let (reply_msg, once) = {
        let s = get_settings_lock().read().await;
        (s.message.clone(), s.reply_only_once)
    };

    match reply_comments(&account, &reply_msg, once).await {
        Ok(_) => Ok("视频评论回复任务已执行，请查看日志".to_string()),
        Err(e) => Err(format!("执行失败: {}", e)),
    }
}

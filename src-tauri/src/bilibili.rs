use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use qrcode::QrCode;
use base64::{Engine as _, engine::general_purpose};

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create HTTP client")
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponse {
    pub qrcode: String,
    pub qrcode_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStatus {
    pub status: String,
    pub user_info: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub uid: String,
    pub name: String,
    pub avatar: String,
    pub cookie: String,
}

pub async fn get_qr_code() -> Result<QrCodeResponse, String> {
    let url = "https://passport.bilibili.com/x/passport-login/web/qrcode/generate";

    let response = get_client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let text = response.text().await.map_err(|e| format!("读取失败: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("解析失败: {} | body={}", e, &text[..text.len().min(200)]))?;

    if json["code"] != 0 {
        return Err(format!("获取二维码失败: {}", json["message"]));
    }

    let qr_url = json["data"]["url"].as_str().ok_or("二维码URL不存在")?;
    let qrcode_key = json["data"]["qrcode_key"].as_str().ok_or("二维码key不存在")?;

    // 生成二维码图片
    let code = QrCode::new(qr_url).map_err(|e| format!("生成二维码失败: {}", e))?;
    let image = code.render::<image::Luma<u8>>().build();
    let mut buffer = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| format!("编码PNG失败: {}", e))?;

    let base64_qrcode = general_purpose::STANDARD.encode(&buffer);

    crate::storage::save_current_qrcode_key(qrcode_key.to_string()).await;

    Ok(QrCodeResponse {
        qrcode: base64_qrcode,
        qrcode_key: qrcode_key.to_string(),
    })
}

pub async fn check_login_status() -> Result<LoginStatus, String> {
    let qrcode_key = crate::storage::get_current_qrcode_key()
        .await
        .ok_or("未找到二维码key，请先获取二维码")?;

    let url = format!(
        "https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}",
        qrcode_key
    );

    let response = get_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    // 提取 Set-Cookie headers
    let set_cookies: Vec<String> = response.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| {
            // 只取 name=value 部分（分号前）
            s.split(';').next().unwrap_or("").trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    let text = response.text().await.map_err(|e| format!("读取失败: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("解析失败: {} | body={}", e, &text[..text.len().min(300)]))?;

    // 先检查外层 code
    let outer_code = json["code"].as_i64().unwrap_or(-1);
    if outer_code != 0 {
        return Ok(LoginStatus {
            status: "expired".to_string(),
            user_info: None,
        });
    }

    // 内层 data.code 表示扫码状态
    let inner_code = json["data"]["code"].as_i64().unwrap_or(-1);
    log::info!("poll 状态: outer_code={}, inner_code={}", outer_code, inner_code);

    match inner_code {
        86101 => Ok(LoginStatus {
            status: "waiting".to_string(),
            user_info: None,
        }),
        86090 => Ok(LoginStatus {
            status: "scanned".to_string(),
            user_info: None,
        }),
        0 => {
            // 登录成功
            // 从 poll 响应的 data.url 中提取关键 cookie 参数
            // URL 格式: https://passport.bilibili.com/login?SESSDATA=xxx&bili_jct=xxx&DedeUserID=xxx
            let url_cookies = json["data"]["url"].as_str()
                .map(|u| extract_cookies_from_url(u))
                .unwrap_or_default();

            // 合并 Set-Cookie 和 URL 中的 cookie
            let mut all_cookies = set_cookies;
            all_cookies.extend(url_cookies);

            // 获取用户信息（此时 client 的 cookie_store 已有登录态）
            let user_info = get_user_info(&all_cookies).await?;

            // 保存账号
            crate::storage::save_account(&user_info).await?;

            Ok(LoginStatus {
                status: "success".to_string(),
                user_info: Some(user_info),
            })
        }
        86038 => Ok(LoginStatus {
            status: "expired".to_string(),
            user_info: None,
        }),
        _ => Err(format!("未知登录状态码: {}", inner_code)),
    }
}

/// 从 B站 poll 登录成功后的 URL 中提取关键 cookie
/// URL 格式: https://passport.bilibili.com/login?SESSDATA=xxx&bili_jct=xxx&DedeUserID=xxx&...
fn extract_cookies_from_url(url: &str) -> Vec<String> {
    let query = url.split('?').nth(1).unwrap_or("");
    let important_keys = ["SESSDATA", "bili_jct", "DedeUserID", "DedeUserID__ckMd5"];

    query.split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next()?;
            if important_keys.contains(&key) && !value.is_empty() {
                Some(format!("{}={}", key, url_decode(value)))
            } else {
                None
            }
        })
        .collect()
}

/// 简易 URL 解码（处理 %XX 格式）
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

async fn get_user_info(extra_cookies: &[String]) -> Result<UserInfo, String> {
    let url = "https://api.bilibili.com/x/web-interface/nav";

    // 将 extra_cookies 拼接成字符串用于请求
    let cookie_header: String = extra_cookies.join("; ");

    let response = get_client()
        .get(url)
        .header("Cookie", &cookie_header)
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    // 从这个请求的响应中也提取 Set-Cookie
    let nav_cookies: Vec<String> = response.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let text = response.text().await.map_err(|e| format!("读取失败: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("解析失败: {} | body={}", e, &text[..text.len().min(200)]))?;

    if json["code"] != 0 {
        return Err(format!("获取用户信息失败: {}", json["message"]));
    }

    let data = &json["data"];
    let uid = data["mid"].as_i64().ok_or("UID不存在")?.to_string();
    let name = data["uname"].as_str().ok_or("用户名不存在")?.to_string();
    let avatar = data["face"].as_str().unwrap_or("").to_string();

    // 合并所有来源的 cookie
    let mut all_cookies: Vec<String> = extra_cookies.to_vec();
    all_cookies.extend(nav_cookies);

    // 去重
    let mut seen = std::collections::HashSet::new();
    let cookie_str: String = all_cookies.iter()
        .filter(|c| {
            let key = c.split('=').next().unwrap_or("");
            seen.insert(key.to_string())
        })
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");

    let has_sessdata = cookie_str.contains("SESSDATA=");
    let has_bili_jct = cookie_str.contains("bili_jct=");
    let has_dede = cookie_str.contains("DedeUserID=");
    log::info!(
        "登录成功: uid={}, name={}, avatar_len={}, cookie_len={}, SESSDATA={}, bili_jct={}, DedeUserID={}",
        uid, name, avatar.len(), cookie_str.len(), has_sessdata, has_bili_jct, has_dede
    );
    if !has_sessdata || !has_bili_jct {
        log::warn!("cookie 不完整! extra_cookies={:?}", extra_cookies);
    }

    Ok(UserInfo {
        uid,
        name,
        avatar,
        cookie: cookie_str,
    })
}

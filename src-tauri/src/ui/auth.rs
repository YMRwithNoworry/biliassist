use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

pub(super) const SUPABASE_URL: &str = "https://dweeixiqejtewdlzfzwx.supabase.co";
pub(super) const SUPABASE_KEY: &str = "sb_publishable__R-vclqmTFFFgfK4zrctsg_dMxRD-nz";
const SESSION_FILE: &str = "auth_session.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub email: String,
    pub tier: String,
}

fn data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bilibili_account_manager")
}

fn error_message(value: &Value) -> String {
    value["error_description"]
        .as_str()
        .or_else(|| value["msg"].as_str())
        .or_else(|| value["message"].as_str())
        .or_else(|| value["error"].as_str())
        .unwrap_or("认证请求失败")
        .to_string()
}

pub(super) fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn parse_session(response: reqwest::Response) -> Result<AuthSession, String> {
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|error| format!("解析认证响应失败：{error}"))?;
    if !status.is_success() {
        return Err(error_message(&value));
    }

    let access_token = value["access_token"]
        .as_str()
        .ok_or_else(|| "认证成功但未返回访问令牌，请确认邮箱后登录".to_string())?;
    let refresh_token = value["refresh_token"].as_str().unwrap_or_default();
    let user_id = value["user"]["id"].as_str().ok_or("认证响应缺少用户 ID")?;
    let email = value["user"]["email"].as_str().unwrap_or_default();
    let tier = fetch_tier(access_token, user_id).await;

    Ok(AuthSession {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        user_id: user_id.to_string(),
        email: email.to_string(),
        tier,
    })
}

pub async fn sign_in_password(email: String, password: String) -> Result<AuthSession, String> {
    if email.trim().is_empty() || password.is_empty() {
        return Err("请输入邮箱和密码".into());
    }
    let response = client()
        .post(format!("{SUPABASE_URL}/auth/v1/token?grant_type=password"))
        .header("apikey", SUPABASE_KEY)
        .json(&json!({ "email": email.trim(), "password": password }))
        .send()
        .await
        .map_err(|error| format!("登录请求失败：{error}"))?;
    parse_session(response).await
}

async fn refresh_session(refresh_token: String) -> Result<AuthSession, String> {
    let response = client()
        .post(format!(
            "{SUPABASE_URL}/auth/v1/token?grant_type=refresh_token"
        ))
        .header("apikey", SUPABASE_KEY)
        .json(&json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .map_err(|error| format!("刷新登录会话失败：{error}"))?;
    parse_session(response).await
}

pub async fn restore_session() -> Option<AuthSession> {
    let stored = load_session()?;
    if stored.refresh_token.is_empty() {
        return Some(stored);
    }

    match refresh_session(stored.refresh_token.clone()).await {
        Ok(session) => {
            if let Err(error) = save_session(&session) {
                log::warn!("保存刷新后的登录会话失败: {error}");
            }
            Some(session)
        }
        Err(error) => {
            log::warn!("刷新登录会话失败，暂时使用本地会话: {error}");
            Some(stored)
        }
    }
}

pub async fn send_otp(email: String) -> Result<(), String> {
    if email.trim().is_empty() {
        return Err("请输入邮箱".into());
    }
    let response = client()
        .post(format!("{SUPABASE_URL}/auth/v1/otp"))
        .header("apikey", SUPABASE_KEY)
        .json(&json!({ "email": email.trim(), "create_user": true }))
        .send()
        .await
        .map_err(|error| format!("发送验证码失败：{error}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        let value: Value = response.json().await.unwrap_or_default();
        Err(error_message(&value))
    }
}

pub async fn verify_otp(email: String, token: String) -> Result<AuthSession, String> {
    if email.trim().is_empty() || token.trim().is_empty() {
        return Err("请输入邮箱和验证码".into());
    }
    let response = client()
        .post(format!("{SUPABASE_URL}/auth/v1/verify"))
        .header("apikey", SUPABASE_KEY)
        .json(&json!({
            "email": email.trim(),
            "token": token.trim(),
            "type": "email"
        }))
        .send()
        .await
        .map_err(|error| format!("验证码校验失败：{error}"))?;
    parse_session(response).await
}

async fn fetch_tier(access_token: &str, user_id: &str) -> String {
    let response = client()
        .get(format!(
            "{SUPABASE_URL}/rest/v1/user_tiers?select=tier&user_id=eq.{user_id}&limit=1"
        ))
        .header("apikey", SUPABASE_KEY)
        .bearer_auth(access_token)
        .send()
        .await;
    let Ok(response) = response else {
        return "basic".into();
    };
    let value: Value = response.json().await.unwrap_or_default();
    value
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item["tier"].as_str())
        .unwrap_or("basic")
        .to_string()
}

pub fn load_session() -> Option<AuthSession> {
    let bytes = std::fs::read(data_dir().join(SESSION_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_session(session: &AuthSession) -> Result<(), String> {
    std::fs::create_dir_all(data_dir()).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(session).map_err(|error| error.to_string())?;
    std::fs::write(data_dir().join(SESSION_FILE), bytes).map_err(|error| error.to_string())
}

pub fn clear_session() -> Result<(), String> {
    let path = data_dir().join(SESSION_FILE);
    if path.exists() {
        std::fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

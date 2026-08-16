use super::auth::{self, AuthSession, SUPABASE_KEY, SUPABASE_URL};
use crate::auto_reply;
use crate::auto_reply::models::AutoReplySettings;
use crate::storage::{self, Account};
use reqwest::{RequestBuilder, Response};
use serde_json::{json, Value};

pub struct CloudDownload {
    pub accounts: Vec<Account>,
    pub settings: Option<AutoReplySettings>,
    pub downloaded_count: usize,
}

fn authorized(request: RequestBuilder, session: &AuthSession) -> RequestBuilder {
    request
        .header("apikey", SUPABASE_KEY)
        .bearer_auth(&session.access_token)
}

fn response_error(value: &Value, fallback: &str) -> String {
    value["message"]
        .as_str()
        .or_else(|| value["error_description"].as_str())
        .or_else(|| value["hint"].as_str())
        .unwrap_or(fallback)
        .to_string()
}

async fn parse_response(response: Response, action: &str) -> Result<Value, String> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("{action}响应读取失败：{error}"))?;
    let value = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).map_err(|error| format!("{action}响应解析失败：{error}"))?
    };
    if status.is_success() {
        Ok(value)
    } else {
        Err(format!("{action}失败：{}", response_error(&value, &text)))
    }
}

async fn upsert(
    session: &AuthSession,
    table: &str,
    conflict: &str,
    body: &Value,
) -> Result<(), String> {
    let response = authorized(
        auth::client()
            .post(format!("{SUPABASE_URL}/rest/v1/{table}"))
            .query(&[("on_conflict", conflict)])
            .header("Prefer", "resolution=merge-duplicates"),
        session,
    )
    .json(body)
    .send()
    .await
    .map_err(|error| format!("上传 {table} 失败：{error}"))?;
    parse_response(response, &format!("上传 {table}")).await?;
    Ok(())
}

async fn select(
    session: &AuthSession,
    table: &str,
    columns: &str,
    order: Option<&str>,
) -> Result<Value, String> {
    let user_filter = format!("eq.{}", session.user_id);
    let mut request = auth::client()
        .get(format!("{SUPABASE_URL}/rest/v1/{table}"))
        .query(&[("select", columns), ("user_id", user_filter.as_str())]);
    if let Some(order) = order {
        request = request.query(&[("order", order)]);
    }
    let response = authorized(request, session)
        .send()
        .await
        .map_err(|error| format!("下载 {table} 失败：{error}"))?;
    parse_response(response, &format!("下载 {table}")).await
}

pub async fn upload_all(session: AuthSession) -> Result<String, String> {
    let accounts = storage::get_accounts().await?;
    if !accounts.is_empty() {
        let rows = accounts
            .iter()
            .map(|account| {
                json!({
                    "user_id": &session.user_id,
                    "uid": account.uid,
                    "name": account.name,
                    "avatar": account.avatar,
                    "cookie": account.cookie,
                    "active": account.active,
                    "created_at": account.created_at,
                })
            })
            .collect::<Vec<_>>();
        upsert(
            &session,
            "bilibili_accounts",
            "user_id,uid",
            &Value::Array(rows),
        )
        .await?;
    }

    let settings = auto_reply::get_settings().await?;
    upsert(
        &session,
        "auto_reply_settings",
        "user_id",
        &json!({
            "user_id": &session.user_id,
            "settings": settings,
        }),
    )
    .await?;

    let replied_set = auto_reply::get_replied_set().await?;
    let liked_set = auto_reply::get_liked_set().await?;
    upsert(
        &session,
        "automation_state",
        "user_id",
        &json!({
            "user_id": &session.user_id,
            "replied_set": replied_set,
            "liked_set": liked_set,
        }),
    )
    .await?;

    Ok(format!(
        "已上传 {} 个账号和全部自动回复数据",
        accounts.len()
    ))
}

pub async fn download_all(session: AuthSession) -> Result<CloudDownload, String> {
    let account_value = select(
        &session,
        "bilibili_accounts",
        "uid,name,avatar,cookie,active,created_at",
        Some("created_at.asc"),
    )
    .await?;
    let mut cloud_accounts: Vec<Account> = serde_json::from_value(account_value)
        .map_err(|error| format!("解析云端账号失败：{error}"))?;

    let accounts = if cloud_accounts.is_empty() {
        storage::get_accounts().await?
    } else {
        let mut active_found = false;
        for account in &mut cloud_accounts {
            if account.active && !active_found {
                active_found = true;
            } else {
                account.active = false;
            }
        }
        if !active_found {
            cloud_accounts[0].active = true;
        }
        storage::sync_accounts(cloud_accounts.clone()).await?
    };
    let mut downloaded_count = cloud_accounts.len();

    let settings_value = select(&session, "auto_reply_settings", "settings", None).await?;
    let settings = settings_value
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("settings"))
        .cloned()
        .map(serde_json::from_value::<AutoReplySettings>)
        .transpose()
        .map_err(|error| format!("解析云端自动回复设置失败：{error}"))?;
    if let Some(settings) = settings.as_ref() {
        auto_reply::save_settings(settings.clone()).await?;
        downloaded_count += 1;
    }

    let state_value = select(&session, "automation_state", "replied_set,liked_set", None).await?;
    if let Some(state) = state_value.as_array().and_then(|rows| rows.first()) {
        if let Some(value) = state.get("replied_set") {
            let entries: Vec<String> = serde_json::from_value(value.clone())
                .map_err(|error| format!("解析云端已回复记录失败：{error}"))?;
            if !entries.is_empty() {
                auto_reply::merge_replied_set(entries).await?;
                downloaded_count += 1;
            }
        }
        if let Some(value) = state.get("liked_set") {
            let entries: Vec<String> = serde_json::from_value(value.clone())
                .map_err(|error| format!("解析云端已点赞记录失败：{error}"))?;
            if !entries.is_empty() {
                auto_reply::merge_liked_set(entries).await?;
                downloaded_count += 1;
            }
        }
    }

    Ok(CloudDownload {
        accounts,
        settings,
        downloaded_count,
    })
}

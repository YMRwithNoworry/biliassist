#![allow(dead_code)]

use chrono::Utc;
use crate::auto_reply::http::{get_http_client, resp_to_json};

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

fn extract_wbi_key(url: &str) -> String {
    url.rsplit('/')
        .next()
        .and_then(|s| s.split('.').next())
        .unwrap_or("")
        .to_string()
}

pub async fn get_wbi_keys(cookie: &str) -> Result<(String, String), String> {
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

    let img_key = extract_wbi_key(img_url);
    let sub_key = extract_wbi_key(sub_url);

    log::info!("获取WBI keys成功: img_key={}, sub_key={}",
        if img_key.len() > 8 { &img_key[..8] } else { &img_key },
        if sub_key.len() > 8 { &sub_key[..8] } else { &sub_key });

    Ok((img_key, sub_key))
}

fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '!' | '\'' | '(' | ')' | '*' => {
                result.push(c);
            }
            ' ' => result.push_str("%20"),
            c if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' => {
                result.push(c);
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

pub fn sign_wbi_params(
    params: &[(String, String)],
    img_key: &str,
    sub_key: &str,
) -> Vec<(String, String)> {
    let mixin_key = get_mixin_key(&format!("{}{}", img_key, sub_key));
    let mut map: Vec<(String, String)> = params.to_vec();
    let wts = Utc::now().timestamp().to_string();
    map.push(("wts".to_string(), wts));

    map.sort_by(|a, b| a.0.cmp(&b.0));

    let query: String = map.iter()
        .map(|(k, v)| format!("{}={}", k, url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let hash = md5_hex(&format!("{}{}", query, mixin_key));
    map.push(("w_rid".to_string(), hash));

    map
}

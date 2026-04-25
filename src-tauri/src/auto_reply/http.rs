use reqwest::Client;

/// 获取配置好的HTTP客户端
pub fn get_http_client() -> Client {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to create HTTP client")
}

/// 从cookie中提取CSRF token (bili_jct)
pub fn extract_csrf(cookie: &str) -> String {
    for pair in cookie.split(';') {
        let pair = pair.trim();
        if pair.starts_with("bili_jct=") {
            return pair.trim_start_matches("bili_jct=").to_string();
        }
    }
    log::warn!("cookie 中未找到 bili_jct，cookie 长度: {}, 前100字符: {}",
        cookie.len(),
        &cookie[..cookie.len().min(100)]);
    String::new()
}

/// 安全获取JSON（先text再parse，避免解码失败）
pub async fn resp_to_json(resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("解析JSON失败: {} | body={}", e, &text[..text.len().min(200)]))
}

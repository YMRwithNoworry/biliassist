use super::models::AiReplyConfig;
use serde::{Deserialize, Serialize};

/// OpenAI 兼容的聊天完成请求
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI 兼容的聊天完成响应
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize)]
struct ChatMessageContent {
    content: String,
}

/// 使用 AI 生成回复内容
///
/// 调用 OpenAI 兼容接口（支持任意 base_url），根据用户消息生成回复。
/// 每次调用都会使用用户配置的 system_prompt 和 prompt_template。
pub async fn generate_reply(
    config: &AiReplyConfig,
    user_name: &str,
    user_message: &str,
    source_name: &str,
) -> Result<String, String> {
    if config.api_key.is_empty() {
        return Err("AI API Key 未配置".to_string());
    }
    if config.base_url.is_empty() {
        return Err("AI Base URL 未配置".to_string());
    }

    // 系统提示词：设定 AI 角色与回复风格
    let system_prompt = config.effective_system_prompt();

    // 回复提示词模板：替换变量后作为用户消息发送给 AI
    let prompt_template = config.effective_prompt_template();
    let user_prompt = prompt_template
        .replace("{用户名}", user_name)
        .replace(
            "{消息内容}",
            if user_message.is_empty() {
                "(无内容)"
            } else {
                user_message
            },
        )
        .replace("{来源}", source_name);

    let request = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        max_tokens: 200,
        temperature: 0.7,
    };

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("AI 请求失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("AI 请求失败 (HTTP {}): {}", status, body));
    }

    let chat_resp: ChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析 AI 响应失败: {}", e))?;

    let reply = chat_resp
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .ok_or("AI 未返回任何回复内容")?;

    Ok(reply)
}

/// 测试 AI 配置是否可用
pub async fn test_ai_config(config: &AiReplyConfig) -> Result<String, String> {
    generate_reply(config, "测试用户", "你好啊，你的视频真好看！", "评论").await
}

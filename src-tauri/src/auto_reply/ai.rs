use super::models::{AiApiFormat, AiReplyConfig};
use serde_json::{json, Value};

const MAX_OUTPUT_TOKENS: u32 = 200;
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug)]
struct PreparedRequest {
    url: String,
    headers: Vec<(&'static str, String)>,
    body: Value,
}

fn endpoint_path(api_format: AiApiFormat) -> &'static str {
    match api_format {
        AiApiFormat::OpenAiChatCompletions => "chat/completions",
        AiApiFormat::OpenAiCompletions => "completions",
        AiApiFormat::OpenAiResponses => "responses",
        AiApiFormat::AnthropicMessages => "messages",
    }
}

fn endpoint_url(base_url: &str, api_format: AiApiFormat) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    let path = endpoint_path(api_format);
    let full_path = format!("/{path}");

    if base_url.ends_with(&full_path) {
        base_url.to_string()
    } else {
        format!("{base_url}/{path}")
    }
}

fn completion_prompt(system_prompt: &str, user_prompt: &str) -> String {
    format!("系统指令：{system_prompt}\n\n用户：{user_prompt}\n\n助手：")
}

fn prepare_request(
    config: &AiReplyConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<PreparedRequest, String> {
    if config.api_key.trim().is_empty() {
        return Err("AI API Key 未配置".to_string());
    }
    if config.base_url.trim().is_empty() {
        return Err("AI Base URL 未配置".to_string());
    }
    if config.model.trim().is_empty() {
        return Err("AI 模型名称未配置".to_string());
    }

    let (headers, body) = match config.api_format {
        AiApiFormat::OpenAiChatCompletions => (
            vec![("Authorization", format!("Bearer {}", config.api_key))],
            json!({
                "model": config.model,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_prompt }
                ],
                "max_tokens": MAX_OUTPUT_TOKENS,
                "temperature": 0.7
            }),
        ),
        AiApiFormat::OpenAiCompletions => (
            vec![("Authorization", format!("Bearer {}", config.api_key))],
            json!({
                "model": config.model,
                "prompt": completion_prompt(system_prompt, user_prompt),
                "max_tokens": MAX_OUTPUT_TOKENS,
                "temperature": 0.7
            }),
        ),
        AiApiFormat::OpenAiResponses => (
            vec![("Authorization", format!("Bearer {}", config.api_key))],
            json!({
                "model": config.model,
                "instructions": system_prompt,
                "input": user_prompt,
                "max_output_tokens": MAX_OUTPUT_TOKENS
            }),
        ),
        AiApiFormat::AnthropicMessages => (
            vec![
                ("x-api-key", config.api_key.clone()),
                ("anthropic-version", ANTHROPIC_VERSION.to_string()),
            ],
            json!({
                "model": config.model,
                "system": system_prompt,
                "messages": [
                    { "role": "user", "content": user_prompt }
                ],
                "max_tokens": MAX_OUTPUT_TOKENS,
                "temperature": 0.7
            }),
        ),
    };

    Ok(PreparedRequest {
        url: endpoint_url(&config.base_url, config.api_format),
        headers,
        body,
    })
}

fn trimmed_text(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn extract_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => trimmed_text(text),
        Value::Array(items) => {
            let text = items
                .iter()
                .filter_map(extract_text)
                .collect::<Vec<_>>()
                .join("");
            trimmed_text(&text)
        }
        Value::Object(object) => object
            .get("text")
            .and_then(extract_text)
            .or_else(|| object.get("content").and_then(extract_text)),
        _ => None,
    }
}

fn extract_reply(api_format: AiApiFormat, response: &Value) -> Result<String, String> {
    let reply = match api_format {
        AiApiFormat::OpenAiChatCompletions => response
            .pointer("/choices/0/message/content")
            .and_then(extract_text),
        AiApiFormat::OpenAiCompletions => {
            response.pointer("/choices/0/text").and_then(extract_text)
        }
        AiApiFormat::OpenAiResponses => response
            .get("output_text")
            .and_then(extract_text)
            .or_else(|| response.get("output").and_then(extract_text)),
        AiApiFormat::AnthropicMessages => response.get("content").and_then(extract_text),
    };

    reply.ok_or_else(|| {
        response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .map(|message| format!("AI 未返回可用回复: {message}"))
            .unwrap_or_else(|| "AI 未返回可用回复".to_string())
    })
}

fn build_user_prompt(
    config: &AiReplyConfig,
    user_name: &str,
    user_message: &str,
    source_name: &str,
) -> String {
    config
        .effective_prompt_template()
        .replace("{用户名}", user_name)
        .replace(
            "{消息内容}",
            if user_message.is_empty() {
                "(无内容)"
            } else {
                user_message
            },
        )
        .replace("{来源}", source_name)
}

/// 使用配置的 AI 服务生成回复内容。
///
/// 支持 OpenAI Chat Completions、OpenAI Completions、OpenAI Responses 和
/// Anthropic Messages 格式。Base URL 可以是 API 根路径，也可以直接填写完整端点。
pub async fn generate_reply(
    config: &AiReplyConfig,
    user_name: &str,
    user_message: &str,
    source_name: &str,
) -> Result<String, String> {
    let system_prompt = config.effective_system_prompt();
    let user_prompt = build_user_prompt(config, user_name, user_message, source_name);
    let request = prepare_request(config, &system_prompt, &user_prompt)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| format!("创建HTTP客户端失败: {error}"))?;

    let mut request_builder = client
        .post(&request.url)
        .header("Content-Type", "application/json")
        .json(&request.body);
    for (name, value) in request.headers {
        request_builder = request_builder.header(name, value);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|error| format!("AI 请求失败: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("读取 AI 响应失败: {error}"))?;

    if !status.is_success() {
        let body_preview = body.chars().take(500).collect::<String>();
        return Err(format!("AI 请求失败 (HTTP {status}): {body_preview}"));
    }

    let response: Value = serde_json::from_str(&body).map_err(|error| {
        let body_preview = body.chars().take(500).collect::<String>();
        format!("解析 AI 响应失败: {error} | body={body_preview}")
    })?;

    extract_reply(config.api_format, &response)
}

/// 测试 AI 配置是否可用。
pub async fn test_ai_config(config: &AiReplyConfig) -> Result<String, String> {
    generate_reply(config, "测试用户", "你好啊，你的视频真好看！", "评论").await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(api_format: AiApiFormat) -> AiReplyConfig {
        AiReplyConfig {
            enabled: true,
            api_format,
            base_url: "https://api.example.test/v1".to_string(),
            model: "example-model".to_string(),
            api_key: "test-key".to_string(),
            system_prompt: String::new(),
            prompt_template: String::new(),
        }
    }

    fn header<'a>(request: &'a PreparedRequest, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|(header_name, _)| *header_name == name)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn builds_openai_chat_completions_request() {
        let request = prepare_request(
            &config(AiApiFormat::OpenAiChatCompletions),
            "系统提示词",
            "用户提示词",
        )
        .unwrap();

        assert_eq!("https://api.example.test/v1/chat/completions", request.url);
        assert_eq!(Some("Bearer test-key"), header(&request, "Authorization"));
        assert_eq!("system", request.body["messages"][0]["role"]);
        assert_eq!("用户提示词", request.body["messages"][1]["content"]);
    }

    #[test]
    fn builds_openai_completions_request() {
        let request = prepare_request(
            &config(AiApiFormat::OpenAiCompletions),
            "系统提示词",
            "用户提示词",
        )
        .unwrap();

        assert_eq!("https://api.example.test/v1/completions", request.url);
        assert_eq!(Some("Bearer test-key"), header(&request, "Authorization"));
        assert!(request.body["prompt"]
            .as_str()
            .unwrap()
            .contains("系统提示词"));
        assert!(request.body["prompt"]
            .as_str()
            .unwrap()
            .contains("用户提示词"));
    }

    #[test]
    fn builds_openai_responses_request() {
        let request = prepare_request(
            &config(AiApiFormat::OpenAiResponses),
            "系统提示词",
            "用户提示词",
        )
        .unwrap();

        assert_eq!("https://api.example.test/v1/responses", request.url);
        assert_eq!(Some("Bearer test-key"), header(&request, "Authorization"));
        assert_eq!("系统提示词", request.body["instructions"]);
        assert_eq!("用户提示词", request.body["input"]);
        assert_eq!(
            Some(MAX_OUTPUT_TOKENS as u64),
            request.body["max_output_tokens"].as_u64()
        );
    }

    #[test]
    fn builds_anthropic_messages_request() {
        let request = prepare_request(
            &config(AiApiFormat::AnthropicMessages),
            "系统提示词",
            "用户提示词",
        )
        .unwrap();

        assert_eq!("https://api.example.test/v1/messages", request.url);
        assert_eq!(Some("test-key"), header(&request, "x-api-key"));
        assert_eq!(
            Some(ANTHROPIC_VERSION),
            header(&request, "anthropic-version")
        );
        assert_eq!(None, header(&request, "Authorization"));
        assert_eq!("系统提示词", request.body["system"]);
        assert_eq!("用户提示词", request.body["messages"][0]["content"]);
    }

    #[test]
    fn preserves_a_directly_configured_endpoint() {
        let mut config = config(AiApiFormat::OpenAiResponses);
        config.base_url = "https://api.example.test/v1/responses/".to_string();

        let request = prepare_request(&config, "系统提示词", "用户提示词").unwrap();

        assert_eq!("https://api.example.test/v1/responses", request.url);
    }

    #[test]
    fn extracts_text_from_each_api_format() {
        let cases = [
            (
                AiApiFormat::OpenAiChatCompletions,
                json!({ "choices": [{ "message": { "content": "Chat reply" } }] }),
                "Chat reply",
            ),
            (
                AiApiFormat::OpenAiCompletions,
                json!({ "choices": [{ "text": "Completion reply" }] }),
                "Completion reply",
            ),
            (
                AiApiFormat::OpenAiResponses,
                json!({
                    "output": [{
                        "type": "message",
                        "content": [{ "type": "output_text", "text": "Responses reply" }]
                    }]
                }),
                "Responses reply",
            ),
            (
                AiApiFormat::AnthropicMessages,
                json!({ "content": [{ "type": "text", "text": "Anthropic reply" }] }),
                "Anthropic reply",
            ),
        ];

        for (api_format, response, expected) in cases {
            assert_eq!(expected, extract_reply(api_format, &response).unwrap());
        }
    }
}

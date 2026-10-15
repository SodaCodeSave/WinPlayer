//! 插件网络请求代理：插件沙箱（Web Worker）无法直接跨域访问任意来源，
//! 所有插件发起的请求都经由这里由 Rust 后端转发。

use base64::Engine;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSpec {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub body_base64: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// "text"（默认）或 "base64"
    #[serde(default)]
    pub response_type: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseSpec {
    pub status: u16,
    pub ok: bool,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_base64: Option<String>,
}

/// 执行一次经 Rust 转发的 HTTP 请求（无 CORS 限制、可自定义任意请求头）
#[tauri::command]
pub async fn http_fetch(spec: HttpRequestSpec) -> Result<HttpResponseSpec, String> {
    let timeout = spec.timeout_ms.unwrap_or(15_000).clamp(1_000, 120_000);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout))
        .user_agent("WinPlayer/0.1")
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;

    let method = Method::from_bytes(spec.method.to_uppercase().as_bytes())
        .map_err(|e| format!("非法 HTTP 方法: {e}"))?;

    let mut request = client.request(method, &spec.url);
    for (key, value) in &spec.headers {
        request = request.header(key, value);
    }
    if let Some(body) = &spec.body {
        request = request.body(body.clone());
    } else if let Some(body_b64) = &spec.body_base64 {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(body_b64)
            .map_err(|e| format!("请求体 base64 解码失败: {e}"))?;
        request = request.body(bytes);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or("").to_string();

    // 同名响应头（典型如登录返回的多个 Set-Cookie）按出现顺序用换行拼接，
    // 否则 HashMap::insert 会只保留最后一个，插件就拿不到完整登录凭证。
    let mut headers: HashMap<String, String> = HashMap::new();
    for (key, value) in response.headers().iter() {
        if let Ok(text) = value.to_str() {
            let name = key.as_str().to_string();
            match headers.get_mut(&name) {
                Some(existing) => {
                    existing.push('\n');
                    existing.push_str(text);
                }
                None => {
                    headers.insert(name, text.to_string());
                }
            }
        }
    }

    if matches!(spec.response_type.as_deref(), Some("base64")) {
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("读取响应体失败: {e}"))?;
        Ok(HttpResponseSpec {
            status: status.as_u16(),
            ok: status.is_success(),
            status_text,
            headers,
            body: None,
            body_base64: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        })
    } else {
        let body = response
            .text()
            .await
            .map_err(|e| format!("读取响应体失败: {e}"))?;
        Ok(HttpResponseSpec {
            status: status.as_u16(),
            ok: status.is_success(),
            status_text,
            headers,
            body: Some(body),
            body_base64: None,
        })
    }
}

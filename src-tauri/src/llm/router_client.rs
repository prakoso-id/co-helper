use reqwest::Client;
use tauri::{AppHandle, Emitter};
use futures::StreamExt;
use tokio::sync::Notify;
use crate::state::Message;

pub async fn stream_chat(
    url: &str,
    model: &str,
    messages: Vec<Message>,
    app: &AppHandle,
    segment_id: &str,
    abort: std::sync::Arc<Notify>,
) -> Result<(), String> {
    let client = Client::new();
    let endpoint = format!("{}/v1/chat/completions", url);

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });

    let res = client
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("9router request failed: {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("9router error {status}: {text}"));
    }

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();
    let mut full_text = String::new();

    loop {
        tokio::select! {
            _ = abort.notified() => {
                let _ = app.emit("llm_end", serde_json::json!({
                    "segmentId": segment_id,
                    "fullText": full_text,
                }));
                return Ok(());
            }
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        while let Some(idx) = buffer.find('\n') {
                            let line: String = buffer.drain(..=idx).collect();
                            let line = line.trim();

                            if let Some(payload) = line.strip_prefix("data: ") {
                                if payload == "[DONE]" {
                                    let _ = app.emit("llm_end", serde_json::json!({
                                        "segmentId": segment_id,
                                        "fullText": full_text,
                                    }));
                                    return Ok(());
                                }

                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                                    if let Some(token) = v["choices"][0]["delta"]["content"].as_str() {
                                        if !token.is_empty() {
                                            full_text.push_str(token);
                                            let _ = app.emit("llm_token", token);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        let _ = app.emit("error", serde_json::json!({
                            "message": format!("Stream error: {e}"),
                            "source": "llm",
                        }));
                        return Err(format!("Stream error: {e}"));
                    }
                    None => {
                        let _ = app.emit("llm_end", serde_json::json!({
                            "segmentId": segment_id,
                            "fullText": full_text,
                        }));
                        return Ok(());
                    }
                }
            }
        }
    }
}

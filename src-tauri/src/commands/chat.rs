use std::time::Instant;
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use crate::DbState;
use crate::db::{self, Message};
use crate::llama::LlamaState;

#[derive(Debug, Serialize, Clone)]
pub struct TokenPayload {
    pub conversation_id: String,
    pub content: String,
    pub is_reasoning: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatCompletePayload {
    pub conversation_id: String,
    pub message: Message,
    pub stats: GenerationStats,
}

#[derive(Debug, Serialize, Clone)]
pub struct GenerationStats {
    pub total_seconds: f64,
    pub thinking_seconds: f64,
    pub token_count: u32,
    pub tokens_per_second: f64,
}

#[tauri::command]
pub fn get_messages(db: State<'_, DbState>, conversation_id: String) -> Result<Vec<Message>, String> {
    let conn = db.0.lock().unwrap();
    db::get_messages(&conn, &conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    db: State<'_, DbState>,
    llama: State<'_, LlamaState>,
    conversation_id: String,
    content: String,
    thinking: bool,
) -> Result<Message, String> {
    // Check server is running
    let port = {
        let model = llama.active_model.lock().unwrap();
        if model.is_none() {
            return Err("No model loaded. Please select a model first.".to_string());
        }
        *llama.port.lock().unwrap()
    };

    // Insert user message
    let model_name = llama.active_model.lock().unwrap().clone();
    let model_filename = model_name.as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .map(|f| f.to_string_lossy().to_string());

    {
        let conn = db.0.lock().unwrap();
        db::insert_message(&conn, &conversation_id, "user", &content, model_filename.as_deref())
            .map_err(|e| e.to_string())?;
    }

    // Build messages array from conversation history
    let messages_for_api = {
        let conn = db.0.lock().unwrap();
        let msgs = db::get_messages(&conn, &conversation_id).map_err(|e| e.to_string())?;
        msgs.iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect::<Vec<_>>()
    };

    // Set generating flag
    *llama.is_generating.lock().unwrap() = true;

    // POST to llama-server
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

    let mut body = serde_json::json!({
        "messages": messages_for_api,
        "stream": true,
    });
    if thinking {
        body["thinking_budget_tokens"] = serde_json::json!(1024);
    } else {
        body["thinking_budget_tokens"] = serde_json::json!(0);
    }

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to llama-server: {}", e))?;

    if !response.status().is_success() {
        *llama.is_generating.lock().unwrap() = false;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("llama-server error ({}): {}", status, text));
    }

    // Stream SSE response
    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut buffer = String::new();
    let start_time = Instant::now();
    let mut thinking_end: Option<Instant> = None;
    let mut was_reasoning = false;
    let mut token_count: u32 = 0;

    while let Some(chunk) = stream.next().await {
        // Check if generation was cancelled
        if !*llama.is_generating.lock().unwrap() {
            break;
        }

        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    continue;
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    // Check for server error (e.g. context size exceeded)
                    if let Some(err_msg) = parsed["error"]["message"].as_str() {
                        *llama.is_generating.lock().unwrap() = false;
                        let _ = app.emit("chat-error", serde_json::json!({
                            "conversation_id": conversation_id,
                            "error": err_msg,
                        }));
                        return Err(err_msg.to_string());
                    }

                    let delta = &parsed["choices"][0]["delta"];
                    if let Some(text) = delta["reasoning_content"].as_str() {
                        was_reasoning = true;
                        token_count += 1;
                        let _ = app.emit("chat-token", TokenPayload {
                            conversation_id: conversation_id.clone(),
                            content: text.to_string(),
                            is_reasoning: true,
                        });
                    }
                    if let Some(text) = delta["content"].as_str() {
                        if was_reasoning && thinking_end.is_none() {
                            thinking_end = Some(Instant::now());
                        }
                        token_count += 1;
                        full_response.push_str(text);
                        let _ = app.emit("chat-token", TokenPayload {
                            conversation_id: conversation_id.clone(),
                            content: text.to_string(),
                            is_reasoning: false,
                        });
                    }
                }
            }
        }
    }

    *llama.is_generating.lock().unwrap() = false;

    // Strip <think>...</think> blocks from the response
    let full_response = strip_think_blocks(&full_response);

    let total_seconds = start_time.elapsed().as_secs_f64();
    let thinking_seconds = thinking_end
        .map(|t| t.duration_since(start_time).as_secs_f64())
        .unwrap_or(0.0);
    let tokens_per_second = if total_seconds > 0.0 {
        token_count as f64 / total_seconds
    } else {
        0.0
    };

    let stats = GenerationStats {
        total_seconds,
        thinking_seconds,
        token_count,
        tokens_per_second,
    };

    // Don't save empty responses (e.g. cancelled during thinking)
    if full_response.is_empty() {
        let _ = app.emit("chat-complete", ChatCompletePayload {
            conversation_id: conversation_id.clone(),
            message: Message {
                id: String::new(),
                conversation_id: conversation_id.clone(),
                role: "assistant".to_string(),
                content: String::new(),
                model: model_filename,
                created_at: String::new(),
            },
            stats,
        });
        return Err("Generation cancelled or produced no output.".to_string());
    }

    // Save assistant message to DB
    let assistant_msg = {
        let conn = db.0.lock().unwrap();
        db::insert_message(&conn, &conversation_id, "assistant", &full_response, model_filename.as_deref())
            .map_err(|e| e.to_string())?
    };

    let _ = app.emit("chat-complete", ChatCompletePayload {
        conversation_id: conversation_id.clone(),
        message: assistant_msg.clone(),
        stats,
    });

    Ok(assistant_msg)
}

#[tauri::command]
pub fn stop_generation(llama: State<'_, LlamaState>) -> Result<(), String> {
    *llama.is_generating.lock().unwrap() = false;
    Ok(())
}

fn strip_think_blocks(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        result.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("</think>") {
            rest = &rest[start + end + 8..];
        } else {
            // Unclosed <think> tag — strip everything after it
            return result.trim().to_string();
        }
    }
    result.push_str(rest);
    result.trim().to_string()
}

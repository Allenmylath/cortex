use dioxus::prelude::*;
use super::types::*;
use super::playback::speaker_clear;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RaviEnvelope {
    pub label: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub id:   Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// Dispatch one text message received from the server into pipeline signals.
pub fn handle_ravi_event(text: &str, pipeline: &mut Pipeline) {
    // Raw server-side interruption (no RAVI label)
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(text) {
        if raw.get("type").and_then(|v| v.as_str()) == Some("interruption")
            && raw.get("label").is_none()
        {
            let was_bot_speaking = *pipeline.is_bot_speaking.read();
            pipeline.energy.set(0.0);
            pipeline.is_bot_speaking.set(false);
            pipeline.user_speaking.set(true);
            pipeline.activity.set(Activity::Listening);
            pipeline.bot_text.set(String::new());
            speaker_clear();
            // Only count and log as a true interruption when the bot was actually speaking.
            // VAD-onset interruptions where the bot was silent are covered by the
            // user-started-speaking RAVI event that follows.
            if was_bot_speaking {
                pipeline.interrupt_count.with_mut(|c| *c += 1);
                log_event(pipeline, "interrupt", Some("server"));
            }
            return;
        }
    }

    let envelope: RaviEnvelope = match serde_json::from_str(text) {
        Ok(e)  => e,
        Err(_) => return,
    };

    if envelope.label != "ravi" {
        return;
    }

    match envelope.msg_type.as_str() {
        "bot-ready" => {
            log_event(pipeline, "bot-ready", None);
        }
        "user-started-speaking" => {
            // State is owned by the client VAD (capture.rs). Only log for observability.
            log_event(pipeline, "user-started-speaking", Some("server"));
        }
        "user-stopped-speaking" => {
            // State is owned by the client VAD (capture.rs). Only log for observability.
            log_event(pipeline, "user-stopped-speaking", Some("server"));
        }
        "user-transcription" => {
            if let Some(data) = envelope.data {
                let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let is_final = data.get("final").and_then(|v| v.as_bool()).unwrap_or(false);
                pipeline.transcript.set(text.clone());
                if is_final {
                    let id = envelope.id.unwrap_or_else(uid);
                    pipeline.messages.with_mut(|v| v.push(ChatMessage {
                        id,
                        role:      "user".into(),
                        text:      text.clone(),
                        final_msg: true,
                    }));
                    pipeline.transcript.set(String::new());
                }
                log_event(pipeline, "user-transcription", Some(&text));
            }
        }
        "bot-llm-started" => {
            pipeline.activity.set(Activity::Processing);
            pipeline.messages.with_mut(|v| {
                if let Some(last) = v.last_mut() {
                    if last.role == "assistant" && !last.final_msg { return; }
                }
                v.push(ChatMessage {
                    id:        uid(),
                    role:      "assistant".into(),
                    text:      String::new(),
                    final_msg: false,
                });
            });
            log_event(pipeline, "bot-llm-started", None);
        }
        "bot-llm-stopped" => {
            log_event(pipeline, "bot-llm-stopped", None);
        }
        "bot-llm-text" => {
            if let Some(data) = envelope.data {
                let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                pipeline.bot_text.with_mut(|t| *t += &text);
                pipeline.messages.with_mut(|v| {
                    if let Some(last) = v.last_mut() {
                        if last.role == "assistant" && !last.final_msg {
                            last.text.push_str(&text);
                            return;
                        }
                    }
                    v.push(ChatMessage {
                        id:        uid(),
                        role:      "assistant".into(),
                        text:      text.clone(),
                        final_msg: false,
                    });
                });
                log_event(pipeline, "bot-llm-text", Some(&text));
            }
        }
        "bot-started-speaking" => {
            pipeline.activity.set(Activity::Speaking);
            pipeline.is_bot_speaking.set(true);
            log_event(pipeline, "bot-started-speaking", None);
        }
        "bot-stopped-speaking" => {
            pipeline.is_bot_speaking.set(false);
            log_event(pipeline, "bot-stopped-speaking", None);
        }
        "bot-transcription" => {
            if let Some(data) = envelope.data {
                let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let id   = envelope.id.unwrap_or_else(uid);
                pipeline.messages.with_mut(|v| {
                    if let Some(last) = v.last_mut() {
                        if last.role == "assistant" && !last.final_msg {
                            last.text      = text.clone();
                            last.final_msg = true;
                            return;
                        }
                    }
                    v.push(ChatMessage {
                        id,
                        role:      "assistant".into(),
                        text:      text.clone(),
                        final_msg: true,
                    });
                });
                pipeline.bot_text.set(String::new());
                log_event(pipeline, "bot-transcription", Some(&text));
            }
        }
        "server-message" => {
            if let Some(data) = envelope.data {
                let data_str = data.to_string();
                let msg_type = data.get("type").and_then(|v| v.as_str())
                    .unwrap_or("unknown").to_string();

                pipeline.server_messages.with_mut(|v| v.push(ServerMessage {
                    msg_type: msg_type.clone(),
                    data:     data_str,
                }));

                match msg_type.as_str() {
                    "function-call-start"     => log_event(pipeline, "function-call-start", None),
                    "function-call-end"       => log_event(pipeline, "function-call-end", None),
                    "function-call-in-progress" => {
                        let id   = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = data.get("function_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        pipeline.function_calls.with_mut(|v| v.push(FunctionCall {
                            id: id.clone(), name, state: "in-progress".into(),
                            payload: Some(data.to_string()),
                        }));
                        log_event(pipeline, "function-call-in-progress", None);
                    }
                    "function-call-result" => {
                        let id   = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = data.get("function_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        pipeline.function_calls.with_mut(|v| {
                            if let Some(fc) = v.iter_mut().find(|f| f.id == id) {
                                fc.state = "completed".into();
                            } else {
                                v.push(FunctionCall {
                                    id, name, state: "completed".into(),
                                    payload: Some(data.to_string()),
                                });
                            }
                        });
                        log_event(pipeline, "function-call-result", None);
                    }
                    "function-call-raw-result" => {
                        log_event(pipeline, "function-call-raw-result", None);
                    }
                    _ => {}
                }
            }
        }
        "error" | "error-response" => {
            if let Some(data) = envelope.data {
                let err = data.get("error").and_then(|v| v.as_str())
                    .unwrap_or("unknown").to_string();
                pipeline.last_error.set(Some(err.clone()));
                log_event(pipeline, &envelope.msg_type, Some(&err));
            }
        }
        _ => {
            log_event(pipeline, &format!("unhandled: {}", envelope.msg_type), None);
        }
    }
}

pub fn log_event(pipeline: &mut Pipeline, event_type: &str, detail: Option<&str>) {
    let now = js_sys::Date::now() as u64;
    pipeline.events.with_mut(|v| {
        v.push(RaviEvent {
            id:         uid(),
            ts:         now,
            event_type: event_type.to_string(),
            detail:     detail.map(|s| s.to_string()),
        });
    });
}

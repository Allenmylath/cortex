use dioxus::prelude::*;
use dioxus::fullstack::{CloseCode, Message, UseWebsocket};

#[derive(Clone, Copy, PartialEq)]
pub enum Activity {
    Idle,
    Listening,
    Processing,
    Speaking,
}

#[derive(Clone, PartialEq)]
pub struct ChatMessage {
    pub id:        String,
    pub role:      String,
    pub text:      String,
    pub final_msg: bool,
}

#[derive(Clone, PartialEq)]
pub struct FunctionCall {
    pub id:      String,
    pub name:    String,
    pub state:   String,
    pub payload: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct RaviEvent {
    pub id:         String,
    pub ts:         u64,
    pub event_type: String,
    pub detail:     Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct ServerMessage {
    pub msg_type: String,
    pub data:     String,
}

/// Reactive handle to the voice pipeline. All fields are `Signal`s.
#[derive(Clone, Copy)]
pub struct Pipeline {
    pub status:          Signal<&'static str>,
    pub activity:        Signal<Activity>,
    pub energy:          Signal<f32>,
    pub messages:        Signal<Vec<ChatMessage>>,
    pub transcript:      Signal<String>,
    pub turn_count:      Signal<u32>,
    pub interrupt_count: Signal<u32>,
    pub buffered_ms:     Signal<u32>,
    pub vad_prob:        Signal<f32>,
    pub last_error:      Signal<Option<String>>,
    pub function_calls:  Signal<Vec<FunctionCall>>,
    pub events:          Signal<Vec<RaviEvent>>,
    pub server_messages: Signal<Vec<ServerMessage>>,
    pub bot_text:        Signal<String>,
    pub is_bot_speaking: Signal<bool>,
    pub pipeline_texts:  Signal<Vec<String>>,
    pub user_speaking:   Signal<bool>,

    // internal — used by hook and audio tasks only
    pub connect_trigger: Signal<u32>,
    pub ws:              UseWebsocket<String, String>,
    pub tasks_started:   Signal<bool>,
}

impl Pipeline {
    pub fn connect(mut self) {
        if self.ws.connecting() || !self.ws.is_closed() {
            return;
        }
        self.connect_trigger.with_mut(|c| *c += 1);
    }

    pub fn disconnect(self) {
        let msg = serde_json::json!({ "label": "ravi", "type": "disconnect-bot" });
        let ws = self.ws;
        spawn(async move {
            let _ = ws.send_raw(Message::Text(msg.to_string())).await;
            let _ = ws.send_raw(Message::Close {
                code:   CloseCode::Normal,
                reason: "Session ended by user".into(),
            }).await;
        });
    }

    pub fn interrupt(self) {
        #[cfg(target_arch = "wasm32")]
        super::playback::speaker_clear();

        let ws = self.ws;
        spawn(async move {
            let _ = ws.send_raw(Message::Text(
                r#"{"type":"client_interruption"}"#.into(),
            )).await.ok();
        });
    }

    pub fn send_text(self, text: String) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() { return; }
        let msg = serde_json::json!({
            "label": "ravi",
            "type":  "send-text",
            "id":    uid(),
            "data":  {
                "content": trimmed,
                "options": { "run_immediately": true, "audio_response": true }
            }
        });
        let ws = self.ws;
        spawn(async move {
            let _ = ws.send_raw(Message::Text(msg.to_string())).await.ok();
        });
    }
}

pub fn uid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:08x}", n)
}

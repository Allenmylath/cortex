use dioxus::prelude::*;
use dioxus::fullstack::{
    use_websocket, Message, ServerFnError, WebSocketOptions, WebsocketState,
};

use crate::pipeline::pipeline_ws;
use super::types::*;

#[cfg(target_arch = "wasm32")]
use super::{capture::stop_mic, client::start_audio_tasks};

/// Dioxus hook — owns the WebSocket and all audio tasks for the voice pipeline.
/// Call once at the page root; pass the returned `Pipeline` via context.
pub fn use_pipeline() -> Pipeline {
    let mut status          = use_signal(|| "idle");
    let mut activity        = use_signal(|| Activity::Idle);
    let mut energy          = use_signal(|| 0.0_f32);
    let mut messages        = use_signal(|| Vec::<ChatMessage>::new());
    let mut transcript      = use_signal(|| String::new());
    let mut tasks_started   = use_signal(|| false);
    let mut turn_count      = use_signal(|| 0u32);
    let mut interrupt_count = use_signal(|| 0u32);
    let mut buffered_ms     = use_signal(|| 0u32);
    let mut vad_prob        = use_signal(|| 0.0_f32);
    let mut last_error      = use_signal(|| None::<String>);
    let mut function_calls  = use_signal(|| Vec::<FunctionCall>::new());
    let mut events          = use_signal(|| Vec::<RaviEvent>::new());
    let mut server_messages = use_signal(|| Vec::<ServerMessage>::new());
    let mut bot_text        = use_signal(|| String::new());
    let mut is_bot_speaking = use_signal(|| false);
    let mut pipeline_texts  = use_signal(|| Vec::<String>::new());
    let mut user_speaking   = use_signal(|| false);
    let mut connect_trigger = use_signal(|| 0u32);

    let ws = use_websocket(move || {
        let trigger = connect_trigger();
        async move {
            if trigger == 0 {
                return Err(ServerFnError::new("not connected").into());
            }
            pipeline_ws(WebSocketOptions::new()).await
        }
    });

    use_effect(move || {
        let state = ws.status().cloned();

        match state {
            WebsocketState::Connecting => {
                status.set("connecting");
            }

            WebsocketState::Open => {
                status.set("connected");
                let ws_ready = ws;
                spawn(async move {
                    let ready = serde_json::json!({
                        "label": "ravi",
                        "type":  "client-ready",
                        "id":    uid(),
                        "data":  {
                            "version": "1.2.0",
                            "about": {
                                "library":         "rustvani-dioxus",
                                "library_version": "0.1.0"
                            }
                        }
                    });
                    let _ = ws_ready.send_raw(Message::Text(ready.to_string())).await;
                });

                if !tasks_started() {
                    tasks_started.set(true);
                    #[cfg(target_arch = "wasm32")]
                    start_audio_tasks(Pipeline {
                        status, activity, energy, messages, transcript,
                        turn_count, interrupt_count, buffered_ms, vad_prob,
                        last_error, function_calls, events, server_messages,
                        bot_text, is_bot_speaking, pipeline_texts, user_speaking,
                        connect_trigger, ws, tasks_started,
                    });
                }
            }

            WebsocketState::Closed => {
                status.set("idle");
                activity.set(Activity::Idle);
                energy.set(0.0);
                is_bot_speaking.set(false);
                user_speaking.set(false);
                tasks_started.set(false);
                connect_trigger.set(0);
                #[cfg(target_arch = "wasm32")]
                stop_mic();
            }

            WebsocketState::FailedToConnect => {
                if tasks_started() {
                    status.set("error");
                    tasks_started.set(false);
                    #[cfg(target_arch = "wasm32")]
                    stop_mic();
                } else {
                    status.set("idle");
                }
                activity.set(Activity::Idle);
                energy.set(0.0);
                is_bot_speaking.set(false);
                user_speaking.set(false);
            }

            WebsocketState::Closing => {}
        }
    });

    Pipeline {
        status, activity, energy, messages, transcript,
        turn_count, interrupt_count, buffered_ms, vad_prob,
        last_error, function_calls, events, server_messages,
        bot_text, is_bot_speaking, pipeline_texts, user_speaking,
        connect_trigger, ws, tasks_started,
    }
}

#[component]
pub fn MicCapture() -> Element {
    rsx! {
        Link {
            class: "bg-slate-800 text-white px-4 py-2 rounded-lg text-sm font-medium \
                    flex items-center gap-2 hover:bg-slate-700 transition-colors",
            to: crate::Route::VoicePage {},
            "🎙 Talk to AI"
        }
    }
}

//! Voice pipeline hook and types.
//!
//! `use_pipeline()` owns the WebSocket (via `use_websocket`), all audio
//! capture/playback, and RAVI event parsing. Components only read signals.

use dioxus::prelude::*;
use dioxus::fullstack::{
    use_websocket, CloseCode, Message, ServerFnError, UseWebsocket, WebSocketOptions,
    Websocket, WebsocketState,
};

use crate::pipeline::pipeline_ws;

// ── wasm-only audio imports ─────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
use {
    std::{cell::RefCell, rc::Rc},
    wasm_bindgen::{closure::Closure, JsCast, JsValue},
    wasm_bindgen_futures::{spawn_local, JsFuture},
    web_sys::{
        AudioContext, AudioContextOptions,
        MediaStreamConstraints, ScriptProcessorNode,
    },
};

// ── Public types ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Activity {
    Idle,
    Listening,
    Processing,
    Speaking,
}

#[derive(Clone, PartialEq)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    pub final_msg: bool,
}

#[derive(Clone, PartialEq)]
pub struct FunctionCall {
    pub id: String,
    pub name: String,
    pub state: String,
    pub payload: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct RaviEvent {
    pub id: String,
    pub ts: u64,
    pub event_type: String,
    pub detail: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct ServerMessage {
    pub msg_type: String,
    pub data: String,
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

    connect_trigger: Signal<u32>,
    ws:              UseWebsocket<String, String>,
    tasks_started:   Signal<bool>,
}

impl Pipeline {
    /// Open the WebSocket. Safe to call multiple times.
    pub fn connect(mut self) {
        if self.ws.connecting() || !self.ws.is_closed() {
            return;
        }
        self.connect_trigger.with_mut(|c| *c += 1);
    }

    /// Send disconnect-bot and close the WebSocket.
    pub fn disconnect(self) {
        #[cfg(target_arch = "wasm32")]
        stop_mic();

        let msg = serde_json::json!({ "label": "ravi", "type": "disconnect-bot" });
        let ws = self.ws;
        spawn(async move {
            let _ = ws.send_raw(Message::Text(msg.to_string())).await;
            let _ = ws.send_raw(Message::Close {
                code: CloseCode::Normal,
                reason: "Session ended by user".into(),
            }).await;
        });
    }

    /// Interrupt bot playback and notify the server.
    pub fn interrupt(self) {
        #[cfg(target_arch = "wasm32")]
        {
            PLAY.with(|p| {
                if let Some((ctx, next_start, _)) = p.borrow().as_ref() {
                    *next_start.borrow_mut() = ctx.current_time();
                }
            });
        }

        let ws = self.ws;
        spawn(async move {
            let _ = ws.send_raw(Message::Text(
                r#"{"type":"client_interruption"}"#.into(),
            )).await.ok();
        });
    }

    /// Send a typed user message as a final user-transcription.
    pub fn send_text(self, text: String) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() { return; }
        let msg = serde_json::json!({
            "label": "ravi",
            "type": "send-text",
            "id": uid(),
            "data": {
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

// ── Hook ────────────────────────────────────────────────────────────────────

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
                        "type": "client-ready",
                        "id": uid(),
                        "data": {
                            "version": "1.2.0",
                            "about": {
                                "library": "rustvani-dioxus",
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
                        status,
                        activity,
                        energy,
                        messages,
                        transcript,
                        turn_count,
                        interrupt_count,
                        buffered_ms,
                        vad_prob,
                        last_error,
                        function_calls,
                        events,
                        server_messages,
                        bot_text,
                        is_bot_speaking,
                        connect_trigger,
                        ws,
                        tasks_started,
                    });
                }
            }

            WebsocketState::Closed => {
                status.set("idle");
                activity.set(Activity::Idle);
                energy.set(0.0);
                is_bot_speaking.set(false);
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
            }

            WebsocketState::Closing => {}
        }
    });

    Pipeline {
        status,
        activity,
        energy,
        messages,
        transcript,
        turn_count,
        interrupt_count,
        buffered_ms,
        vad_prob,
        last_error,
        function_calls,
        events,
        server_messages,
        bot_text,
        is_bot_speaking,
        connect_trigger,
        ws,
        tasks_started,
    }
}

// ── MicCapture nav button ───────────────────────────────────────────────────

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

// ── Audio tasks (wasm32) ────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
struct MicResources {
    _ctx:    AudioContext,
    _stream: web_sys::MediaStream,
    _proc:   ScriptProcessorNode,
    _cb:     Closure<dyn FnMut(web_sys::AudioProcessingEvent)>,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static MIC:  RefCell<Option<MicResources>>                     = RefCell::new(None);
    static PLAY: RefCell<Option<(AudioContext, Rc<RefCell<f64>>, Signal<u32>)>> = RefCell::new(None);
}

#[cfg(target_arch = "wasm32")]
fn start_audio_tasks(pipeline: Pipeline) {
    let play_ctx = match AudioContext::new() {
        Ok(ctx) => ctx,
        Err(_)  => return,
    };
    let next_start = Rc::new(RefCell::new(0.0_f64));
    let buffered_sig = pipeline.buffered_ms;
    PLAY.with(|p| *p.borrow_mut() = Some((play_ctx.clone(), next_start.clone(), buffered_sig)));

    // mic → pipeline
    let ws_mic = pipeline.ws;
    let mut activity_mic = pipeline.activity;
    spawn_local(async move {
        if start_mic(ws_mic).await.is_ok() {
            activity_mic.set(Activity::Listening);
        }
    });

    // pipeline → speaker + RAVI parser
    let ctx = play_ctx;
    let ns  = next_start;
    let mut p_recv = pipeline;

    spawn_local(async move {
        loop {
            match p_recv.ws.recv_raw().await {
                Ok(Message::Binary(bytes)) => {
                    p_recv.energy.set(rms_energy(&bytes));
                    let buffered = (*ns.borrow() - ctx.current_time()).max(0.0) * 1000.0;
                    p_recv.buffered_ms.set(buffered as u32);
                    play_pcm(&ctx, &ns, &bytes);
                    if !*p_recv.is_bot_speaking.read() {
                        p_recv.is_bot_speaking.set(true);
                        p_recv.turn_count.with_mut(|c| *c += 1);
                    }
                }
                Ok(Message::Text(text)) => {
                    handle_ravi_event(&text, &mut p_recv);
                }
                Ok(Message::Close { .. }) | Err(_) => break,
                _ => {}
            }
        }

        p_recv.tasks_started.set(false);
        p_recv.activity.set(Activity::Idle);
        p_recv.energy.set(0.0);
        p_recv.is_bot_speaking.set(false);
        stop_mic();
    });
}

// ── Mic capture (wasm32) ────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
async fn start_mic(ws: UseWebsocket<String, String>) -> Result<(), JsValue> {
    let window     = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let media_devs = window.navigator().media_devices()?;

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&wasm_bindgen::JsValue::TRUE);

    let stream: web_sys::MediaStream =
        JsFuture::from(media_devs.get_user_media_with_constraints(&constraints)?)
            .await?
            .dyn_into()?;

    let opts = AudioContextOptions::new();
    opts.set_sample_rate(16_000.0);
    let mic_ctx = AudioContext::new_with_context_options(&opts)?;

    let source = mic_ctx.create_media_stream_source(&stream)?;
    // 512 samples @ 16 kHz = 32 ms = exactly one Silero VAD window.
    // Larger buffers (e.g. 4096) cause the VAD to receive 8 windows per chunk
    // but only analyse one, building a backlog that delays silence detection by
    // tens of seconds.
    let proc   = mic_ctx
        .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
            512, 1, 1,
        )?;
    source.connect_with_audio_node(&proc)?;
    proc.connect_with_audio_node(&mic_ctx.destination())?;

    let cb: Closure<dyn FnMut(web_sys::AudioProcessingEvent)> =
        Closure::new(move |e: web_sys::AudioProcessingEvent| {
            let Ok(in_buf) = e.input_buffer() else { return };
            let Ok(f32s)   = in_buf.get_channel_data(0) else { return };

            let bytes: Vec<u8> = f32s
                .iter()
                .flat_map(|&s| {
                    let v = (s * 32_768.0).clamp(-32_768.0, 32_767.0) as i16;
                    v.to_le_bytes()
                })
                .collect();

            let ws_send = ws;
            spawn_local(async move {
                ws_send.send_raw(Message::Binary(bytes.into())).await.ok();
            });
        });

    proc.set_onaudioprocess(Some(cb.as_ref().unchecked_ref()));

    MIC.with(|c| {
        *c.borrow_mut() = Some(MicResources {
            _ctx:    mic_ctx,
            _stream: stream,
            _proc:   proc,
            _cb:     cb,
        });
    });

    Ok(())
}

// ── Audio playback (wasm32) ─────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn play_pcm(ctx: &AudioContext, next_start: &Rc<RefCell<f64>>, bytes: &[u8]) {
    let n = bytes.len() / 2;
    if n == 0 { return; }

    let f32s: Vec<f32> = bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32_768.0)
        .collect();

    ctx.resume().ok();

    let Ok(buf) = ctx.create_buffer(1, n as u32, 24_000.0) else { return };
    buf.copy_to_channel(&f32s, 0).ok();

    let Ok(src) = ctx.create_buffer_source() else { return };
    src.set_buffer(Some(&buf));
    src.connect_with_audio_node(&ctx.destination()).ok();

    let now = ctx.current_time();
    let at  = now.max(*next_start.borrow());
    src.start_with_when(at).ok();
    *next_start.borrow_mut() = at + buf.duration();
}

#[cfg(target_arch = "wasm32")]
fn rms_energy(bytes: &[u8]) -> f32 {
    let n = bytes.len() / 2;
    if n == 0 { return 0.0; }
    let sum_sq: f64 = bytes
        .chunks_exact(2)
        .map(|c| {
            let s = i16::from_le_bytes([c[0], c[1]]) as f64 / 32_768.0;
            s * s
        })
        .sum();
    ((sum_sq / n as f64).sqrt() * 4.0).min(1.0) as f32
}

// ── RAVI event parsing (wasm32) ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
struct RaviEnvelope {
    label: String,
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

#[cfg(target_arch = "wasm32")]
fn handle_ravi_event(text: &str, pipeline: &mut Pipeline) {
    // Raw server-side interruption (no RAVI label)
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(text) {
        if raw.get("type").and_then(|v| v.as_str()) == Some("interruption")
            && raw.get("label").is_none()
        {
            pipeline.activity.set(Activity::Idle);
            pipeline.energy.set(0.0);
            pipeline.is_bot_speaking.set(false);
            pipeline.interrupt_count.with_mut(|c| *c += 1);
            log_event(pipeline, "interrupt", Some("server"));
            return;
        }
    }

    let envelope: RaviEnvelope = match serde_json::from_str(text) {
        Ok(e) => e,
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
            pipeline.activity.set(Activity::Listening);
            log_event(pipeline, "user-started-speaking", None);
        }
        "user-stopped-speaking" => {
            log_event(pipeline, "user-stopped-speaking", None);
        }
        "user-transcription" => {
            if let Some(data) = envelope.data {
                let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let is_final = data.get("final").and_then(|v| v.as_bool()).unwrap_or(false);
                pipeline.transcript.set(text.clone());
                if is_final {
                    let id = envelope.id.unwrap_or_else(|| uid());
                    let text_clone = text.clone();
                    pipeline.messages.with_mut(|v| v.push(ChatMessage {
                        id,
                        role: "user".to_string(),
                        text: text_clone,
                        final_msg: true,
                    }));
                    pipeline.transcript.set(String::new());
                }
                log_event(pipeline, "user-transcription", Some(&text));
            }
        }
        "bot-llm-started" => {
            pipeline.activity.set(Activity::Processing);
            log_event(pipeline, "bot-llm-started", None);
        }
        "bot-llm-stopped" => {
            log_event(pipeline, "bot-llm-stopped", None);
        }
        "bot-llm-text" => {
            if let Some(data) = envelope.data {
                let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                pipeline.bot_text.with_mut(|t| *t += &text);
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
                let id = envelope.id.unwrap_or_else(|| uid());
                pipeline.messages.with_mut(|v| {
                    if let Some(last) = v.last_mut() {
                        if last.role == "assistant" && !last.final_msg {
                            last.text = text.clone();
                            last.final_msg = true;
                            return;
                        }
                    }
                    v.push(ChatMessage {
                        id,
                        role: "assistant".to_string(),
                        text: text.clone(),
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
                let msg_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                pipeline.server_messages.with_mut(|v| v.push(ServerMessage {
                    msg_type: msg_type.clone(),
                    data: data_str,
                }));

                match msg_type.as_str() {
                    "function-call-start" => log_event(pipeline, "function-call-start", None),
                    "function-call-end" => log_event(pipeline, "function-call-end", None),
                    "function-call-in-progress" => {
                        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = data.get("function_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        pipeline.function_calls.with_mut(|v| {
                            v.push(FunctionCall {
                                id: id.clone(),
                                name,
                                state: "in-progress".to_string(),
                                payload: Some(data.to_string()),
                            });
                        });
                        log_event(pipeline, "function-call-in-progress", None);
                    }
                    "function-call-result" => {
                        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = data.get("function_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        pipeline.function_calls.with_mut(|v| {
                            if let Some(fc) = v.iter_mut().find(|f| f.id == id) {
                                fc.state = "completed".to_string();
                            } else {
                                v.push(FunctionCall {
                                    id,
                                    name,
                                    state: "completed".to_string(),
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
        "error" => {
            if let Some(data) = envelope.data {
                let err = data.get("error").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                pipeline.last_error.set(Some(err.clone()));
                log_event(pipeline, "error", Some(&err));
            }
        }
        "error-response" => {
            if let Some(data) = envelope.data {
                let err = data.get("error").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                pipeline.last_error.set(Some(err.clone()));
                log_event(pipeline, "error-response", Some(&err));
            }
        }
        _ => {
            log_event(pipeline, &format!("unhandled: {}", envelope.msg_type), None);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn log_event(pipeline: &mut Pipeline, event_type: &str, detail: Option<&str>) {
    let now = js_sys::Date::now() as u64;
    pipeline.events.with_mut(|v| {
        v.push(RaviEvent {
            id: uid(),
            ts: now,
            event_type: event_type.to_string(),
            detail: detail.map(|s| s.to_string()),
        });
    });
}

fn uid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:08x}", n)
}

// ── Cleanup (wasm32) ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn stop_mic() {
    MIC.with(|c| {
        if let Some(m) = c.borrow().as_ref() {
            m._proc.disconnect().ok();
            let tracks = m._stream.get_audio_tracks();
            for i in 0..tracks.length() {
                if let Ok(t) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                    t.stop();
                }
            }
        }
        *c.borrow_mut() = None;
    });

    PLAY.with(|p| {
        if let Some((ctx, _, _)) = p.borrow().as_ref() {
            ctx.close().ok();
        }
        *p.borrow_mut() = None;
    });
}

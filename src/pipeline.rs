//! rustvani::dioxus::pipeline
//!
//! Per-connection Rustvani voice pipeline served through a Dioxus
//! fullstack WebSocket endpoint.
//!
//! ┌─────────────┐  binary/text   ┌──────────────┐  ChannelMessage   ┌──────────────────┐
//! │ Dioxus WASM │ ◄────────────► │  bridge loop │ ◄───────────────► │ ChannelTransport │
//! │   client    │   WebSocket    │  (this file) │   mpsc channels   │  → pipeline …    │
//! └─────────────┘                └──────────────┘                   └──────────────────┘

use dioxus::prelude::*;
use dioxus::fullstack::{Message, WebSocketOptions, Websocket};

/// Handle to the main multi-threaded tokio runtime, set once in `main` before
/// Dioxus launches. Pipeline tasks are spawned here so they run concurrently
/// on the thread-pool rather than serialised on the LocalPool executor that
/// Dioxus uses for `on_upgrade` callbacks.
#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
pub static MAIN_RT: std::sync::OnceLock<tokio::runtime::Handle> = std::sync::OnceLock::new();

// Server-only imports — none of these compile to wasm32
#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
use {
    std::sync::Arc,
    bytes::Bytes,
    tokio::sync::mpsc,
    dioxus::fullstack::TypedWebsocket,
    rustvani::{
        SileroVadNative, VadParams,
        context::shared_context,
        observer::BaseObserver,
        pipeline::{PipelineParams, PipelineTask},
        processors::{
            llm_assistant_aggregator::LLMAssistantAggregator,
            llm_user_aggregator::LLMUserAggregator,
        },
        ravi::{RaviObserver, RaviObserverParams, RaviParams, RaviProcessor},
        system_clock,
        transport::{ChannelMessage, ChannelTransport, TransportParams},
    },
    rustvani::services::{
        DeepgramTtsConfig, DeepgramTtsHandler,
        OpenAILLMConfig, OpenAILLMHandler,
        SarvamSttConfig, SarvamSttHandler,
    },
};

// ---------------------------------------------------------------------------
// Configuration (server only)
// ---------------------------------------------------------------------------

/// Everything the pipeline needs to spin up one connection.
/// Clone-cheap — holds only strings.
#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
#[derive(Clone)]
pub struct PipelineConfig {
    pub sarvam_api_key:   String,
    pub openai_api_key:   String,
    pub deepgram_api_key: String,
    pub system_prompt:    String,
    pub stt_model:        String,
    pub stt_language:     String,
    pub llm_model:        String,
    pub sample_rate_in:   u32,
    pub sample_rate_out:  u32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
impl PipelineConfig {
    /// Pull everything from environment variables.
    /// Panics on missing keys — call at startup, not per-request.
    pub fn from_env() -> Self {
        Self {
            sarvam_api_key:   std::env::var("SARVAM_API_KEY").expect("SARVAM_API_KEY"),
            openai_api_key:   std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY"),
            deepgram_api_key: std::env::var("DEEPGRAM_API_KEY").expect("DEEPGRAM_API_KEY"),
            system_prompt:    std::env::var("SYSTEM_PROMPT").unwrap_or_else(|_|
                "You are a helpful voice assistant. Keep answers concise.".into()
            ),
            stt_model:        "saaras:v3".into(),
            stt_language:     "en-IN".into(),
            llm_model:        "gpt-4o-mini".into(),
            sample_rate_in:   16_000,
            sample_rate_out:  24_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Pipeline spawner (server only)
// ---------------------------------------------------------------------------

/// Handles returned by `spawn_pipeline`.
#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
pub struct PipelineHandle {
    pub incoming_tx: mpsc::Sender<ChannelMessage>,
    pub outgoing_rx: mpsc::Receiver<ChannelMessage>,
}

/// Spawn a full voice pipeline on the tokio runtime.
#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
pub fn spawn_pipeline(config: &PipelineConfig) -> Result<PipelineHandle, String> {
    let (incoming_tx, incoming_rx) = mpsc::channel::<ChannelMessage>(128);
    let (outgoing_tx, outgoing_rx) = mpsc::channel::<ChannelMessage>(128);

    let vad = Arc::new(
        SileroVadNative::new(config.sample_rate_in)
            .map_err(|e| format!("VAD init: {e}"))?
    );

    let transport = ChannelTransport::new(
        "dioxus-pipeline",
        TransportParams {
            audio_in_enabled:         true,
            audio_in_sample_rate:     Some(config.sample_rate_in),
            audio_in_channels:        1,
            audio_in_passthrough:     true,
            audio_in_stream_on_start: true,
            audio_out_enabled:        true,
            audio_out_sample_rate:    Some(config.sample_rate_out),
            audio_out_channels:       1,
            audio_out_10ms_chunks:    4,
            vad_analyzer:             Some(vad),
            vad_params:               VadParams {
                confidence: 0.4,
                min_volume: 0.1,
                ..VadParams::default()
            },
            ..TransportParams::default()
        },
        incoming_rx,
    );

    let context = shared_context(Some(config.system_prompt.clone()));

    let ravi = RaviProcessor::new(RaviParams {
        context: Some(context.clone()),
        ..RaviParams::default()
    });

    let stt = SarvamSttHandler::new(SarvamSttConfig {
        api_key:  config.sarvam_api_key.clone(),
        model:    config.stt_model.clone(),
        language: Some(config.stt_language.clone()),
        mode:     Some("transcribe".into()),
        ..SarvamSttConfig::default()
    })
    .into_processor();

    let user_agg      = LLMUserAggregator::new(context.clone());
    let assistant_agg = LLMAssistantAggregator::new(context.clone());

    let llm = OpenAILLMHandler::new(OpenAILLMConfig {
        api_key: config.openai_api_key.clone(),
        model:   config.llm_model.clone(),
        ..OpenAILLMConfig::default()
    })
    .into_processor();

    let tts = DeepgramTtsHandler::new(DeepgramTtsConfig {
        api_key: config.deepgram_api_key.clone(),
        ..DeepgramTtsConfig::default()
    })
    .map_err(|e| format!("TTS init: {e}"))?
    .into_processor();

    let observer = RaviProcessor::create_observer(&ravi, RaviObserverParams::default());

    let task = PipelineTask::new(
        vec![
            transport.input(),
            ravi,
            stt,
            user_agg,
            llm,
            assistant_agg,
            tts,
            transport.output(),
        ],
        PipelineParams {
            allow_interruptions: true,
            ..PipelineParams::default()
        },
    );

    let push_tx = task.push_sender();

    let rt = MAIN_RT.get().expect("MAIN_RT not initialised — set it in main() before dioxus::launch");

    rt.spawn(async move {
        if let Err(e) = task.run(system_clock(), Some(Arc::new(observer) as Arc<dyn BaseObserver>)).await {
            tracing::error!("[pipeline] error: {e}");
        }
        tracing::info!("[pipeline] stopped");
    });

    rt.spawn(async move {
        transport.run(push_tx, outgoing_tx).await;
        tracing::info!("[transport] stopped");
    });

    Ok(PipelineHandle { incoming_tx, outgoing_rx })
}

// ---------------------------------------------------------------------------
// Dioxus WebSocket endpoint
// ---------------------------------------------------------------------------

/// Dioxus fullstack WS endpoint.
///
/// Client connects with `use_websocket`, sends binary PCM + RAVI JSON text,
/// receives binary TTS audio + RAVI JSON events back.
#[get("/api/pipeline")]
pub async fn pipeline_ws(options: WebSocketOptions) -> Result<Websocket> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
    {
        run_pipeline_ws(options).await
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "server")))]
    {
        Err(ServerFnError::new("pipeline server not available"))
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
async fn run_pipeline_ws(options: WebSocketOptions) -> Result<Websocket> {
    tracing::info!("pipeline_ws: upgrading");

    Ok(options.on_upgrade(
        |mut ws: TypedWebsocket<String, String>| async move {
            let config = PipelineConfig::from_env();

            let mut handle = match spawn_pipeline(&config) {
                Ok(h)  => h,
                Err(e) => {
                    tracing::error!("spawn_pipeline failed: {e}");
                    let _ = ws.send_raw(Message::Close {
                        code: dioxus::fullstack::CloseCode::Error,
                        reason: e,
                    }).await;
                    return;
                }
            };

            tracing::info!("pipeline_ws: bridge running");

            let mut first_audio_at: Option<std::time::Instant> = None;
            let mut first_response_logged = false;
            let mut audio_chunks_in: u32 = 0;

            loop {
                tokio::select! {
                    // ── client → pipeline ──
                    msg = ws.recv_raw() => {
                        match msg {
                            Ok(Message::Binary(bytes)) => {
                                if first_audio_at.is_none() {
                                    first_audio_at = Some(std::time::Instant::now());
                                    tracing::info!("bridge: first audio from client ({} bytes)", bytes.len());
                                }
                                audio_chunks_in += 1;
                                if audio_chunks_in % 100 == 0 {
                                    tracing::info!("bridge: {} audio chunks received so far", audio_chunks_in);
                                }
                                let _ = handle.incoming_tx
                                    .send(ChannelMessage::Audio(bytes.to_vec()))
                                    .await;
                            }
                            Ok(Message::Text(text)) => {
                                let msg = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                    match v.get("type").and_then(|t| t.as_str()) {
                                        Some("client_vad_start") => {
                                            tracing::info!("bridge: client_vad_start");
                                            ChannelMessage::ClientVadStart(0.0)
                                        }
                                        Some("client_vad_stop") => {
                                            tracing::info!("bridge: client_vad_stop");
                                            ChannelMessage::ClientVadStop(0.0)
                                        }
                                        _ => ChannelMessage::Text(text),
                                    }
                                } else {
                                    ChannelMessage::Text(text)
                                };
                                let _ = handle.incoming_tx.send(msg).await;
                            }
                            Ok(Message::Ping(_) | Message::Pong(_)) => {}
                            Ok(Message::Close { .. }) | Err(_) => break,
                        }
                    }

                    // ── pipeline → client ──
                    out = handle.outgoing_rx.recv() => {
                        match out {
                            Some(ChannelMessage::Audio(bytes)) => {
                                if !first_response_logged {
                                    first_response_logged = true;
                                    let elapsed = first_audio_at
                                        .map(|t| format!("{:.1}s", t.elapsed().as_secs_f32()))
                                        .unwrap_or_else(|| "?".into());
                                    tracing::info!("bridge: first TTS audio after {} ({} bytes)", elapsed, bytes.len());
                                }
                                let _ = ws.send_raw(
                                    Message::Binary(Bytes::from(bytes))
                                ).await;
                            }
                            Some(ChannelMessage::Text(text)) => {
                                if !first_response_logged {
                                    first_response_logged = true;
                                    let elapsed = first_audio_at
                                        .map(|t| format!("{:.1}s", t.elapsed().as_secs_f32()))
                                        .unwrap_or_else(|| "?".into());
                                    tracing::info!("bridge: first text from pipeline after {}: {:.120}", elapsed, text);
                                } else {
                                    tracing::info!("bridge: pipeline text: {:.120}", text);
                                }
                                let _ = ws.send_raw(Message::Text(text)).await;
                            }
                            Some(ChannelMessage::Interruption) => {
                                let _ = ws.send_raw(
                                    Message::Text(r#"{"type":"interruption"}"#.into())
                                ).await;
                            }
                            Some(ChannelMessage::ClientVadStart(_))
                            | Some(ChannelMessage::ClientVadStop(_)) => {}
                            None => break,
                        }
                    }
                }
            }

            tracing::info!("pipeline_ws: connection closed");
        },
    ))
}

// ---------------------------------------------------------------------------
// Helpers (server only)
// ---------------------------------------------------------------------------



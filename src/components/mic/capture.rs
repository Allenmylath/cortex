use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{AudioContext, AudioContextOptions, GainNode, MediaStreamConstraints, ScriptProcessorNode};
use dioxus::fullstack::Message;

use super::types::{Activity, Pipeline};
use super::vad::{ClientVad, VadState};
use super::playback::speaker_clear;
use super::ravi::log_event;

pub struct MicResources {
    pub _ctx:    AudioContext,
    pub _stream: web_sys::MediaStream,
    pub _proc:   ScriptProcessorNode,
    pub _gain:   GainNode,
    pub _cb:     Closure<dyn FnMut(web_sys::AudioProcessingEvent)>,
}

thread_local! {
    pub static MIC: RefCell<Option<MicResources>> = RefCell::new(None);
}

/// Request mic permission, set up VAD + PCM capture, stream audio to the server.
pub async fn start_mic(mut pipeline: Pipeline) -> Result<(), JsValue> {
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
    // Larger buffers delay silence detection by tens of seconds.
    let proc = mic_ctx
        .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
            512, 1, 1,
        )?;
    // Route proc → silent gain → destination so onaudioprocess fires, but the
    // mic audio is muted before reaching the speakers (prevents bot-voice echo).
    let silent: GainNode = mic_ctx.create_gain()?;
    silent.gain().set_value(0.0);
    source.connect_with_audio_node(&proc)?;
    proc.connect_with_audio_node(&silent)?;
    silent.connect_with_audio_node(&mic_ctx.destination())?;

    let mut vad = ClientVad::new().map_err(|e| JsValue::from_str(&e))?;

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

            if let Some((state, confidence)) = vad.process(&bytes) {
                pipeline.vad_prob.set(confidence);
                match state {
                    VadState::Speaking => {
                        pipeline.user_speaking.set(true);
                        pipeline.activity.set(Activity::Listening);
                        pipeline.bot_text.set(String::new());
                        pipeline.is_bot_speaking.set(false);
                        log_event(&mut pipeline, "user-started-speaking", Some("client-vad"));
                        speaker_clear();
                        let ws = pipeline.ws;
                        spawn_local(async move {
                            let _ = ws.send_raw(Message::Text(
                                r#"{"type":"client_interruption"}"#.into(),
                            )).await.ok();
                        });
                    }
                    VadState::Quiet => {
                        pipeline.user_speaking.set(false);
                        log_event(&mut pipeline, "user-stopped-speaking", Some("client-vad"));
                    }
                    _ => {}
                }
            }

            let ws = pipeline.ws;
            spawn_local(async move {
                ws.send_raw(Message::Binary(bytes.into())).await.ok();
            });
        });

    proc.set_onaudioprocess(Some(cb.as_ref().unchecked_ref()));

    MIC.with(|c| {
        *c.borrow_mut() = Some(MicResources {
            _ctx:    mic_ctx,
            _stream: stream,
            _proc:   proc,
            _gain:   silent,
            _cb:     cb,
        });
    });

    Ok(())
}

/// Stop mic capture and release all audio resources.
pub fn stop_mic() {
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

    super::playback::speaker_close();
}

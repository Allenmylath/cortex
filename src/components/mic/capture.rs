/// capture.rs
///
/// Changes from the previous version
/// ───────────────────────────────────────────────────────────────────────────
/// 1. **Shared AudioContext** — we call `playback::shared_context()` instead
///    of creating a new one.  This means the browser's built-in AEC can see
///    the bot's playback signal as a reference, so it can actually cancel it.
///
/// 2. **AudioWorkletNode instead of ScriptProcessorNode** — the deprecated
///    ScriptProcessorNode ran on the main thread and caused audio glitches
///    under any UI load.  The worklet runs on the browser's dedicated audio
///    thread, giving glitch-free, low-latency capture.
///
/// 3. **RNNoise inside the worklet** — a lightweight noise suppression pass
///    (via the rnnoise WASM module loaded from a CDN) runs in the worklet
///    before the audio reaches the network.  This catches keyboard noise,
///    background TV, and fan noise that the browser's noiseSuppression misses.
///
/// 4. **RMS normaliser inside the worklet** — a rolling RMS gain stage
///    normalises the mic level before sending to the server, giving more
///    consistent VAD confidence and better STT accuracy.
///
/// The rest of the logic (VAD, WebSocket send, MicResources drop guard) is
/// identical to the previous version.

use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    AudioContext, AudioWorkletNode, AudioWorkletNodeOptions,
    GainNode, MediaStreamConstraints,
};
use dioxus::fullstack::Message;

use super::types::{Activity, Pipeline};
use super::vad::{ClientVad, VadState};
use super::playback::{speaker_clear, shared_context};
use super::ravi::log_event;

// ── AudioWorklet processor source ────────────────────────────────────────────
//
// This JS string is registered as a Blob URL and added to the AudioWorklet.
// It runs entirely on the audio thread (AudioWorkletGlobalScope), so it has
// no access to the DOM or Dioxus signals — it only posts processed PCM back
// to the main thread via the MessagePort.
//
// Pipeline inside the worklet (per 128-sample render quantum):
//   mic f32 → RNNoise (if loaded) → RMS normaliser → i16-LE bytes → port.postMessage
//
// RNNoise is loaded lazily from a CDN the first time the worklet is created.
// If the fetch fails (offline / CSP) the worklet falls back to raw mic audio —
// the rest of the pipeline is unaffected.

const WORKLET_JS: &str = r#"
// ── RNNoise lazy loader ──────────────────────────────────────────────────────
let rnnoise = null;
let rnnoiseReady = false;

// Try to load RNNoise WASM from CDN. Silently skip if unavailable.
(async () => {
  try {
    // rnnoise-wasm provides a factory function; we use the ESM CDN build.
    const mod = await import('https://cdn.jsdelivr.net/npm/rnnoise-wasm@0.2.2/dist/rnnoise.js');
    const factory = mod.default || mod;
    rnnoise = await factory();
    rnnoiseReady = true;
  } catch (_) {
    // Offline or CSP block — continue without RNNoise
  }
})();

// ── RMS normaliser state ─────────────────────────────────────────────────────
const RMS_WINDOW  = 1600;   // ~100 ms of history at 16 kHz
const RMS_TARGET  = 0.08;   // target RMS (linear, 0–1)
const GAIN_MIN    = 0.1;
const GAIN_MAX    = 8.0;
const GAIN_SMOOTH = 0.95;   // smoothing coefficient (per-quantum)

let rmsHistory    = new Float32Array(RMS_WINDOW);
let rmsIdx        = 0;
let rmsSumSq      = 0.0;
let currentGain   = 1.0;

function updateGain(samples) {
  for (let i = 0; i < samples.length; i++) {
    const old = rmsHistory[rmsIdx];
    rmsSumSq -= old * old;
    rmsHistory[rmsIdx] = samples[i];
    rmsSumSq += samples[i] * samples[i];
    rmsIdx = (rmsIdx + 1) % RMS_WINDOW;
  }
  const rms = Math.sqrt(Math.max(rmsSumSq, 0) / RMS_WINDOW);
  if (rms > 1e-6) {
    const targetGain = Math.min(Math.max(RMS_TARGET / rms, GAIN_MIN), GAIN_MAX);
    currentGain = GAIN_SMOOTH * currentGain + (1.0 - GAIN_SMOOTH) * targetGain;
  }
}

// ── RNNoise frame helper ─────────────────────────────────────────────────────
// RNNoise expects exactly 480 samples per frame.
const RNNOISE_FRAME = 480;
let rnBuf   = new Float32Array(0);   // carry-over between quanta
let rnState = null;

function processRnnoise(samples) {
  // Accumulate
  const combined = new Float32Array(rnBuf.length + samples.length);
  combined.set(rnBuf, 0);
  combined.set(samples, rnBuf.length);

  const out     = new Float32Array(combined.length);
  let   outIdx  = 0;

  // RNNoise works on 16-bit scale internally (multiply by 32768, then back)
  let i = 0;
  while (i + RNNOISE_FRAME <= combined.length) {
    const frame16 = new Float32Array(RNNOISE_FRAME);
    for (let k = 0; k < RNNOISE_FRAME; k++) {
      frame16[k] = combined[i + k] * 32768.0;
    }
    if (!rnState) rnState = rnnoise.newState();
    rnnoise.processFrame(rnState, frame16);
    for (let k = 0; k < RNNOISE_FRAME; k++) {
      out[outIdx++] = frame16[k] / 32768.0;
    }
    i += RNNOISE_FRAME;
  }

  // Carry over remainder
  rnBuf = combined.slice(i);
  return out.slice(0, outIdx);
}

// ── AudioWorkletProcessor ────────────────────────────────────────────────────
class MicProcessor extends AudioWorkletProcessor {
  process(inputs) {
    const input = inputs[0];
    if (!input || !input[0]) return true;

    let samples = input[0];  // Float32Array, 128 samples

    // 1. RNNoise (if available)
    if (rnnoiseReady && rnnoise) {
      const denoised = processRnnoise(samples);
      if (denoised.length > 0) samples = denoised;
    }

    // 2. RMS normaliser
    updateGain(samples);
    const normalised = new Float32Array(samples.length);
    for (let i = 0; i < samples.length; i++) {
      normalised[i] = Math.max(-1.0, Math.min(1.0, samples[i] * currentGain));
    }

    // 3. f32 → i16-LE bytes
    const bytes = new Uint8Array(normalised.length * 2);
    const view  = new DataView(bytes.buffer);
    for (let i = 0; i < normalised.length; i++) {
      const v = Math.max(-32768, Math.min(32767, Math.round(normalised[i] * 32768)));
      view.setInt16(i * 2, v, true);   // little-endian
    }

    this.port.postMessage(bytes.buffer, [bytes.buffer]);
    return true;
  }
}

registerProcessor('mic-processor', MicProcessor);
"#;

// ── MicResources drop guard ───────────────────────────────────────────────────

pub struct MicResources {
    pub _stream:  web_sys::MediaStream,
    pub _node:    AudioWorkletNode,
    pub _gain:    GainNode,
    pub _on_msg:  Closure<dyn FnMut(web_sys::MessageEvent)>,
}

thread_local! {
    pub static MIC: RefCell<Option<MicResources>> = RefCell::new(None);
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Request mic permission, set up VAD + PCM capture, stream audio to the server.
///
/// Key difference from the previous version: we borrow the AudioContext that
/// the speaker already created (`shared_context()`).  Using the same context
/// for both capture and playback is what makes the browser's AEC effective —
/// it can see the playback signal as a reference and subtract it from the mic.
pub async fn start_mic(mut pipeline: Pipeline) -> Result<(), JsValue> {
    let window     = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let media_devs = window.navigator().media_devices()?;

    // ── getUserMedia with full hardware-level processing ─────────────────────
    // echoCancellation + noiseSuppression + autoGainControl are still enabled
    // as a first-pass hardware layer.  Our worklet adds a second software layer
    // on top.
    let constraints = MediaStreamConstraints::new();
    let audio = js_sys::Object::new();
    js_sys::Reflect::set(&audio, &"echoCancellation".into(),  &true.into()).unwrap();
    js_sys::Reflect::set(&audio, &"noiseSuppression".into(),  &true.into()).unwrap();
    js_sys::Reflect::set(&audio, &"autoGainControl".into(),   &true.into()).unwrap();
    // channelCount: 1 avoids stereo→mono down-mix artefacts in some browsers
    js_sys::Reflect::set(&audio, &"channelCount".into(),      &1_u32.into()).unwrap();
    constraints.set_audio(&audio);

    let stream: web_sys::MediaStream =
        JsFuture::from(media_devs.get_user_media_with_constraints(&constraints)?)
            .await?
            .dyn_into()?;

    // ── Shared AudioContext (from playback.rs) ────────────────────────────────
    let ctx = shared_context();

    // ── Register the AudioWorklet module ─────────────────────────────────────
    // We turn the worklet JS source into a Blob URL so we can add it without
    // a separate file on the server.
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&JsValue::from_str(WORKLET_JS));
    let blob = web_sys::Blob::new_with_str_sequence_and_options(
        &blob_parts,
        web_sys::BlobPropertyBag::new().type_("application/javascript"),
    )?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    JsFuture::from(ctx.audio_worklet()?.add_module(&url)?).await?;
    web_sys::Url::revoke_object_url(&url)?;

    // ── Build the processing graph ────────────────────────────────────────────
    //
    //   MediaStream source
    //        │
    //   AudioWorkletNode  ← runs RNNoise + RMS normaliser; posts i16-LE bytes
    //        │
    //   GainNode (gain=0) ← mutes the output so the mic is never heard in the
    //        │              speakers (prevents feedback/echo), while still
    //   ctx.destination   keeping the audio graph alive so onaudioprocess fires.

    let source = ctx.create_media_stream_source(&stream)?;

    let worklet_opts = AudioWorkletNodeOptions::new();
    worklet_opts.set_number_of_inputs(1);
    worklet_opts.set_number_of_outputs(1);
    let worklet = AudioWorkletNode::new_with_options(&ctx, "mic-processor", &worklet_opts)?;

    let silent: GainNode = ctx.create_gain()?;
    silent.gain().set_value(0.0);

    source.connect_with_audio_node(&worklet)?;
    worklet.connect_with_audio_node(&silent)?;
    silent.connect_with_audio_node(&ctx.destination())?;

    // ── Message handler: worklet → VAD → WebSocket ───────────────────────────
    let mut vad = ClientVad::new().map_err(|e| JsValue::from_str(&e))?;

    let on_msg: Closure<dyn FnMut(web_sys::MessageEvent)> =
        Closure::new(move |e: web_sys::MessageEvent| {
            // The worklet transfers the ArrayBuffer ownership to us
            let array_buf: js_sys::ArrayBuffer = match e.data().dyn_into() {
                Ok(ab) => ab,
                Err(_) => return,
            };
            let bytes_js = js_sys::Uint8Array::new(&array_buf);
            let bytes: Vec<u8> = bytes_js.to_vec();

            if let Some((state, confidence)) = vad.process(&bytes) {
                pipeline.vad_prob.set(confidence);
                if confidence > 0.5 {
                    speaker_clear();
                }
                match state {
                    VadState::Speaking => {
                        if !*pipeline.user_speaking.read() {
                            pipeline.user_speaking.set(true);
                            pipeline.is_bot_speaking.set(false);
                            pipeline.activity.set(Activity::Listening);
                            pipeline.bot_text.set(String::new());
                            let ws = pipeline.ws;
                            spawn_local(async move {
                                let _ = ws.send_raw(Message::Text(
                                    r#"{"type":"client_vad_start"}"#.into(),
                                )).await.ok();
                            });
                            log_event(&mut pipeline, "user-started-speaking", Some("client-vad"));
                        }
                    }
                    VadState::Quiet => {
                        if *pipeline.user_speaking.read() {
                            pipeline.user_speaking.set(false);
                            let ws = pipeline.ws;
                            spawn_local(async move {
                                let _ = ws.send_raw(Message::Text(
                                    r#"{"type":"client_vad_stop"}"#.into(),
                                )).await.ok();
                            });
                            log_event(&mut pipeline, "user-stopped-speaking", Some("client-vad"));
                        }
                    }
                    _ => {}
                }
            }

            let ws = pipeline.ws;
            spawn_local(async move {
                ws.send_raw(Message::Binary(bytes.into())).await.ok();
            });
        });

    worklet
        .port()
        .map_err(|_| JsValue::from_str("worklet port unavailable"))?
        .set_onmessage(Some(on_msg.as_ref().unchecked_ref()));

    MIC.with(|c| {
        *c.borrow_mut() = Some(MicResources {
            _stream:  stream,
            _node:    worklet,
            _gain:    silent,
            _on_msg:  on_msg,
        });
    });

    Ok(())
}

/// Stop mic capture and release all audio resources.
pub fn stop_mic() {
    MIC.with(|c| {
        if let Some(m) = c.borrow().as_ref() {
            m._node.disconnect().ok();
            if let Ok(port) = m._node.port() {
                port.set_onmessage(None);
            }
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
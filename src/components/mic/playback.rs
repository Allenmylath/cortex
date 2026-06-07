/// playback.rs
///
/// Changes from the previous version
/// ───────────────────────────────────────────────────────────────────────────
/// 1. The `AudioContext` is now created at **16 000 Hz** (same as the capture
///    path) so both mic and speaker share a single context.  The incoming TTS
///    audio (24 kHz i16-LE from the server) is resampled to 16 kHz in
///    `push()` using linear interpolation before scheduling.
///
/// 2. `shared_context()` is a new public function that hands the same
///    `AudioContext` to `capture.rs`.  This makes the browser's built-in AEC
///    aware of the playback signal, because AEC only works when mic and
///    speaker live in the same Web Audio graph.
///
/// 3. `init_speaker` now returns the shared context via `Ok(AudioContext)` so
///    `client.rs` / `capture.rs` can receive it without an extra global.
///
/// Everything else (jitter buffer, clear, close, rms_energy) is unchanged.

use std::cell::RefCell;
use dioxus::prelude::*;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextOptions, GainNode};

const IGNORE_AFTER_INTERRUPT_MS: f64 = 150.0;
const JITTER_BUFFER_MS:          f64 = 80.0;
const JITTER_BUFFER_MAX_WAIT_MS: f64 = 150.0;

/// Sample rate shared by both the mic capture and the speaker.
pub const SHARED_SAMPLE_RATE: f32 = 16_000.0;
/// TTS audio from the server arrives at this rate.
const SERVER_TTS_RATE: f32 = 24_000.0;

// ── Speaker ──────────────────────────────────────────────────────────────────

pub struct Speaker {
    ctx:           AudioContext,
    gain:          GainNode,
    chain_end:     f64,
    active_srcs:   Vec<AudioBufferSourceNode>,
    cleared_at:    f64,
    priming:       bool,
    pending:       Vec<AudioBuffer>,
    pending_ms:    f64,
    prime_started: Option<f64>,
    buffered_ms:   Signal<u32>,
}

impl Speaker {
    fn new(buffered_ms: Signal<u32>) -> Result<Self, wasm_bindgen::JsValue> {
        // ── Shared 16 kHz context ────────────────────────────────────────────
        // Using the same AudioContext for both capture and playback is the
        // prerequisite for the browser's echo-cancellation to see the bot
        // audio as a reference signal.
        let opts = AudioContextOptions::new();
        opts.set_sample_rate(SHARED_SAMPLE_RATE);
        let ctx = AudioContext::new_with_context_options(&opts)?;

        let gain = ctx.create_gain()?;
        gain.gain().set_value(1.0);
        gain.connect_with_audio_node(&ctx.destination())?;

        Ok(Self {
            ctx,
            gain,
            chain_end:     0.0,
            active_srcs:   Vec::new(),
            cleared_at:    f64::NEG_INFINITY,
            priming:       true,
            pending:       Vec::new(),
            pending_ms:    0.0,
            prime_started: None,
            buffered_ms,
        })
    }

    /// Append a server TTS chunk (24 kHz mono i16-LE) to the playback queue.
    /// The chunk is resampled to SHARED_SAMPLE_RATE before scheduling.
    pub fn push(&mut self, bytes: &[u8]) {
        let n = bytes.len() / 2;
        if n == 0 { return; }

        // Post-interrupt discard window.
        if js_sys::Date::now() - self.cleared_at < IGNORE_AFTER_INTERRUPT_MS {
            return;
        }

        // Decode i16-LE → f32
        let src_f32: Vec<f32> = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32_768.0)
            .collect();

        // Linear-interpolation resample 24 kHz → 16 kHz
        let ratio      = SERVER_TTS_RATE / SHARED_SAMPLE_RATE;   // 1.5
        let out_len    = ((n as f32) / ratio).ceil() as usize;
        let mut resampled = Vec::with_capacity(out_len);
        for i in 0..out_len {
            let pos = i as f32 * ratio;
            let lo  = pos.floor() as usize;
            let hi  = (lo + 1).min(n - 1);
            let t   = pos - lo as f32;
            resampled.push(src_f32[lo] * (1.0 - t) + src_f32[hi] * t);
        }

        let Ok(buf) = self.ctx.create_buffer(
            1, resampled.len() as u32, SHARED_SAMPLE_RATE,
        ) else { return };
        buf.copy_to_channel(&resampled, 0).ok();

        let now = self.ctx.current_time();

        // Re-enter priming when previous turn's queue has drained.
        if !self.priming && now >= self.chain_end {
            self.active_srcs.clear();
            self.priming       = true;
            self.pending       = Vec::new();
            self.pending_ms    = 0.0;
            self.prime_started = Some(now);
        }

        if self.priming {
            if self.prime_started.is_none() { self.prime_started = Some(now); }
            self.pending_ms += buf.duration() * 1000.0;
            self.pending.push(buf);
            let elapsed = (now - self.prime_started.unwrap()) * 1000.0;
            if self.pending_ms >= JITTER_BUFFER_MS || elapsed >= JITTER_BUFFER_MAX_WAIT_MS {
                self.flush_pending();
            }
            return;
        }

        self.schedule(buf);
    }

    fn flush_pending(&mut self) {
        self.priming       = false;
        self.prime_started = None;
        self.pending_ms    = 0.0;
        self.ctx.resume().ok();
        self.gain.gain().set_value(1.0);
        for buf in std::mem::take(&mut self.pending) {
            self.schedule(buf);
        }
    }

    fn schedule(&mut self, buf: AudioBuffer) {
        let Ok(src) = self.ctx.create_buffer_source() else { return };
        src.set_buffer(Some(&buf));
        src.connect_with_audio_node(&self.gain).ok();

        let now = self.ctx.current_time();
        let at  = now.max(self.chain_end);
        src.start_with_when(at).ok();
        self.chain_end = at + buf.duration();
        self.active_srcs.push(src);

        self.buffered_ms.set(((self.chain_end - now).max(0.0) * 1000.0) as u32);
    }

    pub fn clear(&mut self) {
        for src in self.active_srcs.drain(..) { src.stop().ok(); }
        self.gain.gain().set_value(0.0);
        self.chain_end     = self.ctx.current_time();
        self.cleared_at    = js_sys::Date::now();
        self.priming       = true;
        self.pending       = Vec::new();
        self.pending_ms    = 0.0;
        self.prime_started = None;
        self.buffered_ms.set(0);
    }

    fn close(self) { self.ctx.close().ok(); }

    /// Hand out a clone of the underlying AudioContext so that capture.rs
    /// can attach the mic source to the same graph.
    pub fn audio_context(&self) -> AudioContext {
        self.ctx.clone()
    }
}

// ── Module-level API (thread-local singleton) ─────────────────────────────────

thread_local! {
    static SPEAKER: RefCell<Option<Speaker>> = RefCell::new(None);
}

/// Initialise the speaker.  Must be called before `shared_context()`.
pub fn init_speaker(buffered_ms: Signal<u32>) -> Result<(), wasm_bindgen::JsValue> {
    SPEAKER.with(|s| -> Result<(), wasm_bindgen::JsValue> {
        *s.borrow_mut() = Some(Speaker::new(buffered_ms)?);
        Ok(())
    })
}

/// Return the shared `AudioContext` so capture.rs can attach to the same graph.
/// Panics if called before `init_speaker`.
pub fn shared_context() -> AudioContext {
    SPEAKER.with(|s| {
        s.borrow()
         .as_ref()
         .expect("init_speaker must be called before shared_context")
         .audio_context()
    })
}

pub fn speaker_push(bytes: &[u8]) {
    SPEAKER.with(|s| {
        if let Some(spk) = s.borrow_mut().as_mut() { spk.push(bytes); }
    });
}

pub fn speaker_clear() {
    SPEAKER.with(|s| {
        if let Some(spk) = s.borrow_mut().as_mut() { spk.clear(); }
    });
}

pub fn speaker_close() {
    SPEAKER.with(|s| {
        if let Some(spk) = s.borrow_mut().take() { spk.close(); }
    });
}

// ── Utility ───────────────────────────────────────────────────────────────────

/// RMS energy of a 16-bit LE PCM chunk, normalised to 0.0–1.0.
pub fn rms_energy(bytes: &[u8]) -> f32 {
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
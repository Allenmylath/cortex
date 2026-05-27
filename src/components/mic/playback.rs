use std::cell::RefCell;
use dioxus::prelude::*;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode};

const IGNORE_AFTER_INTERRUPT_MS: f64 = 150.0;
const JITTER_BUFFER_MS:          f64 = 80.0;
const JITTER_BUFFER_MAX_WAIT_MS: f64 = 150.0;

// ── Speaker ──────────────────────────────────────────────────────────────────

pub struct Speaker {
    ctx:           AudioContext,
    gain:          GainNode,
    chain_end:     f64,
    /// All source nodes that have been started but not yet stopped.
    /// Pruned when the queue drains; stopped immediately on `clear`.
    active_srcs:   Vec<AudioBufferSourceNode>,
    /// wall-clock ms of last `clear()` call; guards the discard window.
    cleared_at:    f64,
    /// True while accumulating the jitter buffer for a new turn.
    priming:       bool,
    pending:       Vec<AudioBuffer>,
    pending_ms:    f64,
    prime_started: Option<f64>,
    buffered_ms:   Signal<u32>,
}

impl Speaker {
    fn new(buffered_ms: Signal<u32>) -> Result<Self, wasm_bindgen::JsValue> {
        let ctx  = AudioContext::new()?;
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

    /// Append a 24 kHz mono i16-LE chunk to the playback queue.
    pub fn push(&mut self, bytes: &[u8]) {
        let n = bytes.len() / 2;
        if n == 0 { return; }

        // Post-interrupt discard window — drop audio that arrived before the
        // server had time to react to our interruption signal.
        if js_sys::Date::now() - self.cleared_at < IGNORE_AFTER_INTERRUPT_MS {
            return;
        }

        let f32s: Vec<f32> = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32_768.0)
            .collect();

        let Ok(buf) = self.ctx.create_buffer(1, n as u32, 24_000.0) else { return };
        buf.copy_to_channel(&f32s, 0).ok();

        let now = self.ctx.current_time();

        // Re-enter priming when the previous turn's queue has drained.
        if !self.priming && now >= self.chain_end {
            self.active_srcs.clear();
            self.priming      = true;
            self.pending      = Vec::new();
            self.pending_ms   = 0.0;
            self.prime_started = Some(now);
        }

        if self.priming {
            if self.prime_started.is_none() {
                self.prime_started = Some(now);
            }
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

    /// Stop all queued audio immediately and discard the queue.
    pub fn clear(&mut self) {
        for src in self.active_srcs.drain(..) {
            src.stop().ok();
        }
        self.gain.gain().set_value(0.0);
        self.chain_end     = self.ctx.current_time();
        self.cleared_at    = js_sys::Date::now();
        self.priming       = true;
        self.pending       = Vec::new();
        self.pending_ms    = 0.0;
        self.prime_started = None;
        self.buffered_ms.set(0);
    }

    fn close(self) {
        self.ctx.close().ok();
    }
}

// ── Module-level API (thread-local singleton) ─────────────────────────────────

thread_local! {
    static SPEAKER: RefCell<Option<Speaker>> = RefCell::new(None);
}

pub fn init_speaker(buffered_ms: Signal<u32>) -> Result<(), wasm_bindgen::JsValue> {
    SPEAKER.with(|s| -> Result<(), wasm_bindgen::JsValue> {
        *s.borrow_mut() = Some(Speaker::new(buffered_ms)?);
        Ok(())
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

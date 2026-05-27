use rustvani_vad::{SileroVad, StateMachine, VadParams};
pub use rustvani_vad::VadState;

pub struct ClientVad {
    vad: SileroVad,
    sm:  StateMachine,
}

impl ClientVad {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            vad: SileroVad::new()?,
            sm:  StateMachine::new(16_000, VadParams::default()),
        })
    }

    /// Feed one PCM chunk (i16 LE bytes). Returns `Some((state, confidence))` when
    /// the VAD window is complete, `None` while still accumulating samples.
    pub fn process(&mut self, bytes: &[u8]) -> Option<(VadState, f32)> {
        let window = self.sm.next_window(bytes)?;
        let confidence = self.vad.infer(&window).unwrap_or(0.0);
        Some((self.sm.advance(confidence, &window), confidence))
    }
}

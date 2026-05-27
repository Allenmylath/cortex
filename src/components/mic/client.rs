use dioxus::prelude::*;
use wasm_bindgen_futures::spawn_local;
use dioxus::fullstack::Message;

use super::types::{Activity, Pipeline};
use super::capture::{start_mic, stop_mic};
use super::playback::{init_speaker, speaker_push, speaker_clear, rms_energy};
use super::ravi::handle_ravi_event;

pub fn start_audio_tasks(pipeline: Pipeline) {
    if init_speaker(pipeline.buffered_ms).is_err() {
        return;
    }

    // Task 1: mic → VAD → server
    let mut activity_mic = pipeline.activity;
    spawn_local(async move {
        if start_mic(pipeline).await.is_ok() {
            activity_mic.set(Activity::Listening);
        }
    });

    // Task 2: server → speaker + RAVI dispatch
    let mut p = pipeline;
    spawn_local(async move {
        loop {
            match p.ws.recv_raw().await {
                Ok(Message::Binary(bytes)) => {
                    p.energy.set(rms_energy(&bytes));
                    if (p.user_speaking)() {
                        speaker_clear();
                        p.is_bot_speaking.set(false);
                    } else {
                        speaker_push(&bytes);
                        if !*p.is_bot_speaking.read() {
                            p.is_bot_speaking.set(true);
                            p.turn_count.with_mut(|c| *c += 1);
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    p.pipeline_texts.with_mut(|v| {
                        if v.len() >= 100 { v.remove(0); }
                        v.push(text.clone());
                    });
                    handle_ravi_event(&text, &mut p);
                }
                Ok(Message::Close { .. }) | Err(_) => break,
                _ => {}
            }
        }

        p.tasks_started.set(false);
        p.activity.set(Activity::Idle);
        p.energy.set(0.0);
        p.is_bot_speaking.set(false);
        p.user_speaking.set(false);
        stop_mic();
    });
}

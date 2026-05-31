use crate::components::mic::{
    use_pipeline, Activity, ChatMessage, FunctionCall, Pipeline, RaviEvent, ServerMessage,
};
use dioxus::prelude::*;

// ── The page ──

#[component]
pub fn VoicePage() -> Element {
    let pipeline = use_pipeline();
    use_context_provider(|| pipeline);
    let mut show_panel = use_signal(|| false);
    let mut show_chat = use_signal(|| false);

    let page_bg = page_bg(&pipeline);

    rsx! {
        div {
            class: "relative min-h-screen flex flex-col overflow-hidden",
            style: "{page_bg} transition: background 0.8s ease; font-family: system-ui;",

            // Chat panel toggle
            button {
                class: "absolute top-4 left-4 z-20 px-3 py-1.5 rounded-full border border-white/15 text-white/70 text-xs cursor-pointer hover:bg-white/10 transition",
                onclick: move |_| show_chat.set(!show_chat()),
                if show_chat() { "Close chat" } else { "Chat" }
            }

            if show_chat() {
                ChatPanel { on_close: move |_| show_chat.set(false) }
            }

            // Info panel toggle
            button {
                class: "absolute top-4 right-4 z-20 px-3 py-1.5 rounded-full border border-white/15 text-white/70 text-xs cursor-pointer hover:bg-white/10 transition",
                onclick: move |_| show_panel.set(!show_panel()),
                if show_panel() { "Close monitor" } else { "Monitor" }
            }

            if show_panel() {
                InfoPanel { on_close: move |_| show_panel.set(false) }
            }

            // Header
            Header {}

            // Main content area
            div { class: "flex-1 flex flex-col items-center justify-center px-4 gap-4",
                Orb {}
                TranscriptBar {}
            }

            // Function call chip
            FunctionCallChip {}

            // Controls
            Controls {}
        }
    }
}

// ── Header ──

#[component]
fn Header() -> Element {
    let p = use_context::<Pipeline>();
    let status = p.status.read();
    let activity = p.activity.read();

    let dot_color = match *status {
        "connected" => "bg-green-400",
        "connecting" => "bg-yellow-400 animate-pulse",
        "error" => "bg-red-400",
        _ => "bg-gray-400",
    };

    let status_text = match *activity {
        Activity::Idle => "Idle",
        Activity::Listening => "Listening...",
        Activity::Processing => "Thinking...",
        Activity::Speaking => "Speaking...",
    };

    rsx! {
        header {
            class: "flex items-center justify-between px-6 py-4 shrink-0",
            div { class: "flex items-center gap-2",
                span { class: "h-2 w-2 rounded-full {dot_color}" }
                h1 { class: "text-white/90 text-lg font-medium", "Voice Assistant" }
            }
            span { class: "text-white/50 text-sm", "{status_text}" }
        }
    }
}

// ── Orb ──

#[component]
fn Orb() -> Element {
    let p = use_context::<Pipeline>();
    let energy = p.energy.read();
    let activity = p.activity.read();

    let scale = 1.0 + *energy * 0.4;
    let blur = 40.0 + *energy * 60.0;

    let color = match *activity {
        Activity::Idle       => "120, 120, 200",
        Activity::Listening  => "100, 220, 140",
        Activity::Processing => "240, 200, 60",
        Activity::Speaking   => "220, 100, 120",
    };

    let opacity = match *activity {
        Activity::Idle => 0.3,
        _              => 0.4 + *energy * 0.4,
    };

    let orb_style = format!(
        "width: 180px; height: 180px; border-radius: 50%; background: rgba({color},{opacity}); \
         box-shadow: 0 0 {blur}px rgba({color},{opacity}); transform: scale({scale}); \
         transition: all 0.15s ease-out;"
    );

    rsx! {
        div {
            class: "flex items-center justify-center py-6",
            div { class: "orb", style: "{orb_style}" }
        }
    }
}

// ── TranscriptBar ──

#[component]
fn TranscriptBar() -> Element {
    let p = use_context::<Pipeline>();
    let bot_text = p.bot_text.read();
    let messages = p.messages.read();

    let text = if !bot_text.is_empty() {
        bot_text.clone()
    } else {
        messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant")
            .map(|m| m.text.clone())
            .unwrap_or_default()
    };

    let opacity = if !text.is_empty() { 1.0 } else { 0.0 };

    rsx! {
        div {
            class: "text-center px-4 min-h-[1.5rem] transition-opacity duration-300",
            style: "opacity: {opacity};",
            span { class: "text-white/70 text-base", "{text}" }
        }
    }
}

// ── MessageList ──

#[component]
fn MessageList() -> Element {
    let p = use_context::<Pipeline>();
    let messages = p.messages.read();

    rsx! {
        div {
            class: "flex flex-col-reverse gap-3 w-full h-full overflow-y-auto pr-1",
            for msg in messages.iter().rev() {
                MessageBubble {
                    role: msg.role.clone(),
                    text: msg.text.clone(),
                    final_msg: msg.final_msg,
                }
            }
        }
    }
}

#[component]
fn MessageBubble(role: String, text: String, final_msg: bool) -> Element {
    let is_user = role == "user";
    let align = if is_user { "self-end items-end" } else { "self-start items-start" };
    let bg = if is_user { "rgba(59,130,246,0.35)" } else { "rgba(255,255,255,0.08)" };
    let label = if is_user { "You" } else { "Bot" };
    let label_color = if is_user { "text-blue-300" } else { "text-gray-400" };

    rsx! {
        div {
            class: "flex flex-col max-w-[80%] {align} gap-0.5",
            span { class: "text-[10px] {label_color} px-1", "{label}" }
            div {
                class: "rounded-xl px-3 py-2 text-sm break-words",
                style: "background: {bg}; color: rgba(255,255,255,0.9);",
                if !text.is_empty() {
                    "{text}"
                } else if !final_msg {
                    span { class: "inline-flex gap-1 text-white/50",
                        span { class: "animate-bounce", style: "animation-delay: 0ms;", "•" }
                        span { class: "animate-bounce", style: "animation-delay: 150ms;", "•" }
                        span { class: "animate-bounce", style: "animation-delay: 300ms;", "•" }
                    }
                }
            }
        }
    }
}

// ── FunctionCallChip ──

#[component]
fn FunctionCallChip() -> Element {
    let p = use_context::<Pipeline>();
    let calls = p.function_calls.read();

    let active = calls.iter().rev().find(|c| c.state == "in-progress");

    match active {
        Some(fc) => rsx! {
            div {
                class: "mx-auto mb-3 flex items-center gap-2 rounded-full border border-yellow-500/30 px-4 py-1.5 text-xs text-yellow-200",
                style: "background: rgba(240,200,60,0.1);",
                "⚡"
                span { class: "font-medium", "{friendly_label(&fc.name)}" }
                span { class: "inline-flex gap-0.5 opacity-70",
                    span { class: "animate-bounce", style: "animation-delay: 0ms;", "•" }
                    span { class: "animate-bounce", style: "animation-delay: 150ms;", "•" }
                    span { class: "animate-bounce", style: "animation-delay: 300ms;", "•" }
                }
            }
        },
        None => rsx! {}
    }
}

fn friendly_label(name: &str) -> String {
    match name {
        "get_menu" | "view_menu" => "Fetching menu",
        "add_to_cart" => "Adding to cart",
        "remove_from_cart" | "update_cart" => "Updating cart",
        "view_cart" => "Checking cart",
        "view_order" => "Reviewing order",
        "place_order" => "Sending to kitchen",
        _ => name,
    }.to_string()
}

// ── ChatPanel ──

#[component]
fn ChatPanel(on_close: EventHandler<()>) -> Element {
    let p = use_context::<Pipeline>();
    let status = p.status.read();
    let mut input = use_signal(|| String::new());
    let is_connected = *status == "connected";

    rsx! {
        aside {
            class: "absolute left-0 top-0 z-10 h-full w-80 flex flex-col border-r border-white/10 text-white/80",
            style: "background: rgba(10,10,15,0.92); backdrop-filter: blur(8px);",

            // Header
            div { class: "flex items-center justify-between px-4 py-3 border-b border-white/10 shrink-0",
                h2 { class: "text-sm font-semibold", "Chat" }
                button {
                    class: "text-xs text-white/50 hover:text-white transition",
                    onclick: move |_| on_close.call(()),
                    "Close"
                }
            }

            // Messages
            div { class: "flex-1 overflow-y-auto px-4 py-3 min-h-0",
                MessageList {}
            }

            // Input
            div { class: "px-4 py-3 border-t border-white/10 shrink-0",
                form {
                    class: "flex gap-2",
                    onsubmit: move |e| {
                        e.prevent_default();
                        if !input().trim().is_empty() {
                            p.send_text(input());
                            input.set(String::new());
                        }
                    },
                    input {
                        class: "flex-1 rounded-full bg-white/10 border border-white/15 px-4 py-2 text-sm text-white placeholder-white/40 outline-none focus:border-white/30",
                        placeholder: if is_connected { "Type a message..." } else { "Connect to send messages" },
                        disabled: !is_connected,
                        value: "{input}",
                        oninput: move |e| input.set(e.value()),
                    }
                    button {
                        class: "rounded-full bg-white/15 px-4 py-2 text-sm text-white hover:bg-white/25 disabled:opacity-30 disabled:cursor-not-allowed transition",
                        r#type: "submit",
                        disabled: !is_connected || input().trim().is_empty(),
                        "Send"
                    }
                }
            }
        }
    }
}

// ── Controls ──

#[component]
fn Controls() -> Element {
    let p = use_context::<Pipeline>();
    let status = p.status.read();
    let activity = p.activity.read();

    rsx! {
        div {
            class: "flex justify-center gap-3 pb-8 pt-2 shrink-0",
            match *status {
                "idle" | "disconnected" | "error" => rsx! {
                    button {
                        class: "rounded-full px-6 py-3 text-sm font-medium text-white border transition cursor-pointer",
                        style: "background: rgba(100,220,140,0.25); border-color: rgba(100,220,140,0.3);",
                        onclick: move |_| p.connect(),
                        "Connect"
                    }
                },
                "connected" => rsx! {
                    if *activity == Activity::Speaking {
                        button {
                            class: "rounded-full px-6 py-3 text-sm font-medium text-white border transition cursor-pointer",
                            style: "background: rgba(240,200,60,0.25); border-color: rgba(240,200,60,0.3);",
                            onclick: move |_| p.interrupt(),
                            "Interrupt"
                        }
                    }
                    button {
                        class: "rounded-full px-6 py-3 text-sm font-medium text-white border transition cursor-pointer",
                        style: "background: rgba(220,100,100,0.25); border-color: rgba(220,100,100,0.3);",
                        onclick: move |_| p.disconnect(),
                        "End"
                    }
                },
                _ => rsx! {
                    span { class: "text-white/50 text-sm", "Connecting..." }
                }
            }
        }
    }
}

// ── InfoPanel ──

#[component]
fn InfoPanel(on_close: EventHandler<()>) -> Element {
    let p = use_context::<Pipeline>();
    let status = p.status.read();
    let buffered = p.buffered_ms.read();
    let turn_count = p.turn_count.read();
    let interrupt_count = p.interrupt_count.read();
    let is_bot_speaking = p.is_bot_speaking.read();
    let energy = p.energy.read();
    let vad_prob = p.vad_prob.read();
    let last_error = p.last_error.read();
    let events = p.events.read();
    let function_calls = p.function_calls.read();
    let pipeline_texts = p.pipeline_texts.read();

    let mut vad_only = use_signal(|| false);

    let last_func = function_calls.iter().rev().next();
    let vad_color = if *vad_prob >= 0.7 { "rgb(100,220,140)" } else { "rgb(100,180,255)" };
    let vad_width = *vad_prob * 100.0;

    rsx! {
        aside {
            class: "absolute right-0 top-0 z-10 h-full w-80 flex flex-col border-l border-white/10 text-white/80",
            style: "background: rgba(10,10,15,0.92); backdrop-filter: blur(8px);",

            // Header
            div { class: "flex items-center justify-between px-4 py-3 border-b border-white/10 shrink-0",
                h2 { class: "text-sm font-semibold", "Kitchen Monitor" }
                button {
                    class: "text-xs text-white/50 hover:text-white transition",
                    onclick: move |_| on_close.call(()),
                    "Close"
                }
            }

            // Last error
            if let Some(err) = last_error.as_ref() {
                div { class: "mx-4 mt-3 p-2 rounded text-xs break-words",
                    style: "background: rgba(220,80,80,0.12); border: 1px solid rgba(220,80,80,0.3); color: rgb(255,160,160);",
                    "⚠ {err}"
                }
            }

            // Stats
            div { class: "px-4 py-3 border-b border-white/10 shrink-0",
                h3 { class: "text-[10px] uppercase tracking-widest text-white/40 mb-2", "Service Stats" }
                div { class: "grid grid-cols-2 gap-y-1 text-xs",
                    StatRow { label: "Connection".to_string(), value: status.to_string() }
                    StatRow { label: "Turns".to_string(), value: turn_count.to_string() }
                    StatRow { label: "Interrupts".to_string(), value: interrupt_count.to_string() }
                    StatRow { label: "Buffered".to_string(), value: format!("{buffered} ms") }
                    StatRow { label: "Bot speaking".to_string(), value: if *is_bot_speaking { "yes".to_string() } else { "no".to_string() } }
                    StatRow { label: "Audio peak".to_string(), value: format!("{energy:.2}") }
                    StatRow { label: "VAD prob".to_string(), value: format!("{vad_prob:.2}") }
                }
                div { class: "mt-2 h-1 w-full rounded-full overflow-hidden",
                    style: "background: rgba(255,255,255,0.1);",
                    div {
                        class: "h-full transition-all duration-75",
                        style: "width: {vad_width}%; background: {vad_color};",
                    }
                }
            }

            // Last action
            if let Some(fc) = last_func {
                div { class: "px-4 py-3 border-b border-white/10 shrink-0",
                    h3 { class: "text-[10px] uppercase tracking-widest text-white/40 mb-2", "Last Action" }
                    div { class: "rounded border border-white/10 p-2 text-xs",
                        style: "background: rgba(255,255,255,0.03);",
                        div { class: "flex justify-between gap-2",
                            span { class: "font-mono text-white truncate", "{fc.name}" }
                            span {
                                class: "rounded px-1.5 py-0.5 text-[10px] uppercase shrink-0",
                                style: if fc.state == "in-progress" {
                                    "background: rgba(240,200,60,0.15); color: rgb(240,200,60);"
                                } else {
                                    "background: rgba(100,220,140,0.15); color: rgb(100,220,140);"
                                },
                                "{fc.state}"
                            }
                        }
                        if let Some(payload) = fc.payload.as_ref() {
                            pre { class: "mt-1 max-h-24 overflow-auto text-[10px] break-all",
                                style: "color: rgba(255,255,255,0.45);",
                                "{payload}"
                            }
                        }
                    }
                }
            }

            // Pipeline messages (raw JSON)
            div { class: "flex-1 overflow-y-auto px-4 py-3 min-h-0",
                h3 { class: "text-[10px] uppercase tracking-widest text-white/40 mb-2", "Pipeline Messages" }
                if pipeline_texts.is_empty() {
                    p { class: "text-xs", style: "color: rgba(255,255,255,0.3);", "No messages yet." }
                } else {
                    div { class: "flex flex-col gap-2",
                        for text in pipeline_texts.iter().rev().take(100) {
                            pre { class: "rounded border border-white/5 p-2 text-[10px] break-all",
                                style: "background: rgba(255,255,255,0.03); color: rgba(255,255,255,0.5);",
                                "{text}"
                            }
                        }
                    }
                }
            }

            // Event log
            div { class: "flex-1 overflow-y-auto px-4 py-3 min-h-0",
                div { class: "flex items-center justify-between mb-2",
                    h3 { class: "text-[10px] uppercase tracking-widest text-white/40", "Event Log" }
                    button {
                        class: "text-[10px] px-2 py-0.5 rounded-full border cursor-pointer transition",
                        style: if vad_only() {
                            "background: rgba(100,220,140,0.2); border-color: rgba(100,220,140,0.4); color: rgb(100,220,140);"
                        } else {
                            "background: transparent; border-color: rgba(255,255,255,0.15); color: rgba(255,255,255,0.4);"
                        },
                        onclick: move |_| vad_only.set(!vad_only()),
                        "VAD only"
                    }
                }
                if events.is_empty() {
                    p { class: "text-xs", style: "color: rgba(255,255,255,0.3);", "No activity yet." }
                } else {
                    div { class: "flex flex-col gap-1",
                        for e in events.iter().rev()
                            .filter(|e| {
                                if !vad_only() { return true; }
                                matches!(e.event_type.as_str(),
                                    "user-started-speaking" | "user-stopped-speaking" | "interrupt"
                                )
                            })
                            .take(50)
                        {
                            div { class: "rounded border border-white/5 px-2 py-1 text-[11px]",
                                style: "background: rgba(255,255,255,0.03);",
                                div { class: "flex justify-between gap-2",
                                    span { class: "font-mono text-white truncate", "{e.event_type}" }
                                    span { class: "shrink-0 text-[10px]",
                                        style: "color: rgba(255,255,255,0.35);",
                                        "{format_time(e.ts)}"
                                    }
                                }
                                if let Some(d) = e.detail.as_ref() {
                                    p { class: "mt-0.5 text-[10px] break-words",
                                        style: "color: rgba(255,255,255,0.4);",
                                        "{d}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between",
            span { style: "color: rgba(255,255,255,0.45);", "{label}" }
            span { class: "font-mono font-medium text-white", "{value}" }
        }
    }
}

// ── Helpers ──

fn page_bg(p: &Pipeline) -> String {
    let activity = p.activity.read();
    let bg = match *activity {
        Activity::Idle       => "15, 15, 30",
        Activity::Listening  => "15, 25, 20",
        Activity::Processing => "25, 22, 15",
        Activity::Speaking   => "25, 15, 18",
    };
    format!("background: rgb({bg});")
}

fn format_time(ts: u64) -> String {
    let secs = ts / 1000;
    let mins = (secs / 60) % 60;
    let hrs = (secs / 3600) % 24;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hrs, mins, s)
}
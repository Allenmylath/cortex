use crate::models::client::{create_client, delete_client, get_clients, Client};
use dioxus::prelude::*;

#[component]
pub fn Clients() -> Element {
    let mut show_modal = use_signal(|| false);
    let mut clients = use_resource(move || async move { get_clients().await });

    // Form state
    let mut f_name = use_signal(String::new);
    let mut f_email = use_signal(String::new);
    let mut f_phone = use_signal(String::new);
    let mut f_budget_min = use_signal(String::new);
    let mut f_budget_max = use_signal(String::new);
    let mut f_areas = use_signal(String::new);
    let mut f_notes = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error_msg = use_signal(String::new);

    let on_submit = move |_| {
        let name = f_name();
        let email = f_email();
        if name.trim().is_empty() || email.trim().is_empty() {
            error_msg.set("Name and email are required.".into());
            return;
        }
        let budget_min = f_budget_min().trim().parse::<i64>().ok();
        let budget_max = f_budget_max().trim().parse::<i64>().ok();

        submitting.set(true);
        error_msg.set(String::new());

        spawn(async move {
            match create_client(name, email, f_phone(), budget_min, budget_max, f_areas(), f_notes()).await {
                Ok(_) => {
                    show_modal.set(false);
                    f_name.set(String::new());
                    f_email.set(String::new());
                    f_phone.set(String::new());
                    f_budget_min.set(String::new());
                    f_budget_max.set(String::new());
                    f_areas.set(String::new());
                    f_notes.set(String::new());
                    clients.restart();
                }
                Err(e) => {
                    error_msg.set(e.to_string());
                }
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "max-w-5xl mx-auto",
            // Header row
            div { class: "flex items-center justify-between mb-6",
                div {
                    h1 { class: "text-xl font-bold text-slate-800", "Clients" }
                    p { class: "text-slate-500 text-sm", "Manage your client relationships" }
                }
                button {
                    class: "bg-slate-800 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-slate-700 transition-colors flex items-center gap-2",
                    onclick: move |_| show_modal.set(true),
                    span { "+" }
                    "Add Client"
                }
            }

            // Client list
            {
                match clients() {
                    None => rsx! {
                        div { class: "space-y-3",
                            for _ in 0..3 {
                                div { class: "bg-white rounded-xl border border-slate-100 p-5 h-20 animate-pulse" }
                            }
                        }
                    },
                    Some(Err(_)) => rsx! {
                        div { class: "bg-red-50 text-red-600 rounded-xl p-5 text-sm", "Failed to load clients." }
                    },
                    Some(Ok(list)) => {
                        if list.is_empty() {
                            rsx! {
                                div { class: "bg-white rounded-xl border border-slate-100 p-12 text-center",
                                    p { class: "text-4xl mb-3", "👥" }
                                    p { class: "text-slate-600 font-medium", "No clients yet" }
                                    p { class: "text-slate-400 text-sm mt-1", "Add your first client to get started." }
                                    button {
                                        class: "mt-4 bg-slate-800 text-white px-4 py-2 rounded-lg text-sm hover:bg-slate-700",
                                        onclick: move |_| show_modal.set(true),
                                        "Add Client"
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "space-y-3",
                                    for client in list {
                                        ClientRow {
                                            key: "{client.id}",
                                            client: client.clone(),
                                            on_delete: move |id: String| {
                                                spawn(async move {
                                                    let _ = delete_client(id).await;
                                                    clients.restart();
                                                });
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add Client modal
        if show_modal() {
            div {
                class: "fixed inset-0 z-50 bg-black/40 flex justify-end",
                onclick: move |_| show_modal.set(false),
                div {
                    class: "w-96 h-full bg-white shadow-xl flex flex-col overflow-y-auto",
                    onclick: move |e| e.stop_propagation(),

                    div { class: "flex items-center justify-between px-6 py-5 border-b border-slate-100",
                        h2 { class: "font-semibold text-slate-800", "Add New Client" }
                        button {
                            class: "text-slate-400 hover:text-slate-600 text-xl",
                            onclick: move |_| show_modal.set(false),
                            "✕"
                        }
                    }

                    div { class: "flex-1 px-6 py-5 space-y-4",
                        if !error_msg().is_empty() {
                            div { class: "bg-red-50 text-red-600 text-sm rounded-lg p-3", "{error_msg}" }
                        }
                        FormField { label: "Full Name *", placeholder: "Jane Smith",
                            value: f_name(),
                            oninput: move |e: Event<FormData>| f_name.set(e.value()),
                        }
                        FormField { label: "Email *", placeholder: "jane@example.com",
                            value: f_email(),
                            oninput: move |e: Event<FormData>| f_email.set(e.value()),
                        }
                        FormField { label: "Phone", placeholder: "+1 (555) 000-0000",
                            value: f_phone(),
                            oninput: move |e: Event<FormData>| f_phone.set(e.value()),
                        }
                        div { class: "grid grid-cols-2 gap-3",
                            FormField { label: "Budget Min ($)", placeholder: "200000",
                                value: f_budget_min(),
                                oninput: move |e: Event<FormData>| f_budget_min.set(e.value()),
                            }
                            FormField { label: "Budget Max ($)", placeholder: "500000",
                                value: f_budget_max(),
                                oninput: move |e: Event<FormData>| f_budget_max.set(e.value()),
                            }
                        }
                        FormField { label: "Preferred Areas", placeholder: "Downtown, Suburbs",
                            value: f_areas(),
                            oninput: move |e: Event<FormData>| f_areas.set(e.value()),
                        }
                        div {
                            label { class: "block text-sm font-medium text-slate-700 mb-1", "Notes" }
                            textarea {
                                class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300 resize-none",
                                rows: "3",
                                placeholder: "Any additional notes...",
                                value: "{f_notes}",
                                oninput: move |e| f_notes.set(e.value()),
                            }
                        }
                    }

                    div { class: "px-6 py-4 border-t border-slate-100 flex gap-3",
                        button {
                            class: "flex-1 bg-slate-800 text-white py-2.5 rounded-lg text-sm font-medium hover:bg-slate-700 transition-colors disabled:opacity-50",
                            disabled: submitting(),
                            onclick: on_submit,
                            if submitting() { "Saving..." } else { "Save Client" }
                        }
                        button {
                            class: "px-4 py-2.5 border border-slate-200 rounded-lg text-sm text-slate-600 hover:bg-slate-50",
                            onclick: move |_| show_modal.set(false),
                            "Cancel"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ClientRow(client: Client, on_delete: EventHandler<String>) -> Element {
    let budget = match (client.budget_min, client.budget_max) {
        (Some(min), Some(max)) => format!("${} – ${}", format_money(min), format_money(max)),
        (Some(min), None) => format!("From ${}", format_money(min)),
        (None, Some(max)) => format!("Up to ${}", format_money(max)),
        _ => "No budget set".into(),
    };

    let status_cls = if client.status == "active" {
        "bg-green-100 text-green-700"
    } else {
        "bg-slate-100 text-slate-500"
    };

    rsx! {
        div {
            class: "bg-white rounded-xl border border-slate-100 p-5 flex items-center gap-4 hover:shadow-sm transition-shadow",
            div {
                class: "w-10 h-10 rounded-full bg-blue-100 flex items-center justify-center text-blue-600 font-semibold text-sm flex-shrink-0",
                {client.name.chars().next().map(|c| c.to_ascii_uppercase()).unwrap_or('?').to_string()}
            }
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-2",
                    p { class: "font-medium text-slate-800 truncate", "{client.name}" }
                    span { class: "text-xs px-2 py-0.5 rounded-full font-medium {status_cls}", "{client.status}" }
                }
                p { class: "text-slate-500 text-sm truncate", "{client.email}" }
            }
            div { class: "text-right hidden sm:block",
                p { class: "text-sm font-medium text-slate-700", "{budget}" }
                p { class: "text-xs text-slate-400",
                    "{client.preferred_areas.as_deref().unwrap_or(\"No area preference\")}"
                }
            }
            p { class: "text-xs text-slate-400 hidden md:block", "{client.created_at}" }
            button {
                class: "text-slate-300 hover:text-red-400 transition-colors ml-2 flex-shrink-0",
                title: "Delete client",
                onclick: {
                    let id = client.id.clone();
                    move |_| on_delete.call(id.clone())
                },
                "🗑"
            }
        }
    }
}

#[component]
fn FormField(
    label: String,
    placeholder: String,
    value: String,
    oninput: EventHandler<Event<FormData>>,
) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-slate-700 mb-1", "{label}" }
            input {
                class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300",
                r#type: "text",
                placeholder: "{placeholder}",
                value: "{value}",
                oninput: move |e| oninput.call(e),
            }
        }
    }
}

fn format_money(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}

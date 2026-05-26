use crate::models::client::get_clients;
use crate::models::property::get_properties;
use dioxus::prelude::*;

fn fmt_budget(min: Option<i64>, max: Option<i64>) -> String {
    match (min, max) {
        (Some(a), Some(b)) => format!("Budget: ${} – ${}", fmt_price(a), fmt_price(b)),
        (Some(a), None) => format!("Budget: from ${}", fmt_price(a)),
        (None, Some(b)) => format!("Budget: up to ${}", fmt_price(b)),
        _ => "No budget set".into(),
    }
}

fn fmt_price(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}

#[component]
pub fn Matches() -> Element {
    let clients = use_resource(move || async move { get_clients().await });
    let properties = use_resource(move || async move { get_properties().await });

    let clients_ok = clients().and_then(|r| r.ok()).unwrap_or_default();
    let props_ok = properties().and_then(|r| r.ok()).unwrap_or_default();
    let loading = clients().is_none() || properties().is_none();

    rsx! {
        div { class: "max-w-5xl mx-auto",
            div { class: "mb-6",
                h1 { class: "text-xl font-bold text-slate-800", "Smart Matches" }
                p { class: "text-slate-500 text-sm",
                    "Clients matched to properties within their budget range"
                }
            }

            if loading {
                div { class: "space-y-4",
                    for _ in 0..3 {
                        div { class: "bg-white rounded-xl border border-slate-100 p-5 h-24 animate-pulse" }
                    }
                }
            } else if clients_ok.is_empty() || props_ok.is_empty() {
                div { class: "bg-white rounded-xl border border-slate-100 p-12 text-center",
                    p { class: "text-4xl mb-3", "🔗" }
                    p { class: "text-slate-600 font-medium", "No matches available" }
                    p { class: "text-slate-400 text-sm mt-1",
                        "Add clients with budgets and properties with prices to see matches."
                    }
                    div { class: "flex gap-3 justify-center mt-4",
                        Link {
                            class: "text-sm bg-slate-800 text-white px-4 py-2 rounded-lg hover:bg-slate-700",
                            to: crate::Route::Clients {},
                            "Add Client"
                        }
                        Link {
                            class: "text-sm border border-slate-200 text-slate-600 px-4 py-2 rounded-lg hover:bg-slate-50",
                            to: crate::Route::Properties {},
                            "Add Property"
                        }
                    }
                }
            } else {
                div { class: "space-y-4",
                    for client in &clients_ok {
                        {
                            let matched: Vec<_> = props_ok.iter().filter(|p| {
                                p.status == "available" && match (client.budget_min, client.budget_max) {
                                    (Some(min), Some(max)) => p.price >= min && p.price <= max,
                                    (Some(min), None) => p.price >= min,
                                    (None, Some(max)) => p.price <= max,
                                    _ => false,
                                }
                            }).collect();

                            if !matched.is_empty() {
                                let initial = client.name.chars().next()
                                    .map(|c| c.to_ascii_uppercase())
                                    .unwrap_or('?')
                                    .to_string();
                                let budget_label = fmt_budget(client.budget_min, client.budget_max);
                                let match_label = if matched.len() == 1 {
                                    "1 match".to_string()
                                } else {
                                    format!("{} matches", matched.len())
                                };

                                rsx! {
                                    div {
                                        class: "bg-white rounded-xl border border-slate-100 p-5",
                                        key: "{client.id}",
                                        div { class: "flex items-center gap-3 mb-4",
                                            div {
                                                class: "w-9 h-9 rounded-full bg-blue-100 flex items-center justify-center text-blue-600 font-semibold text-sm",
                                                "{initial}"
                                            }
                                            div {
                                                p { class: "font-medium text-slate-800", "{client.name}" }
                                                p { class: "text-slate-400 text-xs", "{budget_label}" }
                                            }
                                            span {
                                                class: "ml-auto text-xs bg-blue-50 text-blue-600 px-2.5 py-1 rounded-full font-medium",
                                                "{match_label}"
                                            }
                                        }
                                        div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2",
                                            for prop in matched {
                                                {
                                                    let price_str = fmt_price(prop.price);
                                                    rsx! {
                                                        div {
                                                            class: "flex items-center gap-3 bg-slate-50 rounded-lg p-3",
                                                            span { class: "text-lg flex-shrink-0", "🏠" }
                                                            div { class: "min-w-0",
                                                                p { class: "text-sm font-medium text-slate-700 truncate", "{prop.address}" }
                                                                p { class: "text-xs text-slate-500",
                                                                    "${price_str} · {prop.city}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! { div { key: "{client.id}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

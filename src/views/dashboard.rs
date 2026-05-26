use crate::components::{MicCapture, StatCard};
use crate::models::stats::get_dashboard_stats;
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let stats = use_resource(move || async move { get_dashboard_stats().await });

    rsx! {
        div { class: "max-w-5xl mx-auto",
            // Greeting
            div { class: "mb-6 flex items-start justify-between",
                div {
                    h1 { class: "text-2xl font-bold text-slate-800", "Good day, Realtor" }
                    p { class: "text-slate-500 text-sm mt-1", "Welcome back to your dashboard" }
                }
                MicCapture {}
            }

            // Stat cards
            {
                match stats() {
                    Some(Ok(s)) => rsx! {
                        div { class: "grid grid-cols-2 gap-4 mb-6 lg:grid-cols-4",
                            StatCard {
                                label: "Active Clients".to_string(),
                                value: s.total_clients,
                                icon: "👥".to_string(),
                                icon_bg: "bg-blue-50".to_string(),
                                icon_color: "text-blue-600".to_string(),
                                href: "/clients".to_string(),
                            }
                            StatCard {
                                label: "Listed Properties".to_string(),
                                value: s.active_properties,
                                icon: "🏠".to_string(),
                                icon_bg: "bg-green-50".to_string(),
                                icon_color: "text-green-600".to_string(),
                                href: "/properties".to_string(),
                            }
                            StatCard {
                                label: "Pending Deals".to_string(),
                                value: s.pending_deals,
                                icon: "⏳".to_string(),
                                icon_bg: "bg-amber-50".to_string(),
                                icon_color: "text-amber-600".to_string(),
                                href: "/properties".to_string(),
                            }
                            StatCard {
                                label: "Appts This Week".to_string(),
                                value: s.appointments_this_week,
                                icon: "📅".to_string(),
                                icon_bg: "bg-purple-50".to_string(),
                                icon_color: "text-purple-600".to_string(),
                                href: "/".to_string(),
                            }
                        }
                    },
                    Some(Err(_)) => rsx! {
                        div { class: "grid grid-cols-2 gap-4 mb-6 lg:grid-cols-4",
                            for _ in 0..4 {
                                div { class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5 h-20 animate-pulse" }
                            }
                        }
                    },
                    None => rsx! {
                        div { class: "grid grid-cols-2 gap-4 mb-6 lg:grid-cols-4",
                            for _ in 0..4 {
                                div { class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5 h-20 animate-pulse" }
                            }
                        }
                    },
                }
            }

            // Quick links
            div { class: "grid grid-cols-2 gap-4",
                div { class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5",
                    div { class: "flex items-center gap-2 mb-3",
                        span { class: "text-base", "👥" }
                        h2 { class: "font-semibold text-slate-700", "Recent Clients" }
                    }
                    p { class: "text-slate-400 text-sm",
                        "Navigate to the Clients tab to view and manage your client list."
                    }
                    Link {
                        class: "inline-block mt-3 text-sm text-blue-600 hover:underline font-medium",
                        to: crate::Route::Clients {},
                        "View all clients →"
                    }
                }
                div { class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5",
                    div { class: "flex items-center gap-2 mb-3",
                        span { class: "text-base", "🏠" }
                        h2 { class: "font-semibold text-slate-700", "Property Listings" }
                    }
                    p { class: "text-slate-400 text-sm",
                        "Browse available, pending, and sold properties in your portfolio."
                    }
                    Link {
                        class: "inline-block mt-3 text-sm text-blue-600 hover:underline font-medium",
                        to: crate::Route::Properties {},
                        "View all properties →"
                    }
                }
                div { class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5",
                    div { class: "flex items-center gap-2 mb-3",
                        span { class: "text-base", "🔗" }
                        h2 { class: "font-semibold text-slate-700", "Smart Matches" }
                    }
                    p { class: "text-slate-400 text-sm",
                        "Match clients to properties based on budget and preferences."
                    }
                    Link {
                        class: "inline-block mt-3 text-sm text-blue-600 hover:underline font-medium",
                        to: crate::Route::Matches {},
                        "View matches →"
                    }
                }
                div { class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5",
                    div { class: "flex items-center gap-2 mb-3",
                        span { class: "text-base", "✨" }
                        h2 { class: "font-semibold text-slate-700", "Getting Started" }
                    }
                    p { class: "text-slate-400 text-sm",
                        "Add your first client or property to start tracking deals."
                    }
                    div { class: "flex gap-2 mt-3",
                        Link {
                            class: "text-sm text-blue-600 hover:underline font-medium",
                            to: crate::Route::Clients {},
                            "Add client"
                        }
                        span { class: "text-slate-300", "·" }
                        Link {
                            class: "text-sm text-blue-600 hover:underline font-medium",
                            to: crate::Route::Properties {},
                            "Add property"
                        }
                    }
                }
            }
        }
    }
}

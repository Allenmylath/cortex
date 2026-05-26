use crate::Route;
use dioxus::prelude::*;

struct NavItem {
    label: &'static str,
    icon: &'static str,
    route: Route,
}

#[component]
pub fn Sidebar() -> Element {
    let current = use_route::<Route>();

    let items = vec![
        NavItem { label: "Dashboard", icon: "⊞", route: Route::Dashboard {} },
        NavItem { label: "Clients", icon: "👥", route: Route::Clients {} },
        NavItem { label: "Properties", icon: "🏠", route: Route::Properties {} },
        NavItem { label: "Matches", icon: "🔗", route: Route::Matches {} },
        NavItem { label: "Settings", icon: "⚙", route: Route::Settings {} },
    ];

    rsx! {
        aside {
            class: "w-56 bg-slate-900 flex flex-col h-full flex-shrink-0",

            // Logo
            div {
                class: "flex items-center gap-2 px-5 py-5 border-b border-slate-700/60",
                span { class: "text-2xl", "🏡" }
                span { class: "text-white font-semibold text-base tracking-tight", "RealtyPro" }
            }

            // Nav items
            nav { class: "flex-1 py-4 flex flex-col gap-1 px-2",
                for item in items {
                    {
                        let is_active = current == item.route;
                        let base = "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors";
                        let cls = if is_active {
                            format!("{base} bg-slate-700 text-white")
                        } else {
                            format!("{base} text-slate-400 hover:bg-slate-800 hover:text-white")
                        };
                        rsx! {
                            Link {
                                key: "{item.label}",
                                class: "{cls}",
                                to: item.route,
                                span { class: "text-base w-5 text-center", "{item.icon}" }
                                span { "{item.label}" }
                            }
                        }
                    }
                }
            }

            // User area
            div {
                class: "px-4 py-4 border-t border-slate-700/60 flex items-center gap-3",
                div {
                    class: "w-8 h-8 rounded-full bg-blue-500 flex items-center justify-center text-white text-sm font-semibold flex-shrink-0",
                    "R"
                }
                div { class: "min-w-0",
                    p { class: "text-white text-sm font-medium truncate", "Realtor" }
                    p { class: "text-slate-500 text-xs truncate", "Agent" }
                }
            }
        }
    }
}

use crate::components::sidebar::Sidebar;
use dioxus::prelude::*;

#[component]
pub fn MainLayout() -> Element {
    rsx! {
        div { class: "flex h-screen overflow-hidden bg-slate-50",
            Sidebar {}
            div { class: "flex-1 flex flex-col overflow-hidden",
                // Top header
                header {
                    class: "h-14 bg-white border-b border-slate-200 flex items-center justify-between px-6 flex-shrink-0",
                    p { class: "text-slate-400 text-sm", "RealtyPro — Property Management" }
                    div { class: "flex items-center gap-3",
                        span { class: "text-slate-500 text-sm", "🔔" }
                        div {
                            class: "w-8 h-8 rounded-full bg-slate-200 flex items-center justify-center text-slate-600 text-sm font-medium",
                            "DR"
                        }
                    }
                }
                // Page content
                main { class: "flex-1 overflow-y-auto p-6",
                    Outlet::<crate::Route> {}
                }
            }
        }
    }
}

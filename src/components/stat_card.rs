use dioxus::prelude::*;

#[component]
pub fn StatCard(
    label: String,
    value: i64,
    icon: String,
    icon_bg: String,
    icon_color: String,
    href: String,
) -> Element {
    rsx! {
        div {
            class: "bg-white rounded-xl shadow-sm border border-slate-100 p-5 flex items-center gap-4 flex-1 min-w-0 hover:shadow-md transition-shadow cursor-pointer",
            div {
                class: "w-12 h-12 rounded-xl flex items-center justify-center flex-shrink-0 text-xl {icon_bg}",
                "{icon}"
            }
            div { class: "min-w-0",
                p { class: "text-3xl font-bold text-slate-800 leading-tight", "{value}" }
                p { class: "text-sm text-slate-500 mt-0.5 truncate", "{label}" }
            }
            div { class: "ml-auto text-slate-300 flex-shrink-0", "→" }
        }
    }
}

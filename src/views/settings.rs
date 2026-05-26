use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    rsx! {
        div { class: "max-w-2xl mx-auto",
            div { class: "mb-6",
                h1 { class: "text-xl font-bold text-slate-800", "Settings" }
                p { class: "text-slate-500 text-sm", "Configure your RealtyPro workspace" }
            }

            div { class: "space-y-4",
                // Profile section
                div { class: "bg-white rounded-xl border border-slate-100 p-6",
                    h2 { class: "font-semibold text-slate-700 mb-4 flex items-center gap-2",
                        span { "👤" }
                        "Profile"
                    }
                    div { class: "space-y-3",
                        div { class: "grid grid-cols-2 gap-3",
                            div {
                                label { class: "block text-sm font-medium text-slate-700 mb-1", "First Name" }
                                input {
                                    class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300",
                                    r#type: "text",
                                    placeholder: "Demo",
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-slate-700 mb-1", "Last Name" }
                                input {
                                    class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300",
                                    r#type: "text",
                                    placeholder: "Realtor",
                                }
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-slate-700 mb-1", "Email" }
                            input {
                                class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300",
                                r#type: "email",
                                placeholder: "you@example.com",
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-slate-700 mb-1", "License Number" }
                            input {
                                class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300",
                                r#type: "text",
                                placeholder: "RE-12345678",
                            }
                        }
                    }
                    button {
                        class: "mt-4 bg-slate-800 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-slate-700 transition-colors",
                        "Save Profile"
                    }
                }

                // Database section
                div { class: "bg-white rounded-xl border border-slate-100 p-6",
                    h2 { class: "font-semibold text-slate-700 mb-1 flex items-center gap-2",
                        span { "🗄" }
                        "Database"
                    }
                    p { class: "text-slate-400 text-sm mb-4",
                        "Connected to Neon Postgres. Tables are auto-initialized on first run."
                    }
                    div { class: "bg-slate-50 rounded-lg p-3 font-mono text-xs text-slate-600",
                        "DATABASE_URL=postgresql://...@ep-*.neon.tech/neondb"
                    }
                    p { class: "text-slate-400 text-xs mt-2",
                        "Set the DATABASE_URL environment variable or .env file in project root."
                    }
                }

                // Tables section
                div { class: "bg-white rounded-xl border border-slate-100 p-6",
                    h2 { class: "font-semibold text-slate-700 mb-4 flex items-center gap-2",
                        span { "📋" }
                        "Database Tables"
                    }
                    div { class: "space-y-2",
                        for (table, desc) in [
                            ("clients", "Stores client profiles, budgets, and contact info"),
                            ("properties", "Stores property listings with price and details"),
                            ("appointments", "Stores scheduled viewings and meetings"),
                        ] {
                            div { class: "flex items-center justify-between py-2 border-b border-slate-50 last:border-0",
                                div {
                                    p { class: "text-sm font-medium text-slate-700 font-mono", "{table}" }
                                    p { class: "text-xs text-slate-400", "{desc}" }
                                }
                                span { class: "text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded-full", "active" }
                            }
                        }
                    }
                }
            }
        }
    }
}

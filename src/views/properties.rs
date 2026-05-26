use crate::models::property::{create_property, delete_property, get_properties, Property};
use dioxus::prelude::*;

#[component]
pub fn Properties() -> Element {
    let mut show_modal = use_signal(|| false);
    let mut properties = use_resource(move || async move { get_properties().await });

    // Form state
    let mut f_address = use_signal(String::new);
    let mut f_city = use_signal(String::new);
    let mut f_price = use_signal(String::new);
    let mut f_bedrooms = use_signal(String::new);
    let mut f_bathrooms = use_signal(String::new);
    let mut f_area = use_signal(String::new);
    let mut f_type = use_signal(|| "house".to_string());
    let mut f_desc = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error_msg = use_signal(String::new);

    let on_submit = move |_| {
        let address = f_address();
        let city = f_city();
        let price_str = f_price();

        if address.trim().is_empty() || city.trim().is_empty() || price_str.trim().is_empty() {
            error_msg.set("Address, city and price are required.".into());
            return;
        }
        let Ok(price) = price_str.trim().parse::<i64>() else {
            error_msg.set("Price must be a valid number.".into());
            return;
        };

        let bedrooms = f_bedrooms().trim().parse::<i32>().ok();
        let bathrooms = f_bathrooms().trim().parse::<i32>().ok();
        let area_sqft = f_area().trim().parse::<i32>().ok();

        submitting.set(true);
        error_msg.set(String::new());

        spawn(async move {
            match create_property(address, city, price, bedrooms, bathrooms, area_sqft, f_type(), f_desc()).await {
                Ok(_) => {
                    show_modal.set(false);
                    f_address.set(String::new());
                    f_city.set(String::new());
                    f_price.set(String::new());
                    f_bedrooms.set(String::new());
                    f_bathrooms.set(String::new());
                    f_area.set(String::new());
                    f_type.set("house".into());
                    f_desc.set(String::new());
                    properties.restart();
                }
                Err(e) => error_msg.set(e.to_string()),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "max-w-5xl mx-auto",
            div { class: "flex items-center justify-between mb-6",
                div {
                    h1 { class: "text-xl font-bold text-slate-800", "Properties" }
                    p { class: "text-slate-500 text-sm", "Browse and manage property listings" }
                }
                button {
                    class: "bg-slate-800 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-slate-700 transition-colors flex items-center gap-2",
                    onclick: move |_| show_modal.set(true),
                    span { "+" }
                    "Add Property"
                }
            }

            {
                match properties() {
                    None => rsx! {
                        div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                            for _ in 0..4 {
                                div { class: "bg-white rounded-xl border border-slate-100 p-5 h-36 animate-pulse" }
                            }
                        }
                    },
                    Some(Err(_)) => rsx! {
                        div { class: "bg-red-50 text-red-600 rounded-xl p-5 text-sm", "Failed to load properties." }
                    },
                    Some(Ok(list)) => {
                        if list.is_empty() {
                            rsx! {
                                div { class: "bg-white rounded-xl border border-slate-100 p-12 text-center",
                                    p { class: "text-4xl mb-3", "🏠" }
                                    p { class: "text-slate-600 font-medium", "No properties yet" }
                                    p { class: "text-slate-400 text-sm mt-1", "Add your first listing to get started." }
                                    button {
                                        class: "mt-4 bg-slate-800 text-white px-4 py-2 rounded-lg text-sm hover:bg-slate-700",
                                        onclick: move |_| show_modal.set(true),
                                        "Add Property"
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                                    for prop in list {
                                        PropertyCard {
                                            key: "{prop.id}",
                                            property: prop.clone(),
                                            on_delete: move |id: String| {
                                                spawn(async move {
                                                    let _ = delete_property(id).await;
                                                    properties.restart();
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

        // Add Property modal
        if show_modal() {
            div {
                class: "fixed inset-0 z-50 bg-black/40 flex justify-end",
                onclick: move |_| show_modal.set(false),
                div {
                    class: "w-96 h-full bg-white shadow-xl flex flex-col overflow-y-auto",
                    onclick: move |e| e.stop_propagation(),

                    div { class: "flex items-center justify-between px-6 py-5 border-b border-slate-100",
                        h2 { class: "font-semibold text-slate-800", "Add New Property" }
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
                        PropField { label: "Street Address *", placeholder: "123 Main St",
                            value: f_address(),
                            oninput: move |e: Event<FormData>| f_address.set(e.value()),
                        }
                        PropField { label: "City *", placeholder: "San Francisco",
                            value: f_city(),
                            oninput: move |e: Event<FormData>| f_city.set(e.value()),
                        }
                        PropField { label: "Price ($) *", placeholder: "750000",
                            value: f_price(),
                            oninput: move |e: Event<FormData>| f_price.set(e.value()),
                        }
                        div {
                            label { class: "block text-sm font-medium text-slate-700 mb-1", "Property Type" }
                            select {
                                class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300",
                                value: "{f_type}",
                                onchange: move |e| f_type.set(e.value()),
                                option { value: "house", "House" }
                                option { value: "apartment", "Apartment" }
                                option { value: "condo", "Condo" }
                                option { value: "land", "Land" }
                                option { value: "commercial", "Commercial" }
                            }
                        }
                        div { class: "grid grid-cols-3 gap-3",
                            PropField { label: "Beds", placeholder: "3",
                                value: f_bedrooms(),
                                oninput: move |e: Event<FormData>| f_bedrooms.set(e.value()),
                            }
                            PropField { label: "Baths", placeholder: "2",
                                value: f_bathrooms(),
                                oninput: move |e: Event<FormData>| f_bathrooms.set(e.value()),
                            }
                            PropField { label: "Sq Ft", placeholder: "1400",
                                value: f_area(),
                                oninput: move |e: Event<FormData>| f_area.set(e.value()),
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-slate-700 mb-1", "Description" }
                            textarea {
                                class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-slate-300 resize-none",
                                rows: "3",
                                placeholder: "Describe the property...",
                                value: "{f_desc}",
                                oninput: move |e| f_desc.set(e.value()),
                            }
                        }
                    }

                    div { class: "px-6 py-4 border-t border-slate-100 flex gap-3",
                        button {
                            class: "flex-1 bg-slate-800 text-white py-2.5 rounded-lg text-sm font-medium hover:bg-slate-700 transition-colors disabled:opacity-50",
                            disabled: submitting(),
                            onclick: on_submit,
                            if submitting() { "Saving..." } else { "Save Property" }
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
fn PropertyCard(property: Property, on_delete: EventHandler<String>) -> Element {
    let status_cls = match property.status.as_str() {
        "available" => "bg-green-100 text-green-700",
        "pending" => "bg-amber-100 text-amber-700",
        "sold" => "bg-slate-100 text-slate-500",
        _ => "bg-slate-100 text-slate-500",
    };

    let type_icon = match property.property_type.as_str() {
        "apartment" => "🏢",
        "condo" => "🏙",
        "land" => "🌿",
        "commercial" => "🏪",
        _ => "🏠",
    };

    rsx! {
        div {
            class: "bg-white rounded-xl border border-slate-100 p-5 hover:shadow-sm transition-shadow",
            div { class: "flex items-start justify-between mb-3",
                div { class: "flex items-center gap-2",
                    span { class: "text-xl", "{type_icon}" }
                    div {
                        p { class: "font-medium text-slate-800 text-sm leading-tight", "{property.address}" }
                        p { class: "text-slate-500 text-xs", "{property.city}" }
                    }
                }
                div { class: "flex items-center gap-2",
                    span { class: "text-xs px-2 py-0.5 rounded-full font-medium {status_cls}",
                        "{property.status}"
                    }
                    button {
                        class: "text-slate-300 hover:text-red-400 transition-colors",
                        onclick: {
                            let id = property.id.clone();
                            move |_| on_delete.call(id.clone())
                        },
                        "🗑"
                    }
                }
            }
            p { class: "text-lg font-bold text-slate-800 mb-2",
                "${format_price(property.price)}"
            }
            div { class: "flex items-center gap-4 text-xs text-slate-500",
                if let Some(beds) = property.bedrooms {
                    span { "🛏 {beds} bed" }
                }
                if let Some(baths) = property.bathrooms {
                    span { "🚿 {baths} bath" }
                }
                if let Some(area) = property.area_sqft {
                    span { "📐 {area} sqft" }
                }
            }
            p { class: "text-xs text-slate-400 mt-2", "Added {property.created_at}" }
        }
    }
}

#[component]
fn PropField(
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

fn format_price(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        let thousands = n / 1_000;
        let remainder = (n % 1_000) / 100;
        if remainder > 0 {
            format!("{},{:03}", thousands, n % 1_000)
        } else {
            format!("{}K", thousands)
        }
    } else {
        n.to_string()
    }
}

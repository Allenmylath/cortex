use dioxus::prelude::*;

use components::MainLayout;
use views::{Clients, Dashboard, Matches, Properties, Settings, VoicePage};

mod components;
mod models;
mod pipeline;
mod views;

#[cfg(not(target_arch = "wasm32"))]
mod db;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(MainLayout)]
        #[route("/")]
        Dashboard {},
        #[route("/clients")]
        Clients {},
        #[route("/properties")]
        Properties {},
        #[route("/matches")]
        Matches {},
        #[route("/settings")]
        Settings {},
    #[route("/voice")]
    VoicePage {},
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        dotenvy::dotenv().ok();
    }

    // Capture the main multi-threaded runtime handle before Dioxus launches.
    // Dioxus runs on_upgrade callbacks on a LocalPoolHandle (single-threaded),
    // so pipeline tasks must be explicitly spawned on this handle instead.
    #[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
    {
        let rt = Box::leak(Box::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio multi-thread runtime"),
        ));
        pipeline::MAIN_RT
            .set(rt.handle().clone())
            .expect("MAIN_RT already set");
    }

    dioxus::launch(App);
}

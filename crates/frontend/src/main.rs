mod api;
mod components;
mod coords;
mod pages;

use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/plan/:id")]
    PlanView { id: String },
}

#[component]
fn Home() -> Element {
    rsx! {
        pages::planner::Planner { plan_id: None::<String> }
    }
}

#[component]
fn PlanView(id: String) -> Element {
    rsx! {
        pages::planner::Planner { plan_id: Some(id) }
    }
}

const CSS: Asset = asset!("/assets/main.css");
const FAVICON: Asset = asset!("/assets/favicon.svg");

#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Stylesheet { href: CSS }
        Router::<Route> {}
    }
}

fn main() {
    launch(App);
}

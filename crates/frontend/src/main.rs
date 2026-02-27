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

fn App() -> Element {
    rsx! {
        document::Stylesheet { href: CSS }
        Router::<Route> {}
    }
}

fn main() {
    launch(App);
}

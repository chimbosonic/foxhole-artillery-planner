mod api;
mod components;
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

fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

fn main() {
    launch(App);
}

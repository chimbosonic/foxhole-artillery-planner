use dioxus::prelude::*;

#[component]
pub fn PlanPanel(
    plan_name: Signal<String>,
    plan_url: Signal<Option<String>>,
    on_save: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "panel",
            h3 { "Plan" }
            input {
                r#type: "text",
                placeholder: "Plan name...",
                value: "{plan_name}",
                oninput: move |evt: Event<FormData>| {
                    plan_name.set(evt.value().to_string());
                },
            }
            div { style: "margin-top: 8px;",
                button {
                    onclick: move |_| on_save.call(()),
                    "Save & Share"
                }
            }
            if let Some(url) = &*plan_url.read() {
                div { class: "plan-url",
                    input {
                        r#type: "text",
                        readonly: true,
                        value: "{url}",
                    }
                    button {
                        class: "secondary",
                        onclick: {
                            let url = url.clone();
                            move |_| {
                                let url = url.clone();
                                wasm_bindgen_futures::spawn_local(async move {
                                    if let Some(window) = web_sys::window() {
                                        let clipboard = window.navigator().clipboard();
                                        let _ = wasm_bindgen_futures::JsFuture::from(
                                            clipboard.write_text(&url)
                                        ).await;
                                    }
                                });
                            }
                        },
                        "Copy"
                    }
                }
            }
        }
    }
}

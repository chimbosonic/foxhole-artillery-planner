use dioxus::prelude::*;

#[component]
pub fn WindInput(
    wind_direction: Signal<Option<f64>>,
    wind_strength: Signal<u32>,
) -> Element {
    // Grid layout: NW N NE / W . E / SW S SE
    let grid_order: [(f64, &str); 8] = [
        (315.0, "NW"),
        (0.0, "N"),
        (45.0, "NE"),
        (270.0, "W"),
        // center gap
        (90.0, "E"),
        (225.0, "SW"),
        (180.0, "S"),
        (135.0, "SE"),
    ];

    let current_dir = *wind_direction.read();
    let current_str = *wind_strength.read();

    rsx! {
        div { class: "panel",
            h3 { "Wind" }
            div { class: "wind-grid",
                // First row: NW, N, NE
                for &(deg, label) in &grid_order[0..3] {
                    button {
                        class: if current_dir == Some(deg) { "active" } else { "" },
                        onclick: move |_| {
                            if current_dir == Some(deg) {
                                wind_direction.set(None);
                            } else {
                                wind_direction.set(Some(deg));
                            }
                        },
                        "{label}"
                    }
                }
                // Second row: W, center, E
                button {
                    class: if current_dir == Some(grid_order[3].0) { "active" } else { "" },
                    onclick: move |_| {
                        let deg = 270.0;
                        if current_dir == Some(deg) {
                            wind_direction.set(None);
                        } else {
                            wind_direction.set(Some(deg));
                        }
                    },
                    "{grid_order[3].1}"
                }
                div { class: "center", "+" }
                button {
                    class: if current_dir == Some(grid_order[4].0) { "active" } else { "" },
                    onclick: move |_| {
                        let deg = 90.0;
                        if current_dir == Some(deg) {
                            wind_direction.set(None);
                        } else {
                            wind_direction.set(Some(deg));
                        }
                    },
                    "{grid_order[4].1}"
                }
                // Third row: SW, S, SE
                for &(deg, label) in &grid_order[5..8] {
                    button {
                        class: if current_dir == Some(deg) { "active" } else { "" },
                        onclick: move |_| {
                            if current_dir == Some(deg) {
                                wind_direction.set(None);
                            } else {
                                wind_direction.set(Some(deg));
                            }
                        },
                        "{label}"
                    }
                }
            }
            div { class: "strength-row",
                label { "Strength:" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "5",
                    value: "{current_str}",
                    onchange: move |evt: Event<FormData>| {
                        if let Ok(v) = evt.value().parse::<u32>() {
                            wind_strength.set(v);
                        }
                    },
                }
                span { class: "value", "{current_str}" }
            }
        }
    }
}

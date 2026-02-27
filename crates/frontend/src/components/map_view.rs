use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlacementMode {
    Gun,
    Target,
    Spotter,
}

#[component]
pub fn MapView(
    map_file_name: String,
    placement_mode: Signal<PlacementMode>,
    gun_pos: Signal<Option<(f64, f64)>>,
    target_pos: Signal<Option<(f64, f64)>>,
    spotter_pos: Signal<Option<(f64, f64)>>,
) -> Element {
    let image_url = format!("/assets/images/maps/{}.webp", map_file_name);

    rsx! {
        div {
            class: "map-container",
            style: "width: 100%; height: 100%;",
            onclick: move |evt: Event<MouseData>| {
                let coords = evt.element_coordinates();
                let x = coords.x;
                let y = coords.y;
                match *placement_mode.read() {
                    PlacementMode::Gun => gun_pos.set(Some((x, y))),
                    PlacementMode::Target => target_pos.set(Some((x, y))),
                    PlacementMode::Spotter => spotter_pos.set(Some((x, y))),
                }
            },

            img {
                src: "{image_url}",
                draggable: "false",
            }

            // SVG overlay for lines between markers
            svg {
                class: "svg-overlay",
                // Line from gun to target
                if let (Some(gun), Some(target)) = (*gun_pos.read(), *target_pos.read()) {
                    line {
                        x1: "{gun.0}",
                        y1: "{gun.1}",
                        x2: "{target.0}",
                        y2: "{target.1}",
                        stroke: "rgba(233,69,96,0.6)",
                        stroke_width: "2",
                        stroke_dasharray: "6,4",
                    }
                }
            }

            // Gun marker
            if let Some(pos) = *gun_pos.read() {
                div {
                    class: "marker gun",
                    style: "left: {pos.0}px; top: {pos.1}px;",
                    div { class: "marker-label", "GUN" }
                }
            }

            // Target marker
            if let Some(pos) = *target_pos.read() {
                div {
                    class: "marker target",
                    style: "left: {pos.0}px; top: {pos.1}px;",
                    div { class: "marker-label", "TARGET" }
                }
            }

            // Spotter marker
            if let Some(pos) = *spotter_pos.read() {
                div {
                    class: "marker spotter",
                    style: "left: {pos.0}px; top: {pos.1}px;",
                    div { class: "marker-label", "SPOTTER" }
                }
            }
        }
    }
}

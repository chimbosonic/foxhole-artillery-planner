use dioxus::prelude::*;

#[component]
pub fn HelpOverlay(show: Signal<bool>) -> Element {
    if !*show.read() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "help-overlay-backdrop",
            onclick: move |_| show.set(false),

            div {
                class: "help-overlay",
                onclick: move |evt: Event<MouseData>| evt.stop_propagation(),

                h2 { "Help" }

                // --- Keyboard shortcuts ---

                div { class: "shortcut-section",
                    h3 { "Placement Modes" }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "1" } " / " kbd { "G" } }
                        span { "Gun mode" }
                    }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "2" } " / " kbd { "T" } }
                        span { "Target mode" }
                    }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "3" } " / " kbd { "S" } }
                        span { "Spotter mode" }
                    }
                }

                div { class: "shortcut-section",
                    h3 { "Actions" }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "Del" } " / " kbd { "Backspace" } }
                        span { "Remove selected marker" }
                    }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "Esc" } }
                        span { "Deselect / close help" }
                    }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "R" } }
                        span { "Reset zoom & pan" }
                    }
                }

                div { class: "shortcut-section",
                    h3 { "Undo / Redo" }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "Ctrl" } "+" kbd { "Z" } }
                        span { "Undo" }
                    }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "Ctrl" } "+" kbd { "Shift" } "+" kbd { "Z" } }
                        span { "Redo" }
                    }
                }

                div { class: "shortcut-section",
                    h3 { "Help" }
                    div { class: "shortcut-row",
                        span { class: "shortcut-keys", kbd { "H" } " / " kbd { "?" } }
                        span { "Toggle this help" }
                    }
                }

                // --- How calculations work ---

                div { class: "help-divider" }

                h2 { class: "help-section-title", "How Calculations Work" }

                div { class: "help-info-section",
                    h3 { "Azimuth" }
                    p { "The compass bearing from gun to target in degrees (0\u{00b0}\u{2013}360\u{00b0}). North is 0\u{00b0}, East is 90\u{00b0}, South is 180\u{00b0}, West is 270\u{00b0}. This is the direction you aim your artillery piece." }
                }

                div { class: "help-info-section",
                    h3 { "Distance" }
                    p { "Straight-line distance between gun and target in meters, rounded to the nearest 5m. Each weapon has a minimum and maximum range \u{2014} the status shows " span { class: "in-range-text", "IN RANGE" } " or " span { class: "out-of-range-text", "OUT OF RANGE" } " accordingly." }
                }

                div { class: "help-info-section",
                    h3 { "Accuracy" }
                    p { "The radius of the impact circle around the target, shown as \u{00b1}Xm. Accuracy worsens with distance \u{2014} it interpolates linearly from the weapon's best accuracy at minimum range to worst accuracy at maximum range." }
                }

                div { class: "help-info-section",
                    h3 { "Wind Compensation" }
                    p { "Wind pushes shells in the direction it blows toward (opposite of the \"from\" direction). Each wind strength level adds 8m of lateral drift. The planner compensates by adjusting the aim point: it shifts the target position against the wind and recalculates azimuth and distance to that corrected point." }
                }

                div { class: "help-info-section",
                    h3 { "Gun-Target Pairing" }
                    p { "Each gun is independently paired with a target. New guns auto-pair with the first unpaired target. Click a target while a gun is selected to manually pair them. Multiple guns can share the same target." }
                }

                div { class: "help-info-section",
                    h3 { "Map Interactions" }
                    p { "Left-click places markers or moves a selected one. Right-click removes the nearest marker. Scroll to zoom, drag to pan, double-click to reset view." }
                }

                button {
                    class: "close-help",
                    onclick: move |_| show.set(false),
                    "Close"
                }
            }
        }
    }
}

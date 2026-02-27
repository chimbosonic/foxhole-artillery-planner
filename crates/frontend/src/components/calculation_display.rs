use dioxus::prelude::*;

use crate::api::FiringSolutionData;
use crate::coords;

#[component]
pub fn CalculationDisplay(
    solution: Option<FiringSolutionData>,
    gun_pos: Option<(f64, f64)>,
    target_pos: Option<(f64, f64)>,
) -> Element {
    match solution {
        None => rsx! {
            div { class: "panel",
                h3 { "Firing Solution" }
                // Show grid coords even without a full solution
                if let Some(g) = gun_pos {
                    p { class: "coord-info",
                        "Gun: {coords::format_px_as_grid(g.0, g.1)}"
                    }
                }
                if let Some(t) = target_pos {
                    p { class: "coord-info",
                        "Target: {coords::format_px_as_grid(t.0, t.1)}"
                    }
                }
                if gun_pos.is_none() || target_pos.is_none() {
                    p { style: "color: var(--text-dim); font-size: 13px;",
                        "Place gun and target to calculate."
                    }
                } else {
                    p { style: "color: var(--text-dim); font-size: 13px;",
                        "Select a weapon to see firing solution."
                    }
                }
            }
        },
        Some(sol) => {
            let range_class = if sol.in_range { "value in-range" } else { "value out-of-range" };
            // Round distance to nearest 5m (artillery operates in 5m increments)
            let rounded_dist = (sol.distance / 5.0).round() * 5.0;
            rsx! {
                div { class: "panel",
                    h3 { "Firing Solution" }

                    // Grid coordinates
                    div { class: "coord-row",
                        if let Some(g) = gun_pos {
                            span { class: "coord-info gun-coord",
                                "Gun: {coords::format_px_as_grid(g.0, g.1)}"
                            }
                        }
                        if let Some(t) = target_pos {
                            span { class: "coord-info target-coord",
                                "Tgt: {coords::format_px_as_grid(t.0, t.1)}"
                            }
                        }
                    }

                    div { class: "solution",
                        div { class: "stat",
                            div { class: "label", "Azimuth" }
                            div { class: "{range_class}", "{sol.azimuth:.1}\u{00b0}" }
                        }
                        div { class: "stat",
                            div { class: "label", "Distance" }
                            div { class: "{range_class}", "{rounded_dist:.0}m" }
                        }
                        div { class: "stat",
                            div { class: "label", "Accuracy" }
                            div { class: "value", "\u{00b1}{sol.accuracy_radius:.1}m" }
                        }
                        div { class: "stat",
                            div { class: "label", "Status" }
                            div { class: "{range_class}",
                                if sol.in_range { "IN RANGE" } else { "OUT OF RANGE" }
                            }
                        }
                    }
                    if let (Some(adj_az), Some(adj_dist)) = (sol.wind_adjusted_azimuth, sol.wind_adjusted_distance) {
                        {
                            let rounded_adj = (adj_dist / 5.0).round() * 5.0;
                            rsx! {
                                div { class: "wind-adjusted",
                                    h4 { "Wind Adjusted" }
                                    div { class: "solution",
                                        div { class: "stat",
                                            div { class: "label", "Azimuth" }
                                            div { class: "value", "{adj_az:.1}\u{00b0}" }
                                        }
                                        div { class: "stat",
                                            div { class: "label", "Distance" }
                                            div { class: "value", "{rounded_adj:.0}m" }
                                        }
                                        if let Some(offset) = sol.wind_offset_meters {
                                            div { class: "stat full-width",
                                                div { class: "label", "Wind Drift" }
                                                div { class: "value", "{offset:.1}m" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

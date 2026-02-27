use dioxus::prelude::*;

use crate::api::FiringSolutionData;

#[component]
pub fn CalculationDisplay(solution: Option<FiringSolutionData>) -> Element {
    match solution {
        None => rsx! {
            div { class: "panel",
                h3 { "Firing Solution" }
                p { style: "color: var(--text-dim); font-size: 13px;",
                    "Place gun and target on the map to calculate."
                }
            }
        },
        Some(sol) => {
            let range_class = if sol.in_range { "value in-range" } else { "value out-of-range" };
            rsx! {
                div { class: "panel",
                    h3 { "Firing Solution" }
                    div { class: "solution",
                        div { class: "stat",
                            div { class: "label", "Azimuth" }
                            div { class: "{range_class}", "{sol.azimuth:.1}\u{00b0}" }
                        }
                        div { class: "stat",
                            div { class: "label", "Distance" }
                            div { class: "{range_class}", "{sol.distance:.0}m" }
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
                        div { class: "wind-adjusted",
                            h4 { "Wind Adjusted" }
                            div { class: "solution",
                                div { class: "stat",
                                    div { class: "label", "Azimuth" }
                                    div { class: "value", "{adj_az:.1}\u{00b0}" }
                                }
                                div { class: "stat",
                                    div { class: "label", "Distance" }
                                    div { class: "value", "{adj_dist:.0}m" }
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

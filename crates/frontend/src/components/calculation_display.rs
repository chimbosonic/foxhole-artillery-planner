use dioxus::prelude::*;

use crate::api::{FiringSolutionData, WeaponData};
use crate::coords;

#[component]
pub fn CalculationDisplay(
    solutions: Vec<Option<FiringSolutionData>>,
    gun_positions: Vec<(f64, f64)>,
    target_positions: Vec<(f64, f64)>,
    spotter_positions: Vec<(f64, f64)>,
    gun_weapon_ids: Vec<String>,
    weapons: Vec<WeaponData>,
) -> Element {
    let num_pairs = gun_positions.len().min(target_positions.len());
    let has_any_solution = solutions.iter().any(|s| s.is_some());
    let multiple_pairs = num_pairs > 1;

    // No solutions at all — show coordinate info and prompt
    if !has_any_solution {
        return rsx! {
            div { class: "panel",
                h3 { "Firing Solution" }
                // Show grid coords even without a full solution
                for (i, g) in gun_positions.iter().enumerate() {
                    p { class: "coord-info",
                        if gun_positions.len() > 1 {
                            "Gun {i + 1}: {coords::format_px_as_grid(g.0, g.1)}"
                        } else {
                            "Gun: {coords::format_px_as_grid(g.0, g.1)}"
                        }
                    }
                }
                for (i, t) in target_positions.iter().enumerate() {
                    p { class: "coord-info",
                        if target_positions.len() > 1 {
                            "Target {i + 1}: {coords::format_px_as_grid(t.0, t.1)}"
                        } else {
                            "Target: {coords::format_px_as_grid(t.0, t.1)}"
                        }
                    }
                }
                for (i, s) in spotter_positions.iter().enumerate() {
                    p { class: "coord-info",
                        if spotter_positions.len() > 1 {
                            "Spotter {i + 1}: {coords::format_px_as_grid(s.0, s.1)}"
                        } else {
                            "Spotter: {coords::format_px_as_grid(s.0, s.1)}"
                        }
                    }
                }
                if gun_positions.is_empty() || target_positions.is_empty() {
                    p { style: "color: var(--text-dim); font-size: 13px;",
                        "Place gun and target to calculate."
                    }
                } else {
                    p { style: "color: var(--text-dim); font-size: 13px;",
                        "Select a weapon to see firing solution."
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "panel",
            h3 { "Firing Solution" }

            for pair_idx in 0..num_pairs {
                {
                    let sol = solutions.get(pair_idx).and_then(|s| s.as_ref());
                    let gun = gun_positions.get(pair_idx);
                    let target = target_positions.get(pair_idx);

                    let weapon_name = gun_weapon_ids.get(pair_idx)
                        .and_then(|slug| weapons.iter().find(|w| w.slug == *slug))
                        .map(|w| w.display_name.clone());

                    rsx! {
                        if multiple_pairs {
                            h4 { style: "margin: 8px 0 4px; color: var(--text-dim);",
                                if let Some(ref wn) = weapon_name {
                                    "Pair {pair_idx + 1} — {wn}"
                                } else {
                                    "Pair {pair_idx + 1}"
                                }
                            }
                        } else if let Some(ref wn) = weapon_name {
                            p { style: "color: var(--text-dim); font-size: 12px; margin: 2px 0 4px;",
                                "{wn}"
                            }
                        }

                        // Grid coordinates
                        div { class: "coord-row",
                            if let Some(g) = gun {
                                span { class: "coord-info gun-coord",
                                    if multiple_pairs {
                                        "Gun {pair_idx + 1}: {coords::format_px_as_grid(g.0, g.1)}"
                                    } else {
                                        "Gun: {coords::format_px_as_grid(g.0, g.1)}"
                                    }
                                }
                            }
                            if let Some(t) = target {
                                span { class: "coord-info target-coord",
                                    if multiple_pairs {
                                        "Tgt {pair_idx + 1}: {coords::format_px_as_grid(t.0, t.1)}"
                                    } else {
                                        "Tgt: {coords::format_px_as_grid(t.0, t.1)}"
                                    }
                                }
                            }
                        }

                        if let Some(sol) = sol {
                            {
                                let range_class = if sol.in_range { "value in-range" } else { "value out-of-range" };
                                let rounded_dist = (sol.distance / 5.0).round() * 5.0;
                                rsx! {
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
            }

            // Unpaired guns (extra guns beyond targets)
            for i in num_pairs..gun_positions.len() {
                {
                    let g = &gun_positions[i];
                    rsx! {
                        p { class: "coord-info",
                            "Gun {i + 1}: {coords::format_px_as_grid(g.0, g.1)} (unpaired)"
                        }
                    }
                }
            }
            // Unpaired targets (extra targets beyond guns)
            for i in num_pairs..target_positions.len() {
                {
                    let t = &target_positions[i];
                    rsx! {
                        p { class: "coord-info",
                            "Target {i + 1}: {coords::format_px_as_grid(t.0, t.1)} (unpaired)"
                        }
                    }
                }
            }
            // Spotters (informational)
            for (i, s) in spotter_positions.iter().enumerate() {
                p { class: "coord-info spotter-coord",
                    if spotter_positions.len() > 1 {
                        "Spt {i + 1}: {coords::format_px_as_grid(s.0, s.1)}"
                    } else {
                        "Spt: {coords::format_px_as_grid(s.0, s.1)}"
                    }
                }
            }
        }
    }
}

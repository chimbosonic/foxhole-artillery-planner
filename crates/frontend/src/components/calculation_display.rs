use dioxus::prelude::*;

use crate::api::{FiringSolutionData, WeaponData};
use crate::components::map_view::{MarkerKind, SelectedMarker};
use crate::coords;

#[component]
pub fn CalculationDisplay(
    solutions: Vec<Option<FiringSolutionData>>,
    gun_positions: Vec<(f64, f64)>,
    target_positions: Vec<(f64, f64)>,
    spotter_positions: Vec<(f64, f64)>,
    gun_weapon_ids: Signal<Vec<String>>,
    gun_target_indices: Signal<Vec<Option<usize>>>,
    weapons: Vec<WeaponData>,
    selected_marker: Signal<Option<SelectedMarker>>,
    on_before_change: EventHandler<()>,
) -> Element {
    let has_any_solution = solutions.iter().any(|s| s.is_some());
    let cur_selected = *selected_marker.read();
    let wids = gun_weapon_ids.read().clone();
    let pairings = gun_target_indices.read().clone();
    let multiple_guns = gun_positions.len() > 1;

    let colonial: Vec<&WeaponData> = weapons.iter().filter(|w| w.faction == "COLONIAL" || w.faction == "BOTH").collect();
    let warden: Vec<&WeaponData> = weapons.iter().filter(|w| w.faction == "WARDEN" || w.faction == "BOTH").collect();

    // Find which targets are assigned to at least one gun
    let assigned_targets: Vec<bool> = (0..target_positions.len())
        .map(|ti| pairings.contains(&Some(ti)))
        .collect();

    // No solutions at all — show coordinate info and prompt
    if !has_any_solution && gun_positions.is_empty() && target_positions.is_empty() {
        return rsx! {
            div { class: "panel",
                h3 { "Firing Solution" }
                p { style: "color: var(--text-dim); font-size: 13px;",
                    "Place gun and target to calculate."
                }
            }
        };
    }

    rsx! {
        div { class: "panel",
            h3 { "Firing Solution" }

            // Each gun with its assigned target and firing solution
            for (gun_idx, g) in gun_positions.iter().enumerate() {
                {
                    let sol = solutions.get(gun_idx).and_then(|s| s.as_ref());
                    let target_idx = pairings.get(gun_idx).and_then(|o| *o);
                    let target = target_idx.and_then(|ti| target_positions.get(ti));

                    let weapon_name = wids.get(gun_idx)
                        .and_then(|slug| weapons.iter().find(|w| w.slug == *slug))
                        .map(|w| w.display_name.clone());

                    let gun_selected = cur_selected == Some(SelectedMarker { kind: MarkerKind::Gun, index: gun_idx });
                    let paired_tgt_selected = target_idx
                        .map(|ti| cur_selected == Some(SelectedMarker { kind: MarkerKind::Target, index: ti }))
                        .unwrap_or(false);

                    rsx! {
                        if multiple_guns {
                            h4 { style: "margin: 8px 0 4px; color: var(--text-dim);",
                                if let Some(ref wn) = weapon_name {
                                    "Gun {gun_idx + 1} — {wn}"
                                } else {
                                    "Gun {gun_idx + 1}"
                                }
                            }
                        } else if let Some(ref wn) = weapon_name {
                            p { style: "color: var(--text-dim); font-size: 12px; margin: 2px 0 4px;",
                                "{wn}"
                            }
                        }

                        // Grid coordinates — clickable
                        div { class: "coord-row",
                            {
                                let cls = if gun_selected { "marker-item selected" } else { "marker-item" };
                                rsx! {
                                    div {
                                        class: "{cls}",
                                        onclick: {
                                            let sel = if gun_selected { None } else { Some(SelectedMarker { kind: MarkerKind::Gun, index: gun_idx }) };
                                            move |_| selected_marker.set(sel)
                                        },
                                        span { class: "coord-info gun-coord",
                                            if multiple_guns {
                                                "Gun {gun_idx + 1}: {coords::format_px_as_grid(g.0, g.1)}"
                                            } else {
                                                "Gun: {coords::format_px_as_grid(g.0, g.1)}"
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(t) = target {
                                {
                                    let ti = target_idx.unwrap();
                                    let cls = if paired_tgt_selected { "marker-item selected" } else { "marker-item" };
                                    rsx! {
                                        div {
                                            class: "{cls}",
                                            onclick: {
                                                let sel = if paired_tgt_selected { None } else { Some(SelectedMarker { kind: MarkerKind::Target, index: ti }) };
                                                move |_| selected_marker.set(sel)
                                            },
                                            span { class: "coord-info target-coord",
                                                if target_positions.len() > 1 {
                                                    "Tgt {ti + 1}: {coords::format_px_as_grid(t.0, t.1)}"
                                                } else {
                                                    "Tgt: {coords::format_px_as_grid(t.0, t.1)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                span { class: "coord-info", style: "color: var(--text-dim); font-style: italic;",
                                    "(no target)"
                                }
                            }
                        }

                        // Inline selectors when gun is selected
                        if gun_selected {
                            {
                                let current_slug = wids.get(gun_idx).cloned().unwrap_or_default();
                                let current_target_val = match pairings.get(gun_idx).and_then(|o| *o) {
                                    Some(ti) => format!("{}", ti),
                                    None => String::new(),
                                };
                                rsx! {
                                    // Weapon selector
                                    select {
                                        class: "inline-weapon-select",
                                        value: "{current_slug}",
                                        onchange: {
                                            let idx = gun_idx;
                                            move |evt: Event<FormData>| {
                                                on_before_change.call(());
                                                let new_slug = evt.value().to_string();
                                                if let Some(entry) = gun_weapon_ids.write().get_mut(idx) {
                                                    *entry = new_slug;
                                                }
                                            }
                                        },
                                        option { value: "", "-- Select Weapon --" }
                                        optgroup { label: "Colonial",
                                            for w in &colonial {
                                                option {
                                                    value: "{w.slug}",
                                                    selected: current_slug == w.slug,
                                                    "{w.display_name} ({w.min_range}-{w.max_range}m)"
                                                }
                                            }
                                        }
                                        optgroup { label: "Warden",
                                            for w in &warden {
                                                option {
                                                    value: "{w.slug}",
                                                    selected: current_slug == w.slug,
                                                    "{w.display_name} ({w.min_range}-{w.max_range}m)"
                                                }
                                            }
                                        }
                                    }
                                    // Target selector
                                    select {
                                        class: "inline-weapon-select",
                                        value: "{current_target_val}",
                                        onchange: {
                                            let idx = gun_idx;
                                            move |evt: Event<FormData>| {
                                                on_before_change.call(());
                                                let val = evt.value().to_string();
                                                let new_target = if val.is_empty() {
                                                    None
                                                } else {
                                                    val.parse::<usize>().ok()
                                                };
                                                if let Some(entry) = gun_target_indices.write().get_mut(idx) {
                                                    *entry = new_target;
                                                }
                                            }
                                        },
                                        option { value: "", "-- No Target --" }
                                        for (ti, tp) in target_positions.iter().enumerate() {
                                            option {
                                                value: "{ti}",
                                                selected: current_target_val == format!("{}", ti),
                                                if target_positions.len() > 1 {
                                                    "Target {ti + 1}: {coords::format_px_as_grid(tp.0, tp.1)}"
                                                } else {
                                                    "Target: {coords::format_px_as_grid(tp.0, tp.1)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Firing solution
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

            // Unassigned targets (not paired with any gun)
            for (ti, t) in target_positions.iter().enumerate() {
                if !assigned_targets.get(ti).copied().unwrap_or(false) {
                    {
                        let is_selected = cur_selected == Some(SelectedMarker { kind: MarkerKind::Target, index: ti });
                        let cls = if is_selected { "marker-item selected" } else { "marker-item" };
                        rsx! {
                            div {
                                class: "{cls}",
                                onclick: {
                                    let sel = if is_selected { None } else { Some(SelectedMarker { kind: MarkerKind::Target, index: ti }) };
                                    move |_| selected_marker.set(sel)
                                },
                                p { class: "coord-info",
                                    if target_positions.len() > 1 {
                                        "Target {ti + 1}: {coords::format_px_as_grid(t.0, t.1)} (unassigned)"
                                    } else {
                                        "Target: {coords::format_px_as_grid(t.0, t.1)} (unassigned)"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Spotters (informational) — clickable
            for (i, s) in spotter_positions.iter().enumerate() {
                {
                    let is_selected = cur_selected == Some(SelectedMarker { kind: MarkerKind::Spotter, index: i });
                    let cls = if is_selected { "marker-item selected" } else { "marker-item" };
                    rsx! {
                        div {
                            class: "{cls}",
                            onclick: {
                                let sel = if is_selected { None } else { Some(SelectedMarker { kind: MarkerKind::Spotter, index: i }) };
                                move |_| selected_marker.set(sel)
                            },
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

            // Prompt when nothing placed yet
            if gun_positions.is_empty() && target_positions.is_empty() {
                p { style: "color: var(--text-dim); font-size: 13px;",
                    "Place gun and target to calculate."
                }
            }
        }
    }
}

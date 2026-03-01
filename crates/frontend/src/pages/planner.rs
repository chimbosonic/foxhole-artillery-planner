use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;

use crate::api::{self, FiringSolutionData, MapData, WeaponData};
use crate::components::calculation_display::CalculationDisplay;
use crate::components::help_overlay::HelpOverlay;
use crate::components::map_view::{remove_marker, Faction, MapView, PlacementMode, SelectedMarker};
use crate::components::plan_panel::PlanPanel;
use crate::components::weapon_selector::WeaponSelector;
use crate::components::wind_input::WindInput;
use crate::coords;

// ---------------------------------------------------------------------------
// Undo / redo infrastructure
// ---------------------------------------------------------------------------

const UNDO_LIMIT: usize = 50;

#[derive(Clone, Debug)]
pub struct PlanSnapshot {
    pub gun_positions: Vec<(f64, f64)>,
    pub target_positions: Vec<(f64, f64)>,
    pub spotter_positions: Vec<(f64, f64)>,
    pub gun_weapon_ids: Vec<String>,
    pub gun_target_indices: Vec<Option<usize>>,
    pub wind_direction: Option<f64>,
    pub wind_strength: u32,
}

pub fn capture_snapshot(
    gun_positions: &Signal<Vec<(f64, f64)>>,
    target_positions: &Signal<Vec<(f64, f64)>>,
    spotter_positions: &Signal<Vec<(f64, f64)>>,
    gun_weapon_ids: &Signal<Vec<String>>,
    gun_target_indices: &Signal<Vec<Option<usize>>>,
    wind_direction: &Signal<Option<f64>>,
    wind_strength: &Signal<u32>,
) -> PlanSnapshot {
    PlanSnapshot {
        gun_positions: gun_positions.read().clone(),
        target_positions: target_positions.read().clone(),
        spotter_positions: spotter_positions.read().clone(),
        gun_weapon_ids: gun_weapon_ids.read().clone(),
        gun_target_indices: gun_target_indices.read().clone(),
        wind_direction: *wind_direction.read(),
        wind_strength: *wind_strength.read(),
    }
}

#[allow(clippy::too_many_arguments)]
fn restore_snapshot(
    snapshot: &PlanSnapshot,
    gun_positions: &mut Signal<Vec<(f64, f64)>>,
    target_positions: &mut Signal<Vec<(f64, f64)>>,
    spotter_positions: &mut Signal<Vec<(f64, f64)>>,
    gun_weapon_ids: &mut Signal<Vec<String>>,
    gun_target_indices: &mut Signal<Vec<Option<usize>>>,
    wind_direction: &mut Signal<Option<f64>>,
    wind_strength: &mut Signal<u32>,
) {
    gun_positions.set(snapshot.gun_positions.clone());
    target_positions.set(snapshot.target_positions.clone());
    spotter_positions.set(snapshot.spotter_positions.clone());
    gun_weapon_ids.set(snapshot.gun_weapon_ids.clone());
    gun_target_indices.set(snapshot.gun_target_indices.clone());
    wind_direction.set(snapshot.wind_direction);
    wind_strength.set(snapshot.wind_strength);
}

pub fn push_undo(
    undo_stack: &mut Signal<Vec<PlanSnapshot>>,
    redo_stack: &mut Signal<Vec<PlanSnapshot>>,
    snapshot: PlanSnapshot,
) {
    let mut stack = undo_stack.write();
    stack.push(snapshot);
    if stack.len() > UNDO_LIMIT {
        stack.remove(0);
    }
    drop(stack);
    redo_stack.write().clear();
}

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------

fn is_input_focused() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(doc) = window.document() else {
        return false;
    };
    let Some(active) = doc.active_element() else {
        return false;
    };
    let tag = active.tag_name().to_uppercase();
    matches!(tag.as_str(), "INPUT" | "SELECT" | "TEXTAREA")
}

fn load_saved_faction() -> Faction {
    let storage: Option<web_sys::Storage> = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten());
    storage
        .and_then(|s| s.get_item("faction").ok().flatten())
        .map(|v| {
            if v == "colonial" {
                Faction::Colonial
            } else {
                Faction::Warden
            }
        })
        .unwrap_or(Faction::Warden)
}

fn save_faction(faction: Faction) {
    let storage: Option<web_sys::Storage> = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten());
    if let Some(storage) = storage {
        let val = match faction {
            Faction::Warden => "warden",
            Faction::Colonial => "colonial",
        };
        let _ = storage.set_item("faction", val);
    }
}

#[component]
pub fn Planner(plan_id: Option<String>) -> Element {
    // Data resources
    let maps_resource = use_resource(api::fetch_maps);
    let weapons_resource = use_resource(api::fetch_weapons);

    // UI state signals — positions are in native map-image pixel space (2048x1776)
    let mut selected_map = use_signal(String::new);
    let mut selected_weapon = use_signal(String::new);
    let mut placement_mode = use_signal(|| PlacementMode::Gun);
    let mut gun_positions = use_signal(Vec::<(f64, f64)>::new);
    let mut target_positions = use_signal(Vec::<(f64, f64)>::new);
    let mut spotter_positions = use_signal(Vec::<(f64, f64)>::new);
    let mut wind_direction = use_signal(|| None::<f64>);
    let mut wind_strength = use_signal(|| 0u32);
    let mut gun_weapon_ids = use_signal(Vec::<String>::new);
    let mut gun_target_indices = use_signal(Vec::<Option<usize>>::new);
    let mut selected_marker = use_signal(|| None::<SelectedMarker>);
    let mut plan_name = use_signal(|| "New Plan".to_string());
    let mut plan_url = use_signal(|| None::<String>);
    let mut firing_solutions = use_signal(Vec::<Option<FiringSolutionData>>::new);

    // Undo / redo stacks
    let mut undo_stack = use_signal(Vec::<PlanSnapshot>::new);
    let mut redo_stack = use_signal(Vec::<PlanSnapshot>::new);

    // Help overlay and view-reset signaling
    let mut show_help = use_signal(|| false);
    let mut reset_view_counter = use_signal(|| 0u64);

    // Faction theme
    let mut faction = use_signal(load_saved_faction);

    // Auto-focus .app div on mount so keyboard shortcuts work immediately
    use_effect(|| {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
            if let Some(doc) = window.document() {
                if let Some(el) = doc.query_selector(".app").ok().flatten() {
                    if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                        let _ = html_el.focus();
                    }
                }
            }
        }
    });

    // Load plan if we have an ID
    let _plan_loader = use_resource(move || {
        let plan_id = plan_id.clone();
        async move {
            if let Some(id) = plan_id {
                if let Ok(Some(plan)) = api::fetch_plan(&id).await {
                    selected_map.set(plan.map_id);
                    if let Some(first) = plan.weapon_ids.first() {
                        selected_weapon.set(first.clone());
                    }
                    let num_guns = plan.gun_positions.len();
                    let num_targets = plan.target_positions.len();
                    gun_weapon_ids.set(plan.weapon_ids);
                    plan_name.set(plan.name);
                    // Plan stores meter coordinates, convert to image pixels
                    gun_positions.set(
                        plan.gun_positions
                            .iter()
                            .map(|p| coords::meters_to_map_px(p.x, p.y))
                            .collect(),
                    );
                    target_positions.set(
                        plan.target_positions
                            .iter()
                            .map(|p| coords::meters_to_map_px(p.x, p.y))
                            .collect(),
                    );
                    spotter_positions.set(
                        plan.spotter_positions
                            .iter()
                            .map(|p| coords::meters_to_map_px(p.x, p.y))
                            .collect(),
                    );
                    // Load explicit pairings, or fall back to index-based for old plans
                    if plan.gun_target_indices.is_empty() {
                        // Legacy plan: pair by index
                        let indices: Vec<Option<usize>> = (0..num_guns)
                            .map(|i| if i < num_targets { Some(i) } else { None })
                            .collect();
                        gun_target_indices.set(indices);
                    } else {
                        gun_target_indices.set(
                            plan.gun_target_indices
                                .iter()
                                .map(|o| o.map(|v| v as usize))
                                .collect(),
                        );
                    }
                    if let Some(dir) = plan.wind_direction {
                        wind_direction.set(Some(dir));
                    }
                    wind_strength.set(plan.wind_strength);
                }
            }
        }
    });

    // Auto-calculate when inputs change — use explicit pairings
    let _calc_effect = use_resource(move || {
        let gun_wids = gun_weapon_ids.read().clone();
        let guns = gun_positions.read().clone();
        let targets = target_positions.read().clone();
        let pairings = gun_target_indices.read().clone();
        let w_dir = *wind_direction.read();
        let w_str = *wind_strength.read();
        async move {
            if guns.is_empty() {
                firing_solutions.set(vec![]);
                return;
            }
            let mut results = Vec::with_capacity(guns.len());
            for (i, g_px) in guns.iter().enumerate() {
                let wid = gun_wids.get(i).cloned().unwrap_or_default();
                let target_idx = pairings.get(i).and_then(|o| *o);
                let t_px = target_idx.and_then(|ti| targets.get(ti));
                if wid.is_empty() || t_px.is_none() {
                    results.push(None);
                    continue;
                }
                let t_px = t_px.unwrap();
                let (gx, gy) = coords::map_px_to_meters(g_px.0, g_px.1);
                let (tx, ty) = coords::map_px_to_meters(t_px.0, t_px.1);
                match api::calculate(gx, gy, tx, ty, &wid, w_dir, Some(w_str)).await {
                    Ok(sol) => results.push(Some(sol)),
                    Err(_) => results.push(None),
                }
            }
            firing_solutions.set(results);
        }
    });

    // Wait for initial data
    let maps: Vec<MapData> = match &*maps_resource.read() {
        Some(Ok(m)) => m.clone(),
        _ => vec![],
    };
    let weapons: Vec<WeaponData> = match &*weapons_resource.read() {
        Some(Ok(w)) => w.clone(),
        _ => vec![],
    };

    // Set default map if none selected
    if selected_map.read().is_empty() && !maps.is_empty() {
        selected_map.set(maps[0].file_name.clone());
    }

    let current_map = selected_map.read().clone();

    // Compute accuracy radii in image pixels for the map overlay (one per gun, using pairings)
    let accuracy_radii_px = use_memo(move || {
        firing_solutions
            .read()
            .iter()
            .map(|sol| {
                sol.as_ref()
                    .map(|s| coords::meters_to_image_px(s.accuracy_radius))
            })
            .collect::<Vec<_>>()
    });

    // Closure to push undo snapshot from planner-level code
    let mut push_snapshot = move || {
        let snap = capture_snapshot(
            &gun_positions,
            &target_positions,
            &spotter_positions,
            &gun_weapon_ids,
            &gun_target_indices,
            &wind_direction,
            &wind_strength,
        );
        push_undo(&mut undo_stack, &mut redo_stack, snap);
    };

    let app_class = if *faction.read() == Faction::Colonial {
        "app colonial"
    } else {
        "app"
    };

    rsx! {
        div {
            class: "{app_class}",
            tabindex: "0",

            onkeydown: move |evt: Event<KeyboardData>| {
                let key = evt.key();
                let mods = evt.data().modifiers();
                let ctrl_or_cmd = mods.contains(Modifiers::CONTROL) || mods.contains(Modifiers::META);
                let shift = mods.contains(Modifiers::SHIFT);

                // Skip most shortcuts when typing in an input
                if is_input_focused() {
                    if key == Key::Escape {
                        // Blur the focused element
                        if let Some(w) = web_sys::window() {
                            if let Some(doc) = w.document() {
                                if let Some(active) = doc.active_element() {
                                    use wasm_bindgen::JsCast;
                                    if let Some(el) = active.dyn_ref::<web_sys::HtmlElement>() {
                                        let _ = el.blur();
                                    }
                                }
                            }
                        }
                    }
                    return;
                }

                match &key {
                    // Undo: Ctrl+Z / Cmd+Z (without Shift)
                    Key::Character(c) if c == "z" && ctrl_or_cmd && !shift => {
                        evt.prevent_default();
                        if let Some(snap) = undo_stack.write().pop() {
                            let current = capture_snapshot(
                                &gun_positions, &target_positions, &spotter_positions,
                                &gun_weapon_ids, &gun_target_indices,
                                &wind_direction, &wind_strength,
                            );
                            redo_stack.write().push(current);
                            restore_snapshot(
                                &snap,
                                &mut gun_positions, &mut target_positions, &mut spotter_positions,
                                &mut gun_weapon_ids, &mut gun_target_indices,
                                &mut wind_direction, &mut wind_strength,
                            );
                            selected_marker.set(None);
                        }
                    }
                    // Redo: Ctrl+Shift+Z / Cmd+Shift+Z
                    Key::Character(c) if (c == "Z" || c == "z") && ctrl_or_cmd && shift => {
                        evt.prevent_default();
                        if let Some(snap) = redo_stack.write().pop() {
                            let current = capture_snapshot(
                                &gun_positions, &target_positions, &spotter_positions,
                                &gun_weapon_ids, &gun_target_indices,
                                &wind_direction, &wind_strength,
                            );
                            undo_stack.write().push(current);
                            restore_snapshot(
                                &snap,
                                &mut gun_positions, &mut target_positions, &mut spotter_positions,
                                &mut gun_weapon_ids, &mut gun_target_indices,
                                &mut wind_direction, &mut wind_strength,
                            );
                            selected_marker.set(None);
                        }
                    }
                    // Placement modes
                    Key::Character(c) if c == "1" || c == "g" => {
                        placement_mode.set(PlacementMode::Gun);
                    }
                    Key::Character(c) if c == "2" || c == "t" => {
                        placement_mode.set(PlacementMode::Target);
                    }
                    Key::Character(c) if c == "3" || c == "s" => {
                        placement_mode.set(PlacementMode::Spotter);
                    }
                    // Help overlay
                    Key::Character(c) if c == "h" || c == "?" => {
                        let current = *show_help.read();
                        show_help.set(!current);
                    }
                    // Delete selected marker
                    Key::Delete | Key::Backspace => {
                        let cur_sel = *selected_marker.read();
                        if let Some(sm) = cur_sel {
                            push_snapshot();
                            remove_marker(
                                sm.kind, sm.index,
                                &mut gun_positions, &mut target_positions, &mut spotter_positions,
                                &mut gun_weapon_ids, &mut gun_target_indices,
                            );
                            selected_marker.set(None);
                        }
                    }
                    // Reset zoom/pan
                    Key::Character(c) if c == "r" => {
                        let current = *reset_view_counter.read();
                        reset_view_counter.set(current + 1);
                    }
                    // Escape: close help or deselect
                    Key::Escape => {
                        if *show_help.read() {
                            show_help.set(false);
                        } else {
                            selected_marker.set(None);
                        }
                    }
                    _ => {}
                }
            },

            // Header
            div { class: "header",
                h1 { "Foxhole Artillery Planner" }
                div { class: "placement-mode",
                    button {
                        class: if *placement_mode.read() == PlacementMode::Gun { "active-gun" } else { "" },
                        onclick: move |_| placement_mode.set(PlacementMode::Gun),
                        "Gun"
                    }
                    button {
                        class: if *placement_mode.read() == PlacementMode::Target { "active-target" } else { "" },
                        onclick: move |_| placement_mode.set(PlacementMode::Target),
                        "Target"
                    }
                    button {
                        class: if *placement_mode.read() == PlacementMode::Spotter { "active-spotter" } else { "" },
                        onclick: move |_| placement_mode.set(PlacementMode::Spotter),
                        "Spotter"
                    }
                }
                div { class: "header-right",
                    div { class: "faction-toggle",
                        button {
                            class: if *faction.read() == Faction::Warden { "active" } else { "" },
                            onclick: move |_| {
                                faction.set(Faction::Warden);
                                save_faction(Faction::Warden);
                            },
                            "Warden"
                        }
                        button {
                            class: if *faction.read() == Faction::Colonial { "active" } else { "" },
                            onclick: move |_| {
                                faction.set(Faction::Colonial);
                                save_faction(Faction::Colonial);
                            },
                            "Colonial"
                        }
                    }
                }
            }

            // Sidebar
            div { class: "sidebar",
                // Map selector
                div { class: "panel",
                    h3 { "Map" }
                    select {
                        value: "{selected_map}",
                        onchange: move |evt: Event<FormData>| {
                            push_snapshot();
                            selected_map.set(evt.value().to_string());
                            gun_positions.set(vec![]);
                            target_positions.set(vec![]);
                            spotter_positions.set(vec![]);
                            gun_weapon_ids.set(vec![]);
                            gun_target_indices.set(vec![]);
                            selected_marker.set(None);
                        },
                        for m in &maps {
                            option {
                                value: "{m.file_name}",
                                selected: *selected_map.read() == m.file_name,
                                "{m.display_name}"
                            }
                        }
                    }
                }

                WeaponSelector {
                    weapons: weapons.clone(),
                    selected_weapon: selected_weapon,
                }

                WindInput {
                    wind_direction: wind_direction,
                    wind_strength: wind_strength,
                    on_before_change: move |_| push_snapshot(),
                }

                CalculationDisplay {
                    solutions: firing_solutions.read().clone(),
                    gun_positions: gun_positions.read().clone(),
                    target_positions: target_positions.read().clone(),
                    spotter_positions: spotter_positions.read().clone(),
                    gun_weapon_ids: gun_weapon_ids,
                    gun_target_indices: gun_target_indices,
                    weapons: weapons.clone(),
                    selected_marker: selected_marker,
                    on_before_change: move |_| push_snapshot(),
                }

                PlanPanel {
                    plan_name: plan_name,
                    plan_url: plan_url,
                    on_save: move |_| {
                        let map = selected_map.read().clone();
                        let wids = gun_weapon_ids.read().clone();
                        let name = plan_name.read().clone();
                        let guns = gun_positions.read().clone();
                        let targets = target_positions.read().clone();
                        let spotters = spotter_positions.read().clone();
                        let pairings = gun_target_indices.read().clone();
                        let w_dir = *wind_direction.read();
                        let w_str = *wind_strength.read();
                        spawn(async move {
                            // Convert pixel positions to meters for storage
                            let gun_m: Vec<(f64, f64)> = guns.iter()
                                .map(|g| coords::map_px_to_meters(g.0, g.1))
                                .collect();
                            let tgt_m: Vec<(f64, f64)> = targets.iter()
                                .map(|t| coords::map_px_to_meters(t.0, t.1))
                                .collect();
                            let spt_m: Vec<(f64, f64)> = spotters.iter()
                                .map(|s| coords::map_px_to_meters(s.0, s.1))
                                .collect();
                            match api::create_plan(
                                &name, &map, &wids,
                                &gun_m, &tgt_m, &spt_m,
                                &pairings,
                                w_dir, Some(w_str),
                            ).await {
                                Ok(plan) => {
                                    let window = web_sys::window().unwrap();
                                    let origin = window.location().origin().unwrap();
                                    plan_url.set(Some(api::build_plan_url(&origin, &plan.id)));
                                }
                                Err(e) => {
                                    web_sys::window()
                                        .unwrap()
                                        .alert_with_message(&format!("Failed to save: {}", e))
                                        .ok();
                                }
                            }
                        });
                    },
                }

                div { class: "panel",
                    h3 { "Help & Info" }
                    p { style: "font-size: 12px; color: var(--text-dim); margin-bottom: 8px;",
                        "View keyboard shortcuts and learn how firing calculations work."
                    }
                    button {
                        style: "width: 100%;",
                        onclick: move |_| show_help.set(true),
                        "Open Help"
                    }
                }

                div { class: "panel about",
                    h3 { "About" }
                    p { "Foxhole Artillery Planner — a tool for planning artillery operations in Foxhole." }
                    p {
                        "Built by "
                        a {
                            href: "https://keyoxide.org/alexis.lowe%40chimbosonic.com",
                            target: "_blank",
                            "Alexis Lowe aka Chimbosonic"
                        }
                        "."
                    }
                    p {
                        "Map assets by "
                        a {
                            href: "https://rustard.itch.io/improved-map-mod",
                            target: "_blank",
                            "Rustard's Improved Map Mod"
                        }
                        "."
                    }
                }
            }

            // Map view
            if !current_map.is_empty() {
                MapView {
                    key: "{current_map}",
                    map_file_name: current_map,
                    placement_mode: placement_mode,
                    gun_positions: gun_positions,
                    target_positions: target_positions,
                    spotter_positions: spotter_positions,
                    gun_weapon_ids: gun_weapon_ids,
                    gun_target_indices: gun_target_indices,
                    selected_weapon_slug: selected_weapon,
                    weapons: weapons.clone(),
                    accuracy_radii_px: accuracy_radii_px,
                    selected_marker: selected_marker,
                    undo_stack: undo_stack,
                    redo_stack: redo_stack,
                    wind_direction: wind_direction,
                    wind_strength: wind_strength,
                    reset_view_counter: reset_view_counter,
                    faction: faction,
                }
            }

            HelpOverlay { show: show_help }
        }
    }
}

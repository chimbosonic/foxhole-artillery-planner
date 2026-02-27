use dioxus::prelude::*;

use crate::api::{self, FiringSolutionData, MapData, WeaponData};
use crate::components::calculation_display::CalculationDisplay;
use crate::components::map_view::{MapView, PlacementMode, SelectedMarker};
use crate::components::plan_panel::PlanPanel;
use crate::components::weapon_selector::WeaponSelector;
use crate::components::wind_input::WindInput;
use crate::coords;

#[component]
pub fn Planner(plan_id: Option<String>) -> Element {
    // Data resources
    let maps_resource = use_resource(api::fetch_maps);
    let weapons_resource = use_resource(api::fetch_weapons);

    // UI state signals — positions are in native map-image pixel space (1024x888)
    let mut selected_map = use_signal(String::new);
    let mut selected_weapon = use_signal(String::new);
    let mut placement_mode = use_signal(|| PlacementMode::Gun);
    let mut gun_positions = use_signal(Vec::<(f64, f64)>::new);
    let mut target_positions = use_signal(Vec::<(f64, f64)>::new);
    let mut spotter_positions = use_signal(Vec::<(f64, f64)>::new);
    let mut wind_direction = use_signal(|| None::<f64>);
    let mut wind_strength = use_signal(|| 0u32);
    let mut gun_weapon_ids = use_signal(Vec::<String>::new);
    let mut selected_marker = use_signal(|| None::<SelectedMarker>);
    let mut plan_name = use_signal(|| "New Plan".to_string());
    let mut plan_url = use_signal(|| None::<String>);
    let mut firing_solutions = use_signal(Vec::<Option<FiringSolutionData>>::new);

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
                    gun_weapon_ids.set(plan.weapon_ids);
                    plan_name.set(plan.name);
                    // Plan stores meter coordinates, convert to image pixels
                    gun_positions.set(
                        plan.gun_positions.iter().map(|p| coords::meters_to_map_px(p.x, p.y)).collect()
                    );
                    target_positions.set(
                        plan.target_positions.iter().map(|p| coords::meters_to_map_px(p.x, p.y)).collect()
                    );
                    spotter_positions.set(
                        plan.spotter_positions.iter().map(|p| coords::meters_to_map_px(p.x, p.y)).collect()
                    );
                    if let Some(dir) = plan.wind_direction {
                        wind_direction.set(Some(dir));
                    }
                    wind_strength.set(plan.wind_strength);
                }
            }
        }
    });

    // Auto-calculate when inputs change — convert pixel positions to meters for the API
    let _calc_effect = use_resource(move || {
        let gun_wids = gun_weapon_ids.read().clone();
        let guns = gun_positions.read().clone();
        let targets = target_positions.read().clone();
        let w_dir = *wind_direction.read();
        let w_str = *wind_strength.read();
        async move {
            if guns.is_empty() || targets.is_empty() {
                firing_solutions.set(vec![]);
                return;
            }
            let mut results = Vec::with_capacity(guns.len().min(targets.len()));
            for (i, (g_px, t_px)) in guns.iter().zip(targets.iter()).enumerate() {
                let wid = gun_wids.get(i).cloned().unwrap_or_default();
                if wid.is_empty() {
                    results.push(None);
                    continue;
                }
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

    // Compute accuracy radii in image pixels for the map overlay (one per pair)
    let accuracy_radii_px: Vec<Option<f64>> = firing_solutions.read().iter().map(|sol| {
        sol.as_ref().map(|s| coords::meters_to_image_px(s.accuracy_radius))
    }).collect();

    rsx! {
        div { class: "app",
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
            }

            // Sidebar
            div { class: "sidebar",
                // Map selector
                div { class: "panel",
                    h3 { "Map" }
                    select {
                        value: "{selected_map}",
                        onchange: move |evt: Event<FormData>| {
                            selected_map.set(evt.value().to_string());
                            gun_positions.set(vec![]);
                            target_positions.set(vec![]);
                            spotter_positions.set(vec![]);
                            gun_weapon_ids.set(vec![]);
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
                }

                CalculationDisplay {
                    solutions: firing_solutions.read().clone(),
                    gun_positions: gun_positions.read().clone(),
                    target_positions: target_positions.read().clone(),
                    spotter_positions: spotter_positions.read().clone(),
                    gun_weapon_ids: gun_weapon_ids,
                    weapons: weapons.clone(),
                    selected_marker: selected_marker,
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
                    selected_weapon_slug: selected_weapon,
                    weapons: weapons.clone(),
                    accuracy_radii_px: accuracy_radii_px,
                    selected_marker: selected_marker,
                }
            }
        }
    }
}

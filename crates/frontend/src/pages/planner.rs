use dioxus::prelude::*;

use crate::api::{self, FiringSolutionData, MapData, WeaponData};
use crate::components::calculation_display::CalculationDisplay;
use crate::components::map_view::{MapView, PlacementMode};
use crate::components::plan_panel::PlanPanel;
use crate::components::weapon_selector::WeaponSelector;
use crate::components::wind_input::WindInput;
use crate::coords;

#[component]
pub fn Planner(plan_id: Option<String>) -> Element {
    // Data resources
    let maps_resource = use_resource(|| api::fetch_maps());
    let weapons_resource = use_resource(|| api::fetch_weapons());

    // UI state signals — positions are in native map-image pixel space (1024x888)
    let mut selected_map = use_signal(|| String::new());
    let mut selected_weapon = use_signal(|| String::new());
    let mut placement_mode = use_signal(|| PlacementMode::Gun);
    let mut gun_pos = use_signal(|| None::<(f64, f64)>);
    let mut target_pos = use_signal(|| None::<(f64, f64)>);
    let mut spotter_pos = use_signal(|| None::<(f64, f64)>);
    let mut wind_direction = use_signal(|| None::<f64>);
    let mut wind_strength = use_signal(|| 0u32);
    let mut plan_name = use_signal(|| "New Plan".to_string());
    let mut plan_url = use_signal(|| None::<String>);
    let mut firing_solution = use_signal(|| None::<FiringSolutionData>);

    // Load plan if we have an ID
    let _plan_loader = use_resource(move || {
        let plan_id = plan_id.clone();
        async move {
            if let Some(id) = plan_id {
                if let Ok(Some(plan)) = api::fetch_plan(&id).await {
                    selected_map.set(plan.map_id);
                    selected_weapon.set(plan.weapon_id);
                    plan_name.set(plan.name);
                    // Plan stores meter coordinates, convert to image pixels
                    if let (Some(mx), Some(my)) = (plan.gun_x, plan.gun_y) {
                        let (px, py) = coords::meters_to_map_px(mx, my);
                        gun_pos.set(Some((px, py)));
                    }
                    if let (Some(mx), Some(my)) = (plan.target_x, plan.target_y) {
                        let (px, py) = coords::meters_to_map_px(mx, my);
                        target_pos.set(Some((px, py)));
                    }
                    if let (Some(mx), Some(my)) = (plan.spotter_x, plan.spotter_y) {
                        let (px, py) = coords::meters_to_map_px(mx, my);
                        spotter_pos.set(Some((px, py)));
                    }
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
        let weapon = selected_weapon.read().clone();
        let gun = *gun_pos.read();
        let target = *target_pos.read();
        let w_dir = *wind_direction.read();
        let w_str = *wind_strength.read();
        async move {
            if let (Some(g_px), Some(t_px)) = (gun, target) {
                if !weapon.is_empty() {
                    let (gx, gy) = coords::map_px_to_meters(g_px.0, g_px.1);
                    let (tx, ty) = coords::map_px_to_meters(t_px.0, t_px.1);
                    match api::calculate(gx, gy, tx, ty, &weapon, w_dir, Some(w_str)).await {
                        Ok(sol) => firing_solution.set(Some(sol)),
                        Err(_) => firing_solution.set(None),
                    }
                }
            } else {
                firing_solution.set(None);
            }
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
    let current_weapon_slug = selected_weapon.read().clone();

    // Find the currently selected weapon data for range visualization
    let current_weapon_data = weapons.iter().find(|w| w.slug == current_weapon_slug).cloned();

    // Compute accuracy radius in image pixels for the map overlay
    let accuracy_radius_px = firing_solution.read().as_ref().map(|sol| {
        coords::meters_to_image_px(sol.accuracy_radius)
    });

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
                            gun_pos.set(None);
                            target_pos.set(None);
                            spotter_pos.set(None);
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
                    solution: firing_solution.read().clone(),
                    gun_pos: *gun_pos.read(),
                    target_pos: *target_pos.read(),
                }

                PlanPanel {
                    plan_name: plan_name,
                    plan_url: plan_url,
                    on_save: move |_| {
                        let map = selected_map.read().clone();
                        let weapon = selected_weapon.read().clone();
                        let name = plan_name.read().clone();
                        let gun = *gun_pos.read();
                        let target = *target_pos.read();
                        let w_dir = *wind_direction.read();
                        let w_str = *wind_strength.read();
                        spawn(async move {
                            // Convert pixel positions to meters for storage
                            let (gun_mx, gun_my) = match gun {
                                Some(g) => {
                                    let (mx, my) = coords::map_px_to_meters(g.0, g.1);
                                    (Some(mx), Some(my))
                                }
                                None => (None, None),
                            };
                            let (tgt_mx, tgt_my) = match target {
                                Some(t) => {
                                    let (mx, my) = coords::map_px_to_meters(t.0, t.1);
                                    (Some(mx), Some(my))
                                }
                                None => (None, None),
                            };
                            match api::create_plan(
                                &name, &map, &weapon,
                                gun_mx, gun_my, tgt_mx, tgt_my,
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
            }

            // Map view
            if !current_map.is_empty() {
                MapView {
                    map_file_name: current_map,
                    placement_mode: placement_mode,
                    gun_pos: gun_pos,
                    target_pos: target_pos,
                    spotter_pos: spotter_pos,
                    selected_weapon: current_weapon_data,
                    accuracy_radius_px: accuracy_radius_px,
                }
            }
        }
    }
}

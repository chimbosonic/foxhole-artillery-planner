use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use foxhole_shared::grid;

use crate::api::WeaponData;
use crate::coords;

const MAP_CONTAINER_ID: &str = "artillery-map-container";

/// Drag threshold in pixels — movement below this is treated as a click.
const DRAG_THRESHOLD: f64 = 3.0;

const ZOOM_MIN: f64 = 1.0;
const ZOOM_MAX: f64 = 10.0;
const ZOOM_STEP: f64 = 1.1;

/// Distance threshold (in map-image pixels, before zoom) for right-click removal.
const REMOVE_THRESHOLD: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlacementMode {
    Gun,
    Target,
    Spotter,
}

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------

/// Get the bounding client rect of the map container element.
fn container_rect() -> Option<web_sys::DomRect> {
    let document = web_sys::window()?.document()?;
    let element = document.get_element_by_id(MAP_CONTAINER_ID)?;
    Some(element.get_bounding_client_rect())
}

// ---------------------------------------------------------------------------
// Zoom / pan math (pure functions, easily testable)
// ---------------------------------------------------------------------------

/// Compute new pan offsets so that `cursor` stays over the same content point
/// when zooming from `old_zoom` to `new_zoom`.
fn zoom_pan_at_cursor(
    cursor_x: f64,
    cursor_y: f64,
    old_zoom: f64,
    new_zoom: f64,
    old_pan_x: f64,
    old_pan_y: f64,
) -> (f64, f64) {
    let content_x = (cursor_x - old_pan_x) / old_zoom;
    let content_y = (cursor_y - old_pan_y) / old_zoom;
    (
        cursor_x - content_x * new_zoom,
        cursor_y - content_y * new_zoom,
    )
}

/// Clamp pan values so the map can't be dragged off-screen.
fn clamp_pan(
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    container_w: f64,
    container_h: f64,
) -> (f64, f64) {
    let min_pan_x = -(container_w * zoom - container_w);
    let min_pan_y = -(container_h * zoom - container_h);
    (pan_x.clamp(min_pan_x, 0.0), pan_y.clamp(min_pan_y, 0.0))
}

/// Apply `clamp_pan` using the live container dimensions.
fn clamp_pan_to_container(pan_x: f64, pan_y: f64, zoom: f64) -> (f64, f64) {
    match container_rect() {
        Some(rect) => clamp_pan(pan_x, pan_y, zoom, rect.width(), rect.height()),
        None => (pan_x, pan_y),
    }
}

/// Convert a wheel delta (pixels / lines / pages) to a uniform pixel-like value.
fn wheel_delta_y(delta: WheelDelta) -> f64 {
    match delta {
        WheelDelta::Pixels(d) => d.y,
        WheelDelta::Lines(d) => d.y * 40.0,
        WheelDelta::Pages(d) => d.y * 400.0,
    }
}

// ---------------------------------------------------------------------------
// Marker removal helper
// ---------------------------------------------------------------------------

/// Euclidean distance between two points.
fn dist(a: &(f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

/// Find the index of the nearest position within `threshold` (Euclidean distance).
fn find_nearest(positions: &[(f64, f64)], click: (f64, f64), threshold: f64) -> Option<usize> {
    let mut best_idx = None;
    let mut best_dist = threshold;
    for (i, pos) in positions.iter().enumerate() {
        let dx = pos.0 - click.0;
        let dy = pos.1 - click.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < best_dist {
            best_dist = dist;
            best_idx = Some(i);
        }
    }
    best_idx
}

// ---------------------------------------------------------------------------
// SVG builder
// ---------------------------------------------------------------------------

/// Build the full SVG content as a string for reliable rendering.
/// Positions are in native map-image pixel space (1024×888).
fn build_svg_content(
    guns: &[(f64, f64)],
    targets: &[(f64, f64)],
    spotters: &[(f64, f64)],
    gun_weapons: &[Option<&WeaponData>],
    accuracy_radii_px: &[Option<f64>],
    zoom: f64,
) -> String {
    let mut svg = String::with_capacity(8192);

    build_grid_lines(&mut svg);
    build_grid_labels(&mut svg);
    if zoom >= 3.0 {
        build_keypad_lines(&mut svg);
        build_keypad_labels(&mut svg);
    }
    build_range_circles(&mut svg, guns, gun_weapons, zoom);
    build_firing_lines(&mut svg, guns, targets, zoom);
    build_accuracy_circles(&mut svg, targets, accuracy_radii_px, zoom);
    build_gun_markers(&mut svg, guns, zoom);
    build_target_markers(&mut svg, targets, zoom);
    build_spotter_markers(&mut svg, spotters, zoom);

    svg
}

fn build_grid_lines(svg: &mut String) {
    for col in 0..=grid::GRID_COLS {
        let x = grid::grid_col_px(col);
        svg.push_str(&format!(
            r#"<line x1="{x}" y1="0" x2="{x}" y2="{}" stroke="rgba(255,255,255,0.15)" stroke-width="0.5"/>"#,
            grid::MAP_HEIGHT_PX
        ));
    }
    for row in 0..=grid::GRID_ROWS {
        let y = grid::grid_row_px(row);
        svg.push_str(&format!(
            r#"<line x1="0" y1="{y}" x2="{}" y2="{y}" stroke="rgba(255,255,255,0.15)" stroke-width="0.5"/>"#,
            grid::MAP_WIDTH_PX
        ));
    }
}

fn build_grid_labels(svg: &mut String) {
    let col_step = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    for col in 0..grid::GRID_COLS {
        let x = col as f64 * col_step + col_step / 2.0;
        let letter = grid::col_letter(col);
        svg.push_str(&format!(
            r#"<text x="{x}" y="12" fill="rgba(255,255,255,0.45)" font-size="9" font-family="monospace" font-weight="600" text-anchor="middle" dominant-baseline="central">{letter}</text>"#
        ));
    }
    let row_step = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    for row in 0..grid::GRID_ROWS {
        let y = row as f64 * row_step + row_step / 2.0 + 4.0;
        let num = row + 1;
        svg.push_str(&format!(
            r#"<text x="4" y="{y}" fill="rgba(255,255,255,0.45)" font-size="9" font-family="monospace" font-weight="600" text-anchor="start" dominant-baseline="central">{num}</text>"#
        ));
    }
}

fn build_keypad_lines(svg: &mut String) {
    let cell_w = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    let cell_h = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    let third_w = cell_w / 3.0;
    let third_h = cell_h / 3.0;

    for col in 0..grid::GRID_COLS {
        let x0 = grid::grid_col_px(col);
        for i in 1..3 {
            let x = x0 + third_w * i as f64;
            svg.push_str(&format!(
                r#"<line x1="{x}" y1="0" x2="{x}" y2="{}" stroke="rgba(255,255,255,0.08)" stroke-width="0.3"/>"#,
                grid::MAP_HEIGHT_PX
            ));
        }
    }
    for row in 0..grid::GRID_ROWS {
        let y0 = grid::grid_row_px(row);
        for i in 1..3 {
            let y = y0 + third_h * i as f64;
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y}" x2="{}" y2="{y}" stroke="rgba(255,255,255,0.08)" stroke-width="0.3"/>"#,
                grid::MAP_WIDTH_PX
            ));
        }
    }
}

fn build_keypad_labels(svg: &mut String) {
    let cell_w = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    let cell_h = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    let third_w = cell_w / 3.0;
    let third_h = cell_h / 3.0;

    // Numpad layout: row 0 (top) = 7 8 9, row 1 (mid) = 4 5 6, row 2 (bot) = 1 2 3
    const KEYPAD: [[u8; 3]; 3] = [[7, 8, 9], [4, 5, 6], [1, 2, 3]];

    for col in 0..grid::GRID_COLS {
        let x0 = grid::grid_col_px(col);
        for row in 0..grid::GRID_ROWS {
            let y0 = grid::grid_row_px(row);
            for (kr, keypad_row) in KEYPAD.iter().enumerate() {
                for (kc, &label) in keypad_row.iter().enumerate() {
                    let cx = x0 + third_w * kc as f64 + third_w / 2.0;
                    let cy = y0 + third_h * kr as f64 + third_h / 2.0;
                    svg.push_str(&format!(
                        r#"<text x="{cx}" y="{cy}" fill="rgba(255,255,255,0.2)" font-size="5" font-family="monospace" text-anchor="middle" dominant-baseline="central">{label}</text>"#
                    ));
                }
            }
        }
    }
}

fn build_range_circles(
    svg: &mut String,
    guns: &[(f64, f64)],
    gun_weapons: &[Option<&WeaponData>],
    zoom: f64,
) {
    for (i, &(gx, gy)) in guns.iter().enumerate() {
        let Some(w) = gun_weapons.get(i).and_then(|o| *o) else { continue };
        let s = 1.0 / zoom.min(5.0);
        let max_r = coords::meters_to_image_px(w.max_range);
        let sw1 = 1.5 * s;
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{max_r}" fill="rgba(78,204,163,0.06)" stroke="#4ecca3" stroke-width="{sw1}" stroke-opacity="0.6"/>"##
        ));
        let min_r = coords::meters_to_image_px(w.min_range);
        let sw2 = 1.0 * s;
        let da1 = 4.0 * s;
        let da2 = 3.0 * s;
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{min_r}" fill="rgba(233,69,96,0.06)" stroke="#e94560" stroke-width="{sw2}" stroke-dasharray="{da1} {da2}" stroke-opacity="0.5"/>"##
        ));
    }
}

fn build_firing_lines(
    svg: &mut String,
    guns: &[(f64, f64)],
    targets: &[(f64, f64)],
    zoom: f64,
) {
    for ((gx, gy), (tx, ty)) in guns.iter().zip(targets.iter()) {
        let s = 1.0 / zoom.min(5.0);
        let sw = 1.5 * s;
        let da1 = 6.0 * s;
        let da2 = 4.0 * s;
        svg.push_str(&format!(
            r#"<line x1="{gx}" y1="{gy}" x2="{tx}" y2="{ty}" stroke="rgba(233,69,96,0.7)" stroke-width="{sw}" stroke-dasharray="{da1} {da2}"/>"#
        ));
    }
}

fn build_accuracy_circles(
    svg: &mut String,
    targets: &[(f64, f64)],
    accuracy_radii_px: &[Option<f64>],
    zoom: f64,
) {
    for ((tx, ty), acc_r) in targets.iter().zip(accuracy_radii_px.iter()) {
        if let Some(acc_r) = acc_r {
            let s = 1.0 / zoom.min(5.0);
            let sw = 1.0 * s;
            let da1 = 3.0 * s;
            let da2 = 2.0 * s;
            svg.push_str(&format!(
                r##"<circle cx="{tx}" cy="{ty}" r="{acc_r}" fill="rgba(233,69,96,0.15)" stroke="#e94560" stroke-width="{sw}" stroke-dasharray="{da1} {da2}"/>"##
            ));
        }
    }
}

/// Generate marker label: no number suffix for single markers, numbered for multiple.
fn marker_label(base: &str, index: usize, total: usize) -> String {
    if total <= 1 {
        base.to_string()
    } else {
        format!("{} {}", base, index + 1)
    }
}

fn build_gun_markers(svg: &mut String, guns: &[(f64, f64)], zoom: f64) {
    let total = guns.len();
    for (i, &(gx, gy)) in guns.iter().enumerate() {
        let s = 1.0 / zoom.min(5.0);
        let r = 6.0 * s;
        let sw = 1.5 * s;
        let fs = 8.0 * s;
        let ty = gy - 10.0 * s;
        let tsw = 2.0 * s;
        let label = marker_label("GUN", i, total);
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{r}" fill="#4ecca3" stroke="white" stroke-width="{sw}"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{gx}" y="{ty}" fill="white" font-size="{fs}" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="{tsw}" paint-order="stroke">{label}</text>"##
        ));
    }
}

fn build_target_markers(svg: &mut String, targets: &[(f64, f64)], zoom: f64) {
    let total = targets.len();
    for (i, &(tx, ty)) in targets.iter().enumerate() {
        let s = 1.0 / zoom.min(5.0);
        let arm = 8.0 * s;
        let sw = 1.5 * s;
        let r = 4.0 * s;
        let fs = 8.0 * s;
        let label_y = ty - 12.0 * s;
        let tsw = 2.0 * s;
        let label = marker_label("TARGET", i, total);
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{ty}" x2="{}" y2="{ty}" stroke="#e94560" stroke-width="{sw}"/>"##,
            tx - arm,
            tx + arm
        ));
        svg.push_str(&format!(
            r##"<line x1="{tx}" y1="{}" x2="{tx}" y2="{}" stroke="#e94560" stroke-width="{sw}"/>"##,
            ty - arm,
            ty + arm
        ));
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="{r}" fill="#e94560" stroke="white" stroke-width="{sw}"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{tx}" y="{label_y}" fill="#ffcccc" font-size="{fs}" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="{tsw}" paint-order="stroke">{label}</text>"##
        ));
    }
}

fn build_spotter_markers(svg: &mut String, spotters: &[(f64, f64)], zoom: f64) {
    let total = spotters.len();
    for (i, &(sx, sy)) in spotters.iter().enumerate() {
        let s = 1.0 / zoom.min(5.0);
        let r = 5.0 * s;
        let sw = 1.5 * s;
        let fs = 8.0 * s;
        let label_y = sy - 10.0 * s;
        let tsw = 2.0 * s;
        let label = marker_label("SPOTTER", i, total);
        svg.push_str(&format!(
            r##"<circle cx="{sx}" cy="{sy}" r="{r}" fill="#7ec8e3" stroke="white" stroke-width="{sw}"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{sx}" y="{label_y}" fill="#cce7ff" font-size="{fs}" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="{tsw}" paint-order="stroke">{label}</text>"##
        ));
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn MapView(
    map_file_name: String,
    placement_mode: Signal<PlacementMode>,
    gun_positions: Signal<Vec<(f64, f64)>>,
    target_positions: Signal<Vec<(f64, f64)>>,
    spotter_positions: Signal<Vec<(f64, f64)>>,
    gun_weapon_ids: Signal<Vec<String>>,
    selected_weapon_slug: Signal<String>,
    weapons: Vec<WeaponData>,
    accuracy_radii_px: Vec<Option<f64>>,
) -> Element {
    let image_url = format!("/static/images/maps/{}.webp", map_file_name);

    // Zoom / pan state (local — resets when component is re-created via `key`)
    let mut zoom = use_signal(|| 1.0_f64);
    let mut pan_x = use_signal(|| 0.0_f64);
    let mut pan_y = use_signal(|| 0.0_f64);

    // Drag state
    let mut is_dragging = use_signal(|| false);
    let mut did_drag = use_signal(|| false);
    let mut drag_start_x = use_signal(|| 0.0_f64);
    let mut drag_start_y = use_signal(|| 0.0_f64);
    let mut drag_start_pan_x = use_signal(|| 0.0_f64);
    let mut drag_start_pan_y = use_signal(|| 0.0_f64);

    // Read marker positions for SVG rendering
    let guns = gun_positions.read().clone();
    let targets = target_positions.read().clone();
    let spotters = spotter_positions.read().clone();
    let wids = gun_weapon_ids.read().clone();

    // Resolve per-gun weapon data
    let gun_weapons: Vec<Option<&WeaponData>> = wids.iter().map(|slug| {
        weapons.iter().find(|w| w.slug == *slug)
    }).collect();

    // Snapshot current transform for the render
    let cur_zoom = *zoom.read();

    let svg_content = build_svg_content(&guns, &targets, &spotters, &gun_weapons, &accuracy_radii_px, cur_zoom);
    let svg_html = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none" style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:5;">{}</svg>"#,
        grid::MAP_WIDTH_PX, grid::MAP_HEIGHT_PX, svg_content
    );
    let cur_pan_x = *pan_x.read();
    let cur_pan_y = *pan_y.read();
    let dragging = *is_dragging.read();

    let transform_style = format!(
        "transform: translate({cur_pan_x}px, {cur_pan_y}px) scale({cur_zoom}); transform-origin: 0 0;"
    );
    let container_class = if dragging {
        "map-container dragging"
    } else {
        "map-container"
    };

    // Build coord readout tags
    let gun_tags: Vec<(String, String)> = guns.iter().enumerate().map(|(i, pos)| {
        let label = if guns.len() <= 1 { "GUN".to_string() } else { format!("GUN {}", i + 1) };
        (label, coords::format_px_as_grid(pos.0, pos.1))
    }).collect();
    let target_tags: Vec<(String, String)> = targets.iter().enumerate().map(|(i, pos)| {
        let label = if targets.len() <= 1 { "TGT".to_string() } else { format!("TGT {}", i + 1) };
        (label, coords::format_px_as_grid(pos.0, pos.1))
    }).collect();
    let spotter_tags: Vec<(String, String)> = spotters.iter().enumerate().map(|(i, pos)| {
        let label = if spotters.len() <= 1 { "SPT".to_string() } else { format!("SPT {}", i + 1) };
        (label, coords::format_px_as_grid(pos.0, pos.1))
    }).collect();

    rsx! {
        div {
            id: MAP_CONTAINER_ID,
            class: "{container_class}",

            onwheel: move |evt: Event<WheelData>| {
                evt.prevent_default();

                let delta_y = wheel_delta_y(evt.data().delta());
                let factor = if delta_y < 0.0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
                let old_z = *zoom.read();
                let new_z = (old_z * factor).clamp(ZOOM_MIN, ZOOM_MAX);
                if (new_z - old_z).abs() < 1e-9 {
                    return;
                }

                let Some(rect) = container_rect() else { return };
                let client = evt.data().client_coordinates();
                let cx = client.x - rect.left();
                let cy = client.y - rect.top();

                let (new_px, new_py) =
                    zoom_pan_at_cursor(cx, cy, old_z, new_z, *pan_x.read(), *pan_y.read());
                let (px, py) = clamp_pan(new_px, new_py, new_z, rect.width(), rect.height());

                zoom.set(new_z);
                pan_x.set(px);
                pan_y.set(py);
            },

            onmousedown: move |evt: Event<MouseData>| {
                // Only track drag/click for left mouse button
                if evt.trigger_button() != Some(MouseButton::Primary) {
                    return;
                }
                let client = evt.client_coordinates();
                is_dragging.set(true);
                did_drag.set(false);
                drag_start_x.set(client.x);
                drag_start_y.set(client.y);
                drag_start_pan_x.set(*pan_x.read());
                drag_start_pan_y.set(*pan_y.read());
            },

            onmousemove: move |evt: Event<MouseData>| {
                if !*is_dragging.read() {
                    return;
                }
                let client = evt.client_coordinates();
                let dx = client.x - *drag_start_x.read();
                let dy = client.y - *drag_start_y.read();

                if !*did_drag.read() && (dx.abs() > DRAG_THRESHOLD || dy.abs() > DRAG_THRESHOLD) {
                    did_drag.set(true);
                }
                if *did_drag.read() {
                    let new_px = *drag_start_pan_x.read() + dx;
                    let new_py = *drag_start_pan_y.read() + dy;
                    let (px, py) = clamp_pan_to_container(new_px, new_py, *zoom.read());
                    pan_x.set(px);
                    pan_y.set(py);
                }
            },

            onmouseup: move |evt: Event<MouseData>| {
                let was_dragging = *is_dragging.read();
                let was_drag = *did_drag.read();
                is_dragging.set(false);

                // A mouseup without drag movement = a click → append marker
                if was_dragging && !was_drag {
                    let client = evt.client_coordinates();
                    if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                        client.x, client.y, MAP_CONTAINER_ID,
                        *zoom.read(), *pan_x.read(), *pan_y.read(),
                    ) {
                        let mode = *placement_mode.read();
                        match mode {
                            PlacementMode::Gun => {
                                gun_positions.write().push((img_x, img_y));
                                gun_weapon_ids.write().push(selected_weapon_slug.read().clone());
                            }
                            PlacementMode::Target => target_positions.write().push((img_x, img_y)),
                            PlacementMode::Spotter => spotter_positions.write().push((img_x, img_y)),
                        }
                        // Auto-cycle: Gun → Target → Gun for easy pairing
                        match mode {
                            PlacementMode::Gun => placement_mode.set(PlacementMode::Target),
                            PlacementMode::Target => placement_mode.set(PlacementMode::Gun),
                            PlacementMode::Spotter => {} // stay in spotter mode
                        }
                    }
                }
            },

            oncontextmenu: move |evt: Event<MouseData>| {
                evt.prevent_default();
                let client = evt.client_coordinates();
                if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                    client.x, client.y, MAP_CONTAINER_ID,
                    *zoom.read(), *pan_x.read(), *pan_y.read(),
                ) {
                    let threshold = REMOVE_THRESHOLD / (*zoom.read()).min(5.0);
                    let click = (img_x, img_y);

                    // Clone positions to avoid borrow conflicts with Signal read/write
                    let guns_snap = gun_positions.read().clone();
                    let targets_snap = target_positions.read().clone();
                    let spotters_snap = spotter_positions.read().clone();

                    // Check active placement mode's list first for priority
                    let mode = *placement_mode.read();
                    let removed = match mode {
                        PlacementMode::Gun => {
                            if let Some(idx) = find_nearest(&guns_snap, click, threshold) {
                                gun_positions.write().remove(idx);
                                gun_weapon_ids.write().remove(idx);
                                true
                            } else { false }
                        }
                        PlacementMode::Target => {
                            if let Some(idx) = find_nearest(&targets_snap, click, threshold) {
                                target_positions.write().remove(idx);
                                true
                            } else { false }
                        }
                        PlacementMode::Spotter => {
                            if let Some(idx) = find_nearest(&spotters_snap, click, threshold) {
                                spotter_positions.write().remove(idx);
                                true
                            } else { false }
                        }
                    };

                    // If nothing found in the active mode's list, check all lists
                    if !removed {
                        let gun_hit = find_nearest(&guns_snap, click, threshold)
                            .map(|idx| (idx, dist(&guns_snap[idx], click), 0u8));
                        let tgt_hit = find_nearest(&targets_snap, click, threshold)
                            .map(|idx| (idx, dist(&targets_snap[idx], click), 1u8));
                        let spt_hit = find_nearest(&spotters_snap, click, threshold)
                            .map(|idx| (idx, dist(&spotters_snap[idx], click), 2u8));

                        let nearest = [gun_hit, tgt_hit, spt_hit]
                            .into_iter()
                            .flatten()
                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                        if let Some((idx, _, kind)) = nearest {
                            match kind {
                                0 => {
                                    gun_positions.write().remove(idx);
                                    gun_weapon_ids.write().remove(idx);
                                }
                                1 => { target_positions.write().remove(idx); }
                                _ => { spotter_positions.write().remove(idx); }
                            }
                        }
                    }
                }
            },

            onmouseleave: move |_| {
                is_dragging.set(false);
            },

            ondoubleclick: move |evt: Event<MouseData>| {
                evt.prevent_default();
                zoom.set(1.0);
                pan_x.set(0.0);
                pan_y.set(0.0);
            },

            // Inner wrapper — CSS transform applies zoom/pan to map + overlay together
            div {
                class: "map-inner",
                style: "{transform_style}",

                img { src: "{image_url}", draggable: "false" }

                div {
                    dangerous_inner_html: "{svg_html}",
                    style: "position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;",
                }
            }

            // Coordinate readout (outside the transform so it stays fixed)
            div { class: "coord-readout",
                for (label, coord) in gun_tags {
                    span { class: "coord-tag gun-tag",
                        "{label}: {coord}"
                    }
                }
                for (label, coord) in target_tags {
                    span { class: "coord-tag target-tag",
                        "{label}: {coord}"
                    }
                }
                for (label, coord) in spotter_tags {
                    span { class: "coord-tag spotter-tag",
                        "{label}: {coord}"
                    }
                }
            }
        }
    }
}

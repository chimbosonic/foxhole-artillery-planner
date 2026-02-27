use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::keyboard_types::Key;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkerKind {
    Gun,
    Target,
    Spotter,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedMarker {
    pub kind: MarkerKind,
    pub index: usize,
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
///
/// The map image is rendered at `width: 100%` of the container, so its actual
/// rendered height is `container_w * (MAP_HEIGHT_PX / MAP_WIDTH_PX)`, which may
/// exceed the container height.  We must account for this so the user can pan
/// down to see the full map.
fn clamp_pan(
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    container_w: f64,
    container_h: f64,
) -> (f64, f64) {
    let content_w = container_w * zoom;
    let content_h = container_w * (grid::MAP_HEIGHT_PX / grid::MAP_WIDTH_PX) * zoom;
    let min_pan_x = -(content_w - container_w).max(0.0);
    let min_pan_y = -(content_h - container_h).max(0.0);
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
#[allow(clippy::too_many_arguments)]
fn build_svg_content(
    guns: &[(f64, f64)],
    targets: &[(f64, f64)],
    spotters: &[(f64, f64)],
    gun_weapons: &[Option<&WeaponData>],
    gun_target_indices: &[Option<usize>],
    accuracy_radii_px: &[Option<f64>],
    zoom: f64,
    selected: Option<SelectedMarker>,
) -> String {
    let mut svg = String::with_capacity(8192);

    build_grid_lines(&mut svg);
    build_grid_labels(&mut svg);
    if zoom >= 3.0 {
        build_keypad_lines(&mut svg);
        build_keypad_labels(&mut svg);
    }
    build_range_circles(&mut svg, guns, gun_weapons, zoom);
    build_firing_lines(&mut svg, guns, targets, gun_target_indices, zoom);
    build_accuracy_circles(&mut svg, guns, targets, gun_target_indices, accuracy_radii_px, zoom);
    build_gun_markers(&mut svg, guns, zoom, selected);
    build_target_markers(&mut svg, targets, zoom, selected);
    build_spotter_markers(&mut svg, spotters, zoom, selected);

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
    gun_target_indices: &[Option<usize>],
    zoom: f64,
) {
    for (gun_idx, &(gx, gy)) in guns.iter().enumerate() {
        let target_idx = gun_target_indices.get(gun_idx).and_then(|o| *o);
        if let Some(ti) = target_idx {
            if let Some(&(tx, ty)) = targets.get(ti) {
                let s = 1.0 / zoom.min(5.0);
                let sw = 1.5 * s;
                let da1 = 6.0 * s;
                let da2 = 4.0 * s;
                svg.push_str(&format!(
                    r#"<line x1="{gx}" y1="{gy}" x2="{tx}" y2="{ty}" stroke="rgba(233,69,96,0.7)" stroke-width="{sw}" stroke-dasharray="{da1} {da2}"/>"#
                ));
            }
        }
    }
}

fn build_accuracy_circles(
    svg: &mut String,
    guns: &[(f64, f64)],
    targets: &[(f64, f64)],
    gun_target_indices: &[Option<usize>],
    accuracy_radii_px: &[Option<f64>],
    zoom: f64,
) {
    // Draw accuracy circle at the target for each paired gun that has a solution
    for (gun_idx, _) in guns.iter().enumerate() {
        let target_idx = gun_target_indices.get(gun_idx).and_then(|o| *o);
        let acc_r = accuracy_radii_px.get(gun_idx).and_then(|o| *o);
        if let (Some(ti), Some(acc_r)) = (target_idx, acc_r) {
            if let Some(&(tx, ty)) = targets.get(ti) {
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
}

/// Generate marker label: no number suffix for single markers, numbered for multiple.
fn marker_label(base: &str, index: usize, total: usize) -> String {
    if total <= 1 {
        base.to_string()
    } else {
        format!("{} {}", base, index + 1)
    }
}

fn build_gun_markers(svg: &mut String, guns: &[(f64, f64)], zoom: f64, selected: Option<SelectedMarker>) {
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
        if selected == Some(SelectedMarker { kind: MarkerKind::Gun, index: i }) {
            build_selection_ring(svg, gx, gy, s);
        }
    }
}

fn build_target_markers(svg: &mut String, targets: &[(f64, f64)], zoom: f64, selected: Option<SelectedMarker>) {
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
        if selected == Some(SelectedMarker { kind: MarkerKind::Target, index: i }) {
            build_selection_ring(svg, tx, ty, s);
        }
    }
}

fn build_spotter_markers(svg: &mut String, spotters: &[(f64, f64)], zoom: f64, selected: Option<SelectedMarker>) {
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
        if selected == Some(SelectedMarker { kind: MarkerKind::Spotter, index: i }) {
            build_selection_ring(svg, sx, sy, s);
        }
    }
}

/// Emit an animated dashed selection ring around a marker.
fn build_selection_ring(svg: &mut String, cx: f64, cy: f64, s: f64) {
    let r = 12.0 * s;
    let sw = 1.5 * s;
    let da1 = 3.0 * s;
    let da2 = 2.0 * s;
    svg.push_str(&format!(
        r##"<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="white" stroke-width="{sw}" stroke-dasharray="{da1} {da2}" opacity="0.9"><animate attributeName="opacity" values="0.5;1;0.5" dur="1.2s" repeatCount="indefinite"/></circle>"##
    ));
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
    gun_target_indices: Signal<Vec<Option<usize>>>,
    selected_weapon_slug: Signal<String>,
    weapons: Vec<WeaponData>,
    accuracy_radii_px: Vec<Option<f64>>,
    selected_marker: Signal<Option<SelectedMarker>>,
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
    let pairings = gun_target_indices.read().clone();

    // Resolve per-gun weapon data
    let gun_weapons: Vec<Option<&WeaponData>> = wids.iter().map(|slug| {
        weapons.iter().find(|w| w.slug == *slug)
    }).collect();

    // Snapshot current transform for the render
    let cur_zoom = *zoom.read();
    let cur_selected = *selected_marker.read();

    let svg_content = build_svg_content(&guns, &targets, &spotters, &gun_weapons, &pairings, &accuracy_radii_px, cur_zoom, cur_selected);
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
    let has_selection = cur_selected.is_some();
    let container_class = if dragging {
        "map-container dragging"
    } else if has_selection {
        "map-container move-mode"
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
            tabindex: "0",

            onkeydown: move |evt: Event<KeyboardData>| {
                if evt.key() == Key::Escape {
                    selected_marker.set(None);
                }
            },

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

                // A mouseup without drag movement = a click
                if was_dragging && !was_drag {
                    let client = evt.client_coordinates();
                    if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                        client.x, client.y, MAP_CONTAINER_ID,
                        *zoom.read(), *pan_x.read(), *pan_y.read(),
                    ) {
                        // Move-mode: if a marker is selected, move it instead of placing.
                        // Special case: if a Gun is selected and the click is near an
                        // existing target, pair the gun with that target instead of moving.
                        let cur_sel = *selected_marker.read();
                        if let Some(sm) = cur_sel {
                            match sm.kind {
                                MarkerKind::Gun => {
                                    let targets_snap = target_positions.read().clone();
                                    let threshold = REMOVE_THRESHOLD / (*zoom.read()).min(5.0);
                                    if let Some(ti) = find_nearest(&targets_snap, (img_x, img_y), threshold) {
                                        // Click was near a target — pair the gun with it
                                        if let Some(entry) = gun_target_indices.write().get_mut(sm.index) {
                                            *entry = Some(ti);
                                        }
                                    } else {
                                        // Click on empty space — move the gun
                                        if let Some(pos) = gun_positions.write().get_mut(sm.index) {
                                            *pos = (img_x, img_y);
                                        }
                                    }
                                }
                                MarkerKind::Target => {
                                    if let Some(pos) = target_positions.write().get_mut(sm.index) {
                                        *pos = (img_x, img_y);
                                    }
                                }
                                MarkerKind::Spotter => {
                                    if let Some(pos) = spotter_positions.write().get_mut(sm.index) {
                                        *pos = (img_x, img_y);
                                    }
                                }
                            }
                            selected_marker.set(None);
                            return;
                        }

                        // Normal placement mode
                        let mode = *placement_mode.read();
                        match mode {
                            PlacementMode::Gun => {
                                gun_positions.write().push((img_x, img_y));
                                gun_weapon_ids.write().push(selected_weapon_slug.read().clone());
                                // Auto-pair with first unpaired target
                                let pairings_snap = gun_target_indices.read().clone();
                                let targets_snap = target_positions.read().clone();
                                let unpaired_target = (0..targets_snap.len())
                                    .find(|ti| !pairings_snap.contains(&Some(*ti)));
                                gun_target_indices.write().push(unpaired_target);
                            }
                            PlacementMode::Target => {
                                let targets_snap = target_positions.read().clone();
                                let threshold = REMOVE_THRESHOLD / (*zoom.read()).min(5.0);
                                if let Some(ti) = find_nearest(&targets_snap, (img_x, img_y), threshold) {
                                    // Clicked near an existing target — pair the first unpaired gun with it
                                    let mut pairings = gun_target_indices.write();
                                    if let Some(entry) = pairings.iter_mut().find(|p| p.is_none()) {
                                        *entry = Some(ti);
                                    }
                                } else {
                                    // Place a new target and auto-pair with first unpaired gun
                                    target_positions.write().push((img_x, img_y));
                                    let new_target_idx = target_positions.read().len() - 1;
                                    let mut pairings = gun_target_indices.write();
                                    if let Some(entry) = pairings.iter_mut().find(|p| p.is_none()) {
                                        *entry = Some(new_target_idx);
                                    }
                                }
                            }
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

                    // Snapshot selection to avoid borrow conflicts
                    let cur_sel = *selected_marker.read();

                    // Check active placement mode's list first for priority
                    let mode = *placement_mode.read();
                    let (removed, removed_kind, removed_idx) = match mode {
                        PlacementMode::Gun => {
                            if let Some(idx) = find_nearest(&guns_snap, click, threshold) {
                                (true, Some(MarkerKind::Gun), Some(idx))
                            } else { (false, None, None) }
                        }
                        PlacementMode::Target => {
                            if let Some(idx) = find_nearest(&targets_snap, click, threshold) {
                                (true, Some(MarkerKind::Target), Some(idx))
                            } else { (false, None, None) }
                        }
                        PlacementMode::Spotter => {
                            if let Some(idx) = find_nearest(&spotters_snap, click, threshold) {
                                (true, Some(MarkerKind::Spotter), Some(idx))
                            } else { (false, None, None) }
                        }
                    };

                    // If nothing found in the active mode's list, check all lists
                    let (final_kind, final_idx) = if removed {
                        (removed_kind, removed_idx)
                    } else {
                        let gun_hit = find_nearest(&guns_snap, click, threshold)
                            .map(|idx| (idx, dist(&guns_snap[idx], click), MarkerKind::Gun));
                        let tgt_hit = find_nearest(&targets_snap, click, threshold)
                            .map(|idx| (idx, dist(&targets_snap[idx], click), MarkerKind::Target));
                        let spt_hit = find_nearest(&spotters_snap, click, threshold)
                            .map(|idx| (idx, dist(&spotters_snap[idx], click), MarkerKind::Spotter));

                        let nearest = [gun_hit, tgt_hit, spt_hit]
                            .into_iter()
                            .flatten()
                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                        match nearest {
                            Some((idx, _, kind)) => (Some(kind), Some(idx)),
                            None => (None, None),
                        }
                    };

                    // Perform the removal and fixup
                    if let (Some(kind), Some(idx)) = (final_kind, final_idx) {
                        match kind {
                            MarkerKind::Gun => {
                                gun_positions.write().remove(idx);
                                gun_weapon_ids.write().remove(idx);
                                let mut pairings = gun_target_indices.write();
                                if idx < pairings.len() {
                                    pairings.remove(idx);
                                }
                            }
                            MarkerKind::Target => {
                                target_positions.write().remove(idx);
                                let mut pairings = gun_target_indices.write();
                                for entry in pairings.iter_mut() {
                                    if let Some(ti) = entry {
                                        if *ti == idx {
                                            *entry = None;
                                        } else if *ti > idx {
                                            *ti -= 1;
                                        }
                                    }
                                }
                            }
                            MarkerKind::Spotter => {
                                spotter_positions.write().remove(idx);
                            }
                        }
                        // Fixup selection
                        if let Some(sm) = cur_sel {
                            if sm.kind == kind {
                                if sm.index == idx {
                                    selected_marker.set(None);
                                } else if sm.index > idx {
                                    selected_marker.set(Some(SelectedMarker { kind, index: sm.index - 1 }));
                                }
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- build_firing_lines tests ---

    #[test]
    fn test_firing_lines_with_explicit_pairings() {
        let guns = vec![(100.0, 200.0), (300.0, 400.0)];
        let targets = vec![(150.0, 250.0), (350.0, 450.0)];
        // Gun 0 → Target 1, Gun 1 → Target 0
        let pairings = vec![Some(1), Some(0)];
        let mut svg = String::new();
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0);
        // Should draw line from gun 0 to target 1
        assert!(svg.contains(r#"x1="100""#));
        assert!(svg.contains(r#"y1="200""#));
        assert!(svg.contains(r#"x2="350""#));
        assert!(svg.contains(r#"y2="450""#));
        // Should draw line from gun 1 to target 0
        assert!(svg.contains(r#"x1="300""#));
        assert!(svg.contains(r#"y1="400""#));
        assert!(svg.contains(r#"x2="150""#));
        assert!(svg.contains(r#"y2="250""#));
    }

    #[test]
    fn test_firing_lines_unpaired_gun_draws_nothing() {
        let guns = vec![(100.0, 200.0)];
        let targets = vec![(150.0, 250.0)];
        let pairings = vec![None]; // Gun 0 unpaired
        let mut svg = String::new();
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0);
        assert!(svg.is_empty(), "Unpaired gun should not produce a firing line");
    }

    #[test]
    fn test_firing_lines_invalid_target_index_draws_nothing() {
        let guns = vec![(100.0, 200.0)];
        let targets = vec![(150.0, 250.0)];
        let pairings = vec![Some(5)]; // Out-of-bounds target index
        let mut svg = String::new();
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0);
        assert!(svg.is_empty(), "Invalid target index should not produce a firing line");
    }

    #[test]
    fn test_firing_lines_multiple_guns_same_target() {
        let guns = vec![(100.0, 200.0), (300.0, 400.0)];
        let targets = vec![(500.0, 600.0)];
        // Both guns target the same target
        let pairings = vec![Some(0), Some(0)];
        let mut svg = String::new();
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0);
        // Count the number of line elements — should be 2
        let line_count = svg.matches("<line").count();
        assert_eq!(line_count, 2, "Two guns pointing at same target should produce two lines");
    }

    // --- build_accuracy_circles tests ---

    #[test]
    fn test_accuracy_circles_with_pairings() {
        let guns = vec![(100.0, 200.0)];
        let targets = vec![(150.0, 250.0), (350.0, 450.0)];
        let pairings = vec![Some(1)]; // Gun 0 → Target 1
        let accuracy = vec![Some(10.0)];
        let mut svg = String::new();
        build_accuracy_circles(&mut svg, &guns, &targets, &pairings, &accuracy, 1.0);
        // Circle should be at target 1's position
        assert!(svg.contains(r#"cx="350""#));
        assert!(svg.contains(r#"cy="450""#));
        // Should NOT be at target 0's position
        assert!(!svg.contains(r#"cx="150""#));
    }

    #[test]
    fn test_accuracy_circles_unpaired_gun_draws_nothing() {
        let guns = vec![(100.0, 200.0)];
        let targets = vec![(150.0, 250.0)];
        let pairings = vec![None];
        let accuracy = vec![Some(10.0)];
        let mut svg = String::new();
        build_accuracy_circles(&mut svg, &guns, &targets, &pairings, &accuracy, 1.0);
        assert!(svg.is_empty());
    }

    // --- find_nearest tests ---

    #[test]
    fn test_find_nearest_within_threshold() {
        let positions = vec![(100.0, 100.0), (200.0, 200.0)];
        assert_eq!(find_nearest(&positions, (101.0, 101.0), 30.0), Some(0));
        assert_eq!(find_nearest(&positions, (199.0, 199.0), 30.0), Some(1));
    }

    #[test]
    fn test_find_nearest_outside_threshold() {
        let positions = vec![(100.0, 100.0)];
        assert_eq!(find_nearest(&positions, (200.0, 200.0), 30.0), None);
    }

    #[test]
    fn test_find_nearest_picks_closest() {
        // Two targets both within threshold — should pick the closer one
        let positions = vec![(100.0, 100.0), (110.0, 110.0)];
        assert_eq!(find_nearest(&positions, (108.0, 108.0), 30.0), Some(1));
        assert_eq!(find_nearest(&positions, (102.0, 102.0), 30.0), Some(0));
    }

    // --- marker_label tests ---

    #[test]
    fn test_marker_label_single() {
        assert_eq!(marker_label("GUN", 0, 1), "GUN");
    }

    #[test]
    fn test_marker_label_multiple() {
        assert_eq!(marker_label("GUN", 0, 3), "GUN 1");
        assert_eq!(marker_label("GUN", 2, 3), "GUN 3");
    }

    // --- clamp_pan tests ---

    #[test]
    fn test_clamp_pan_zoom1_map_fits_in_container() {
        // Container is taller than the map: no panning needed
        // container_w=1024, image_h = 1024*(888/1024) = 888, container_h=1000 > 888
        let (px, py) = clamp_pan(0.0, 0.0, 1.0, 1024.0, 1000.0);
        assert!((px - 0.0).abs() < 0.01);
        assert!((py - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_clamp_pan_zoom1_map_taller_than_container() {
        // Wide container: image renders taller than container
        // container_w=1600, image_h = 1600*(888/1024) ≈ 1387.5, container_h=1000
        // min_pan_y = -(1387.5 - 1000) = -387.5
        let (_, py) = clamp_pan(0.0, -200.0, 1.0, 1600.0, 1000.0);
        assert!((py - (-200.0)).abs() < 0.01, "Should allow panning down");
        let (_, py) = clamp_pan(0.0, -500.0, 1.0, 1600.0, 1000.0);
        let min_y = -(1600.0 * (grid::MAP_HEIGHT_PX / grid::MAP_WIDTH_PX) - 1000.0);
        assert!((py - min_y).abs() < 0.01, "Should clamp at min_pan_y");
    }

    #[test]
    fn test_clamp_pan_prevents_positive_pan() {
        // Pan should never go positive (would show empty space on left/top)
        let (px, py) = clamp_pan(50.0, 50.0, 1.0, 800.0, 600.0);
        assert!((px - 0.0).abs() < 0.01);
        assert!((py - 0.0).abs() < 0.01);
    }
}

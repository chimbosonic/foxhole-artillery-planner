use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use foxhole_shared::grid;

use crate::api::WeaponData;
use crate::coords;
use crate::pages::planner::{capture_snapshot, push_undo, PlanSnapshot};

const MAP_CONTAINER_ID: &str = "artillery-map-container";

/// Drag threshold in pixels — movement below this is treated as a click.
const DRAG_THRESHOLD: f64 = 3.0;

/// Touch drag threshold — larger than mouse because touch is less precise.
const TOUCH_DRAG_THRESHOLD: f64 = 8.0;

const ZOOM_MIN: f64 = 1.0;
const ZOOM_MAX: f64 = 10.0;
const ZOOM_STEP: f64 = 1.1;

/// Distance threshold (in map-image pixels, before zoom) for right-click removal.
const REMOVE_THRESHOLD: f64 = 60.0;

// --- Faction theme colors for SVG markers ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Faction {
    Warden,
    Colonial,
}

struct ThemeColors {
    gun: &'static str,
    target: &'static str,
    spotter: &'static str,
    target_label: &'static str,
    spotter_label: &'static str,
    min_range_fill: &'static str,
    firing_line_stroke: &'static str,
    accuracy_fill: &'static str,
}

const WARDEN_COLORS: ThemeColors = ThemeColors {
    gun: "#5ab882",
    target: "#c43030",
    spotter: "#4a8fd4",
    target_label: "#f0a0a0",
    spotter_label: "#b3d4f0",
    min_range_fill: "rgba(196,48,48,0.12)",
    firing_line_stroke: "rgba(196,48,48,0.85)",
    accuracy_fill: "rgba(196,48,48,0.25)",
};

const COLONIAL_COLORS: ThemeColors = ThemeColors {
    gun: "#5ab882",
    target: "#c43030",
    spotter: "#4a8fd4",
    target_label: "#f0a0a0",
    spotter_label: "#b3d4f0",
    min_range_fill: "rgba(196,48,48,0.12)",
    firing_line_stroke: "rgba(196,48,48,0.85)",
    accuracy_fill: "rgba(196,48,48,0.25)",
};

fn theme_colors(faction: Faction) -> &'static ThemeColors {
    match faction {
        Faction::Warden => &WARDEN_COLORS,
        Faction::Colonial => &COLONIAL_COLORS,
    }
}

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
fn clamp_pan(pan_x: f64, pan_y: f64, zoom: f64, container_w: f64, container_h: f64) -> (f64, f64) {
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
// Marker removal helpers
// ---------------------------------------------------------------------------

/// Euclidean distance between two points.
fn dist(a: &(f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

/// Distance between two client-coordinate points (for touch threshold checks).
fn point_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
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

/// Remove a marker by kind and index, fixing up gun-target pairings.
pub fn remove_marker(
    kind: MarkerKind,
    index: usize,
    gun_positions: &mut Signal<Vec<(f64, f64)>>,
    target_positions: &mut Signal<Vec<(f64, f64)>>,
    spotter_positions: &mut Signal<Vec<(f64, f64)>>,
    gun_weapon_ids: &mut Signal<Vec<String>>,
    gun_target_indices: &mut Signal<Vec<Option<usize>>>,
) {
    match kind {
        MarkerKind::Gun => {
            gun_positions.write().remove(index);
            gun_weapon_ids.write().remove(index);
            let mut pairings = gun_target_indices.write();
            if index < pairings.len() {
                pairings.remove(index);
            }
        }
        MarkerKind::Target => {
            target_positions.write().remove(index);
            let mut pairings = gun_target_indices.write();
            for entry in pairings.iter_mut() {
                if let Some(ti) = entry {
                    if *ti == index {
                        *entry = None;
                    } else if *ti > index {
                        *ti -= 1;
                    }
                }
            }
        }
        MarkerKind::Spotter => {
            spotter_positions.write().remove(index);
        }
    }
}

// ---------------------------------------------------------------------------
// SVG builder
// ---------------------------------------------------------------------------

/// Reference container width (desktop map panel) used to normalize marker sizes.
const REFERENCE_WIDTH: f64 = 960.0;

/// Build the full SVG content as a string for reliable rendering.
/// Positions are in native map-image pixel space (2048×1776).
#[allow(clippy::too_many_arguments)]
fn build_svg_content(
    guns: &[(f64, f64)],
    targets: &[(f64, f64)],
    spotters: &[(f64, f64)],
    gun_weapons: &[Option<&WeaponData>],
    gun_target_indices: &[Option<usize>],
    accuracy_radii_px: &[Option<f64>],
    zoom: f64,
    container_width: f64,
    selected: Option<SelectedMarker>,
    colors: &ThemeColors,
) -> String {
    let mut svg = String::with_capacity(8192);

    // Scale factor: keeps markers, strokes, and labels a consistent physical
    // size on screen regardless of container width.  On a 960 px desktop panel
    // the boost is 1.0; on a 430 px phone it's ~2.2×.
    let mobile_boost = (REFERENCE_WIDTH / container_width).max(1.0);
    let s = mobile_boost / zoom.min(5.0);

    build_grid_lines(&mut svg, mobile_boost);
    build_grid_labels(&mut svg, mobile_boost);
    if zoom >= 3.0 {
        build_keypad_lines(&mut svg, mobile_boost);
        build_keypad_labels(&mut svg, mobile_boost);
    }
    build_range_circles(&mut svg, guns, gun_weapons, s, colors);
    build_firing_lines(&mut svg, guns, targets, gun_target_indices, s, colors);
    build_accuracy_circles(
        &mut svg,
        guns,
        targets,
        gun_target_indices,
        accuracy_radii_px,
        s,
        colors,
    );
    build_gun_markers(&mut svg, guns, s, selected, colors);
    build_target_markers(&mut svg, targets, s, selected, colors);
    build_spotter_markers(&mut svg, spotters, s, selected, colors);

    svg
}

fn build_grid_lines(svg: &mut String, mb: f64) {
    let sw = 1.0 * mb;
    for col in 0..=grid::GRID_COLS {
        let x = grid::grid_col_px(col);
        svg.push_str(&format!(
            r#"<line x1="{x}" y1="0" x2="{x}" y2="{}" stroke="rgba(255,255,255,0.15)" stroke-width="{sw}"/>"#,
            grid::MAP_HEIGHT_PX
        ));
    }
    for row in 0..=grid::GRID_ROWS {
        let y = grid::grid_row_px(row);
        svg.push_str(&format!(
            r#"<line x1="0" y1="{y}" x2="{}" y2="{y}" stroke="rgba(255,255,255,0.15)" stroke-width="{sw}"/>"#,
            grid::MAP_WIDTH_PX
        ));
    }
}

fn build_grid_labels(svg: &mut String, mb: f64) {
    let fs = 18.0 * mb;
    let col_step = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    let col_y = 24.0 * mb;
    for col in 0..grid::GRID_COLS {
        let x = col as f64 * col_step + col_step / 2.0;
        let letter = grid::col_letter(col);
        svg.push_str(&format!(
            r#"<text x="{x}" y="{col_y}" fill="rgba(255,255,255,0.45)" font-size="{fs}" font-family="monospace" font-weight="600" text-anchor="middle" dominant-baseline="central">{letter}</text>"#
        ));
    }
    let row_step = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    let row_x = 8.0 * mb;
    for row in 0..grid::GRID_ROWS {
        let y = row as f64 * row_step + row_step / 2.0 + 8.0 * mb;
        let num = row + 1;
        svg.push_str(&format!(
            r#"<text x="{row_x}" y="{y}" fill="rgba(255,255,255,0.45)" font-size="{fs}" font-family="monospace" font-weight="600" text-anchor="start" dominant-baseline="central">{num}</text>"#
        ));
    }
}

fn build_keypad_lines(svg: &mut String, mb: f64) {
    let cell_w = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    let cell_h = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    let third_w = cell_w / 3.0;
    let third_h = cell_h / 3.0;
    let sw = 0.6 * mb;

    for col in 0..grid::GRID_COLS {
        let x0 = grid::grid_col_px(col);
        for i in 1..3 {
            let x = x0 + third_w * i as f64;
            svg.push_str(&format!(
                r#"<line x1="{x}" y1="0" x2="{x}" y2="{}" stroke="rgba(255,255,255,0.08)" stroke-width="{sw}"/>"#,
                grid::MAP_HEIGHT_PX
            ));
        }
    }
    for row in 0..grid::GRID_ROWS {
        let y0 = grid::grid_row_px(row);
        for i in 1..3 {
            let y = y0 + third_h * i as f64;
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y}" x2="{}" y2="{y}" stroke="rgba(255,255,255,0.08)" stroke-width="{sw}"/>"#,
                grid::MAP_WIDTH_PX
            ));
        }
    }
}

fn build_keypad_labels(svg: &mut String, mb: f64) {
    let cell_w = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    let cell_h = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    let third_w = cell_w / 3.0;
    let third_h = cell_h / 3.0;
    // Gentler boost — these labels sit inside small keypad cells
    let fs = 10.0 * mb.sqrt();

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
                        r#"<text x="{cx}" y="{cy}" fill="rgba(255,255,255,0.2)" font-size="{fs}" font-family="monospace" text-anchor="middle" dominant-baseline="central">{label}</text>"#
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
    s: f64,
    colors: &ThemeColors,
) {
    for (i, &(gx, gy)) in guns.iter().enumerate() {
        let Some(w) = gun_weapons.get(i).and_then(|o| *o) else {
            continue;
        };
        let max_r = coords::meters_to_image_px(w.max_range);
        let sw1 = 3.0 * s;
        let gun_color = colors.gun;
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{max_r}" fill="rgba(90,184,130,0.06)" stroke="{gun_color}" stroke-width="{sw1}" stroke-opacity="0.6"/>"##
        ));
        let min_r = coords::meters_to_image_px(w.min_range);
        let sw2 = 2.0 * s;
        let da1 = 8.0 * s;
        let da2 = 6.0 * s;
        let min_fill = colors.min_range_fill;
        let target_color = colors.target;
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{min_r}" fill="{min_fill}" stroke="{target_color}" stroke-width="{sw2}" stroke-dasharray="{da1} {da2}" stroke-opacity="0.5"/>"##
        ));
    }
}

fn build_firing_lines(
    svg: &mut String,
    guns: &[(f64, f64)],
    targets: &[(f64, f64)],
    gun_target_indices: &[Option<usize>],
    s: f64,
    colors: &ThemeColors,
) {
    for (gun_idx, &(gx, gy)) in guns.iter().enumerate() {
        let target_idx = gun_target_indices.get(gun_idx).and_then(|o| *o);
        if let Some(ti) = target_idx {
            if let Some(&(tx, ty)) = targets.get(ti) {
                let sw = 3.0 * s;
                let da1 = 12.0 * s;
                let da2 = 8.0 * s;
                let stroke = colors.firing_line_stroke;
                svg.push_str(&format!(
                    r#"<line x1="{gx}" y1="{gy}" x2="{tx}" y2="{ty}" stroke="{stroke}" stroke-width="{sw}" stroke-dasharray="{da1} {da2}"/>"#
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
    s: f64,
    colors: &ThemeColors,
) {
    // Draw accuracy circle at the target for each paired gun that has a solution
    for (gun_idx, _) in guns.iter().enumerate() {
        let target_idx = gun_target_indices.get(gun_idx).and_then(|o| *o);
        let acc_r = accuracy_radii_px.get(gun_idx).and_then(|o| *o);
        if let (Some(ti), Some(acc_r)) = (target_idx, acc_r) {
            if let Some(&(tx, ty)) = targets.get(ti) {
                let sw = 2.0 * s;
                let da1 = 6.0 * s;
                let da2 = 4.0 * s;
                let fill = colors.accuracy_fill;
                let target_color = colors.target;
                svg.push_str(&format!(
                    r##"<circle cx="{tx}" cy="{ty}" r="{acc_r}" fill="{fill}" stroke="{target_color}" stroke-width="{sw}" stroke-dasharray="{da1} {da2}"/>"##
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

fn build_gun_markers(
    svg: &mut String,
    guns: &[(f64, f64)],
    s: f64,
    selected: Option<SelectedMarker>,
    colors: &ThemeColors,
) {
    let total = guns.len();
    for (i, &(gx, gy)) in guns.iter().enumerate() {
        let r = 12.0 * s;
        let sw = 3.0 * s;
        let fs = 16.0 * s;
        let ty = gy - 20.0 * s;
        let tsw = 4.0 * s;
        let label = marker_label("GUN", i, total);
        let gun_color = colors.gun;
        svg.push_str(&format!(r##"<g role="img"><title>{label}</title>"##));
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{r}" fill="{gun_color}" stroke="white" stroke-width="{sw}"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{gx}" y="{ty}" fill="white" font-size="{fs}" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="{tsw}" paint-order="stroke">{label}</text>"##
        ));
        if selected
            == Some(SelectedMarker {
                kind: MarkerKind::Gun,
                index: i,
            })
        {
            build_selection_ring(svg, gx, gy, s);
        }
        svg.push_str("</g>");
    }
}

fn build_target_markers(
    svg: &mut String,
    targets: &[(f64, f64)],
    s: f64,
    selected: Option<SelectedMarker>,
    colors: &ThemeColors,
) {
    let total = targets.len();
    for (i, &(tx, ty)) in targets.iter().enumerate() {
        let arm = 16.0 * s;
        let sw = 3.0 * s;
        let r = 8.0 * s;
        let fs = 16.0 * s;
        let label_y = ty - 24.0 * s;
        let tsw = 4.0 * s;
        let label = marker_label("TARGET", i, total);
        let target_color = colors.target;
        let target_label = colors.target_label;
        svg.push_str(&format!(r##"<g role="img"><title>{label}</title>"##));
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{ty}" x2="{}" y2="{ty}" stroke="{target_color}" stroke-width="{sw}"/>"##,
            tx - arm,
            tx + arm
        ));
        svg.push_str(&format!(
            r##"<line x1="{tx}" y1="{}" x2="{tx}" y2="{}" stroke="{target_color}" stroke-width="{sw}"/>"##,
            ty - arm,
            ty + arm
        ));
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="{r}" fill="{target_color}" stroke="white" stroke-width="{sw}"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{tx}" y="{label_y}" fill="{target_label}" font-size="{fs}" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="{tsw}" paint-order="stroke">{label}</text>"##
        ));
        if selected
            == Some(SelectedMarker {
                kind: MarkerKind::Target,
                index: i,
            })
        {
            build_selection_ring(svg, tx, ty, s);
        }
        svg.push_str("</g>");
    }
}

fn build_spotter_markers(
    svg: &mut String,
    spotters: &[(f64, f64)],
    s: f64,
    selected: Option<SelectedMarker>,
    colors: &ThemeColors,
) {
    let total = spotters.len();
    for (i, &(sx, sy)) in spotters.iter().enumerate() {
        let r = 10.0 * s;
        let sw = 3.0 * s;
        let fs = 16.0 * s;
        let label_y = sy - 20.0 * s;
        let tsw = 4.0 * s;
        let label = marker_label("SPOTTER", i, total);
        let spotter_color = colors.spotter;
        let spotter_label = colors.spotter_label;
        svg.push_str(&format!(r##"<g role="img"><title>{label}</title>"##));
        svg.push_str(&format!(
            r##"<circle cx="{sx}" cy="{sy}" r="{r}" fill="{spotter_color}" stroke="white" stroke-width="{sw}"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{sx}" y="{label_y}" fill="{spotter_label}" font-size="{fs}" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="{tsw}" paint-order="stroke">{label}</text>"##
        ));
        if selected
            == Some(SelectedMarker {
                kind: MarkerKind::Spotter,
                index: i,
            })
        {
            build_selection_ring(svg, sx, sy, s);
        }
        svg.push_str("</g>");
    }
}

/// Emit an animated dashed selection ring around a marker.
fn build_selection_ring(svg: &mut String, cx: f64, cy: f64, s: f64) {
    let r = 24.0 * s;
    let sw = 3.0 * s;
    let da1 = 6.0 * s;
    let da2 = 4.0 * s;
    svg.push_str(&format!(
        r##"<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="white" stroke-width="{sw}" stroke-dasharray="{da1} {da2}" opacity="0.9"><animate attributeName="opacity" values="0.5;1;0.5" dur="1.2s" repeatCount="indefinite"/></circle>"##
    ));
}

// ---------------------------------------------------------------------------
// Shared marker-placement logic (used by both mouse and touch handlers)
// ---------------------------------------------------------------------------

/// Find the index of the first target not paired with any gun.
fn find_first_unpaired_target(pairings: &[Option<usize>], target_count: usize) -> Option<usize> {
    (0..target_count).find(|ti| !pairings.contains(&Some(*ti)))
}

/// Pair the first unpaired gun (None entry) with the given target index.
fn pair_first_unpaired_gun(pairings: &mut [Option<usize>], target_idx: usize) {
    if let Some(entry) = pairings.iter_mut().find(|p| p.is_none()) {
        *entry = Some(target_idx);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_marker_placement(
    img_x: f64,
    img_y: f64,
    zoom: f64,
    selected_marker: &mut Signal<Option<SelectedMarker>>,
    placement_mode: &mut Signal<PlacementMode>,
    gun_positions: &mut Signal<Vec<(f64, f64)>>,
    target_positions: &mut Signal<Vec<(f64, f64)>>,
    spotter_positions: &mut Signal<Vec<(f64, f64)>>,
    gun_weapon_ids: &mut Signal<Vec<String>>,
    gun_target_indices: &mut Signal<Vec<Option<usize>>>,
    selected_weapon_slug: &Signal<String>,
    push_snapshot: &mut dyn FnMut(),
) {
    // Move-mode: if a marker is selected, move it instead of placing.
    // Special case: if a Gun is selected and the click is near an
    // existing target, pair the gun with that target instead of moving.
    let cur_sel = *selected_marker.read();
    if let Some(sm) = cur_sel {
        push_snapshot();
        match sm.kind {
            MarkerKind::Gun => {
                let targets_snap = target_positions.read().clone();
                let threshold = REMOVE_THRESHOLD / zoom.min(5.0);
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
    push_snapshot();
    let mode = *placement_mode.read();
    match mode {
        PlacementMode::Gun => {
            gun_positions.write().push((img_x, img_y));
            let slug = selected_weapon_slug.read().clone();
            gun_weapon_ids.write().push(slug.clone());
            // Auto-pair with first unpaired target
            let unpaired = find_first_unpaired_target(&gun_target_indices.read(), target_positions.read().len());
            gun_target_indices.write().push(unpaired);
            // Fire-and-forget tracking
            crate::api::track_gun_placement_fire(&slug);
        }
        PlacementMode::Target => {
            let targets_snap = target_positions.read().clone();
            let threshold = REMOVE_THRESHOLD / zoom.min(5.0);
            if let Some(ti) = find_nearest(&targets_snap, (img_x, img_y), threshold) {
                // Clicked near an existing target — pair the first unpaired gun with it
                pair_first_unpaired_gun(&mut gun_target_indices.write(), ti);
                crate::api::track_target_placement_fire();
            } else {
                // Place a new target and auto-pair with first unpaired gun
                target_positions.write().push((img_x, img_y));
                let new_target_idx = target_positions.read().len() - 1;
                pair_first_unpaired_gun(&mut gun_target_indices.write(), new_target_idx);
                crate::api::track_target_placement_fire();
            }
        }
        PlacementMode::Spotter => {
            spotter_positions.write().push((img_x, img_y));
            crate::api::track_spotter_placement_fire();
        }
    }
    // Auto-cycle: Gun → Target → Gun for easy pairing
    match mode {
        PlacementMode::Gun => placement_mode.set(PlacementMode::Target),
        PlacementMode::Target => placement_mode.set(PlacementMode::Gun),
        PlacementMode::Spotter => {} // stay in spotter mode
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
#[allow(clippy::too_many_arguments)]
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
    accuracy_radii_px: ReadSignal<Vec<Option<f64>>>,
    selected_marker: Signal<Option<SelectedMarker>>,
    undo_stack: Signal<Vec<PlanSnapshot>>,
    redo_stack: Signal<Vec<PlanSnapshot>>,
    wind_direction: Signal<Option<f64>>,
    wind_strength: Signal<u32>,
    reset_view_counter: Signal<u64>,
    faction: Signal<Faction>,
) -> Element {
    let image_url = format!("/static/images/maps/{}.webp", map_file_name);

    // Zoom / pan state (local — resets when component is re-created via `key`)
    let mut zoom = use_signal(|| 1.0_f64);
    let mut pan_x = use_signal(|| 0.0_f64);
    let mut pan_y = use_signal(|| 0.0_f64);

    // Reset zoom/pan when parent signals via reset_view_counter
    use_effect(move || {
        // Read the Signal inside the effect so Dioxus tracks it as a dependency
        let _counter = *reset_view_counter.read();
        zoom.set(1.0);
        pan_x.set(0.0);
        pan_y.set(0.0);
    });

    // Mutable bindings for undo/redo (Signal is Copy)
    let mut undo_stack = undo_stack;
    let mut redo_stack = redo_stack;

    // Local closure to snapshot state before mutations
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

    // Drag state (mouse)
    let mut is_dragging = use_signal(|| false);
    let mut did_drag = use_signal(|| false);
    let mut drag_start_x = use_signal(|| 0.0_f64);
    let mut drag_start_y = use_signal(|| 0.0_f64);
    let mut drag_start_pan_x = use_signal(|| 0.0_f64);
    let mut drag_start_pan_y = use_signal(|| 0.0_f64);

    // Touch state
    let mut touch_start_pos = use_signal(|| None::<(f64, f64)>);
    let mut touch_did_pan = use_signal(|| false);
    let mut touch_start_pan_x = use_signal(|| 0.0_f64);
    let mut touch_start_pan_y = use_signal(|| 0.0_f64);
    let mut is_pinching = use_signal(|| false);
    let mut pinch_start_distance = use_signal(|| 0.0_f64);
    let mut pinch_start_zoom = use_signal(|| 1.0_f64);
    let mut pinch_midpoint = use_signal(|| (0.0_f64, 0.0_f64));
    let mut pinch_start_pan_x = use_signal(|| 0.0_f64);
    let mut pinch_start_pan_y = use_signal(|| 0.0_f64);

    // Memoize SVG generation — only recomputes when positions, zoom, selection,
    // faction, weapons, pairings, or accuracy radii change. Pan changes (pan_x/pan_y)
    // are read outside this memo so they don't trigger SVG rebuilds.
    let svg_html = use_memo(move || {
        let guns = gun_positions.read();
        let targets = target_positions.read();
        let spotters = spotter_positions.read();
        let wids = gun_weapon_ids.read();
        let pairings = gun_target_indices.read();
        let acc_radii = accuracy_radii_px.read();

        let gun_weapons: Vec<Option<&WeaponData>> = wids
            .iter()
            .map(|slug| weapons.iter().find(|w| w.slug == *slug))
            .collect();

        let cur_zoom = *zoom.read();
        let cur_selected = *selected_marker.read();
        let colors = theme_colors(*faction.read());
        let cw = container_rect().map(|r| r.width()).unwrap_or(REFERENCE_WIDTH);

        let svg_content = build_svg_content(
            &guns,
            &targets,
            &spotters,
            &gun_weapons,
            &pairings,
            &acc_radii,
            cur_zoom,
            cw,
            cur_selected,
            colors,
        );
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none" style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:5;">{}</svg>"#,
            grid::MAP_WIDTH_PX,
            grid::MAP_HEIGHT_PX,
            svg_content
        )
    });

    let cur_pan_x = *pan_x.read();
    let cur_pan_y = *pan_y.read();
    let cur_zoom = *zoom.read();
    let dragging = *is_dragging.read();
    let cur_selected = *selected_marker.read();

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

    // Build coord readout tags (fresh signal reads — cheap, Dioxus deduplicates tracking)
    let guns = gun_positions.read();
    let targets = target_positions.read();
    let spotters = spotter_positions.read();

    let gun_tags: Vec<(String, String)> = guns
        .iter()
        .enumerate()
        .map(|(i, pos)| {
            let label = if guns.len() <= 1 {
                "GUN".to_string()
            } else {
                format!("GUN {}", i + 1)
            };
            (label, coords::format_px_as_grid(pos.0, pos.1))
        })
        .collect();
    let target_tags: Vec<(String, String)> = targets
        .iter()
        .enumerate()
        .map(|(i, pos)| {
            let label = if targets.len() <= 1 {
                "TGT".to_string()
            } else {
                format!("TGT {}", i + 1)
            };
            (label, coords::format_px_as_grid(pos.0, pos.1))
        })
        .collect();
    let spotter_tags: Vec<(String, String)> = spotters
        .iter()
        .enumerate()
        .map(|(i, pos)| {
            let label = if spotters.len() <= 1 {
                "SPT".to_string()
            } else {
                format!("SPT {}", i + 1)
            };
            (label, coords::format_px_as_grid(pos.0, pos.1))
        })
        .collect();

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

                // A mouseup without drag movement = a click
                if was_dragging && !was_drag {
                    let client = evt.client_coordinates();
                    if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                        client.x, client.y, MAP_CONTAINER_ID,
                        *zoom.read(), *pan_x.read(), *pan_y.read(),
                    ) {
                        handle_marker_placement(
                            img_x, img_y, *zoom.read(),
                            &mut selected_marker, &mut placement_mode,
                            &mut gun_positions, &mut target_positions, &mut spotter_positions,
                            &mut gun_weapon_ids, &mut gun_target_indices,
                            &selected_weapon_slug, &mut push_snapshot,
                        );
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
                    let mode_hit = match mode {
                        PlacementMode::Gun => find_nearest(&guns_snap, click, threshold)
                            .map(|idx| (MarkerKind::Gun, idx)),
                        PlacementMode::Target => find_nearest(&targets_snap, click, threshold)
                            .map(|idx| (MarkerKind::Target, idx)),
                        PlacementMode::Spotter => find_nearest(&spotters_snap, click, threshold)
                            .map(|idx| (MarkerKind::Spotter, idx)),
                    };

                    // If nothing found in the active mode's list, check all lists
                    let target = mode_hit.or_else(|| {
                        let gun_hit = find_nearest(&guns_snap, click, threshold)
                            .map(|idx| (idx, dist(&guns_snap[idx], click), MarkerKind::Gun));
                        let tgt_hit = find_nearest(&targets_snap, click, threshold)
                            .map(|idx| (idx, dist(&targets_snap[idx], click), MarkerKind::Target));
                        let spt_hit = find_nearest(&spotters_snap, click, threshold)
                            .map(|idx| (idx, dist(&spotters_snap[idx], click), MarkerKind::Spotter));

                        [gun_hit, tgt_hit, spt_hit]
                            .into_iter()
                            .flatten()
                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                            .map(|(idx, _, kind)| (kind, idx))
                    });

                    // Perform the removal and fixup
                    if let Some((kind, idx)) = target {
                        push_snapshot();
                        remove_marker(
                            kind, idx,
                            &mut gun_positions, &mut target_positions, &mut spotter_positions,
                            &mut gun_weapon_ids, &mut gun_target_indices,
                        );
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

            // --- Touch event handlers ---

            ontouchstart: move |evt: Event<TouchData>| {
                evt.prevent_default();
                let touches = evt.data().touches();
                if touches.len() == 1 {
                    // Single finger: record start position for tap detection and panning
                    let t = &touches[0];
                    touch_start_pos.set(Some((t.client_coordinates().x, t.client_coordinates().y)));
                    touch_did_pan.set(false);
                    touch_start_pan_x.set(*pan_x.read());
                    touch_start_pan_y.set(*pan_y.read());
                } else if touches.len() >= 2 {
                    // Two fingers: start pinch-to-zoom
                    let t0 = &touches[0];
                    let t1 = &touches[1];
                    let p0 = (t0.client_coordinates().x, t0.client_coordinates().y);
                    let p1 = (t1.client_coordinates().x, t1.client_coordinates().y);
                    let d = point_distance(p0, p1);
                    is_pinching.set(true);
                    pinch_start_distance.set(d);
                    pinch_start_zoom.set(*zoom.read());
                    let mid = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
                    pinch_midpoint.set(mid);
                    pinch_start_pan_x.set(*pan_x.read());
                    pinch_start_pan_y.set(*pan_y.read());
                    // Cancel any tap tracking
                    touch_start_pos.set(None);
                    touch_did_pan.set(true);
                }
            },

            ontouchmove: move |evt: Event<TouchData>| {
                evt.prevent_default();
                let touches = evt.data().touches();

                if *is_pinching.read() && touches.len() >= 2 {
                    // Pinch-to-zoom
                    let t0 = &touches[0];
                    let t1 = &touches[1];
                    let p0 = (t0.client_coordinates().x, t0.client_coordinates().y);
                    let p1 = (t1.client_coordinates().x, t1.client_coordinates().y);
                    let d = point_distance(p0, p1);
                    let start_d = *pinch_start_distance.read();
                    if start_d < 1.0 { return; }

                    let scale = d / start_d;
                    let old_z = *pinch_start_zoom.read();
                    let new_z = (old_z * scale).clamp(ZOOM_MIN, ZOOM_MAX);

                    // Zoom centered on the pinch midpoint
                    let Some(rect) = container_rect() else { return };
                    let mid = *pinch_midpoint.read();
                    let cx = mid.0 - rect.left();
                    let cy = mid.1 - rect.top();
                    let (new_px, new_py) = zoom_pan_at_cursor(
                        cx, cy, old_z, new_z,
                        *pinch_start_pan_x.read(), *pinch_start_pan_y.read(),
                    );
                    let (px, py) = clamp_pan(new_px, new_py, new_z, rect.width(), rect.height());
                    zoom.set(new_z);
                    pan_x.set(px);
                    pan_y.set(py);
                } else if touches.len() == 1 {
                    // Single finger pan
                    let t = &touches[0];
                    let cur = (t.client_coordinates().x, t.client_coordinates().y);
                    if let Some(start) = *touch_start_pos.read() {
                        let dx = cur.0 - start.0;
                        let dy = cur.1 - start.1;
                        if !*touch_did_pan.read() && point_distance(start, cur) > TOUCH_DRAG_THRESHOLD {
                            touch_did_pan.set(true);
                        }
                        if *touch_did_pan.read() {
                            let new_px = *touch_start_pan_x.read() + dx;
                            let new_py = *touch_start_pan_y.read() + dy;
                            let (px, py) = clamp_pan_to_container(new_px, new_py, *zoom.read());
                            pan_x.set(px);
                            pan_y.set(py);
                        }
                    }
                }
            },

            ontouchend: move |evt: Event<TouchData>| {
                evt.prevent_default();
                let remaining = evt.data().touches().len();

                if *is_pinching.read() {
                    // Wait for all fingers to lift before resetting pinch state
                    if remaining == 0 {
                        is_pinching.set(false);
                        touch_start_pos.set(None);
                    }
                    return;
                }

                // Single-finger tap: if no pan occurred and all fingers are up, treat as tap
                if remaining == 0 && !*touch_did_pan.read() {
                    if let Some(start) = *touch_start_pos.read() {
                        if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                            start.0, start.1, MAP_CONTAINER_ID,
                            *zoom.read(), *pan_x.read(), *pan_y.read(),
                        ) {
                            handle_marker_placement(
                                img_x, img_y, *zoom.read(),
                                &mut selected_marker, &mut placement_mode,
                                &mut gun_positions, &mut target_positions, &mut spotter_positions,
                                &mut gun_weapon_ids, &mut gun_target_indices,
                                &selected_weapon_slug, &mut push_snapshot,
                            );
                        }
                    }
                }

                if remaining == 0 {
                    touch_start_pos.set(None);
                }
            },

            ontouchcancel: move |_evt: Event<TouchData>| {
                // Reset all touch state
                touch_start_pos.set(None);
                touch_did_pan.set(false);
                is_pinching.set(false);
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
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0, &WARDEN_COLORS);
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
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0, &WARDEN_COLORS);
        assert!(
            svg.is_empty(),
            "Unpaired gun should not produce a firing line"
        );
    }

    #[test]
    fn test_firing_lines_invalid_target_index_draws_nothing() {
        let guns = vec![(100.0, 200.0)];
        let targets = vec![(150.0, 250.0)];
        let pairings = vec![Some(5)]; // Out-of-bounds target index
        let mut svg = String::new();
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0, &WARDEN_COLORS);
        assert!(
            svg.is_empty(),
            "Invalid target index should not produce a firing line"
        );
    }

    #[test]
    fn test_firing_lines_multiple_guns_same_target() {
        let guns = vec![(100.0, 200.0), (300.0, 400.0)];
        let targets = vec![(500.0, 600.0)];
        // Both guns target the same target
        let pairings = vec![Some(0), Some(0)];
        let mut svg = String::new();
        build_firing_lines(&mut svg, &guns, &targets, &pairings, 1.0, &WARDEN_COLORS);
        // Count the number of line elements — should be 2
        let line_count = svg.matches("<line").count();
        assert_eq!(
            line_count, 2,
            "Two guns pointing at same target should produce two lines"
        );
    }

    // --- build_accuracy_circles tests ---

    #[test]
    fn test_accuracy_circles_with_pairings() {
        let guns = vec![(100.0, 200.0)];
        let targets = vec![(150.0, 250.0), (350.0, 450.0)];
        let pairings = vec![Some(1)]; // Gun 0 → Target 1
        let accuracy = vec![Some(10.0)];
        let mut svg = String::new();
        build_accuracy_circles(&mut svg, &guns, &targets, &pairings, &accuracy, 1.0, &WARDEN_COLORS);
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
        build_accuracy_circles(&mut svg, &guns, &targets, &pairings, &accuracy, 1.0, &WARDEN_COLORS);
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
        // container_w=2048, image_h = 2048*(1776/2048) = 1776, container_h=2000 > 1776
        let (px, py) = clamp_pan(0.0, 0.0, 1.0, 2048.0, 2000.0);
        assert!((px - 0.0).abs() < 0.01);
        assert!((py - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_clamp_pan_zoom1_map_taller_than_container() {
        // Wide container: image renders taller than container
        // container_w=3200, image_h = 3200*(1776/2048) ≈ 2775, container_h=2000
        // min_pan_y = -(2775 - 2000) = -775
        let (_, py) = clamp_pan(0.0, -400.0, 1.0, 3200.0, 2000.0);
        assert!((py - (-400.0)).abs() < 0.01, "Should allow panning down");
        let (_, py) = clamp_pan(0.0, -1000.0, 1.0, 3200.0, 2000.0);
        let min_y = -(3200.0 * (grid::MAP_HEIGHT_PX / grid::MAP_WIDTH_PX) - 2000.0);
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

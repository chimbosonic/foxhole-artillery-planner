use dioxus::html::geometry::WheelDelta;
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
// SVG builder
// ---------------------------------------------------------------------------

/// Build the full SVG content as a string for reliable rendering.
/// Positions are in native map-image pixel space (1024×888).
fn build_svg_content(
    gun: Option<(f64, f64)>,
    target: Option<(f64, f64)>,
    spotter: Option<(f64, f64)>,
    weapon: &Option<WeaponData>,
    accuracy_radius_px: Option<f64>,
) -> String {
    let mut svg = String::with_capacity(8192);

    build_grid_lines(&mut svg);
    build_grid_labels(&mut svg);
    build_range_circles(&mut svg, gun, weapon);
    build_firing_line(&mut svg, gun, target);
    build_accuracy_circle(&mut svg, target, accuracy_radius_px);
    build_gun_marker(&mut svg, gun);
    build_target_marker(&mut svg, target);
    build_spotter_marker(&mut svg, spotter);

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

fn build_range_circles(
    svg: &mut String,
    gun: Option<(f64, f64)>,
    weapon: &Option<WeaponData>,
) {
    if let (Some((gx, gy)), Some(w)) = (gun, weapon) {
        let max_r = coords::meters_to_image_px(w.max_range);
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{max_r}" fill="rgba(78,204,163,0.06)" stroke="#4ecca3" stroke-width="1.5" stroke-opacity="0.6"/>"##
        ));
        let min_r = coords::meters_to_image_px(w.min_range);
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="{min_r}" fill="rgba(233,69,96,0.06)" stroke="#e94560" stroke-width="1" stroke-dasharray="4 3" stroke-opacity="0.5"/>"##
        ));
    }
}

fn build_firing_line(
    svg: &mut String,
    gun: Option<(f64, f64)>,
    target: Option<(f64, f64)>,
) {
    if let (Some((gx, gy)), Some((tx, ty))) = (gun, target) {
        svg.push_str(&format!(
            r#"<line x1="{gx}" y1="{gy}" x2="{tx}" y2="{ty}" stroke="rgba(233,69,96,0.7)" stroke-width="1.5" stroke-dasharray="6 4"/>"#
        ));
    }
}

fn build_accuracy_circle(
    svg: &mut String,
    target: Option<(f64, f64)>,
    accuracy_radius_px: Option<f64>,
) {
    if let (Some((tx, ty)), Some(acc_r)) = (target, accuracy_radius_px) {
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="{acc_r}" fill="rgba(233,69,96,0.15)" stroke="#e94560" stroke-width="1" stroke-dasharray="3 2"/>"##
        ));
    }
}

fn build_gun_marker(svg: &mut String, gun: Option<(f64, f64)>) {
    if let Some((gx, gy)) = gun {
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="6" fill="#4ecca3" stroke="white" stroke-width="1.5"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{gx}" y="{}" fill="white" font-size="8" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="2" paint-order="stroke">GUN</text>"##,
            gy - 10.0
        ));
    }
}

fn build_target_marker(svg: &mut String, target: Option<(f64, f64)>) {
    if let Some((tx, ty)) = target {
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{ty}" x2="{}" y2="{ty}" stroke="#e94560" stroke-width="1.5"/>"##,
            tx - 8.0,
            tx + 8.0
        ));
        svg.push_str(&format!(
            r##"<line x1="{tx}" y1="{}" x2="{tx}" y2="{}" stroke="#e94560" stroke-width="1.5"/>"##,
            ty - 8.0,
            ty + 8.0
        ));
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="4" fill="#e94560" stroke="white" stroke-width="1.5"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{tx}" y="{}" fill="#ffcccc" font-size="8" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="2" paint-order="stroke">TARGET</text>"##,
            ty - 12.0
        ));
    }
}

fn build_spotter_marker(svg: &mut String, spotter: Option<(f64, f64)>) {
    if let Some((sx, sy)) = spotter {
        svg.push_str(&format!(
            r##"<circle cx="{sx}" cy="{sy}" r="5" fill="#7ec8e3" stroke="white" stroke-width="1.5"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{sx}" y="{}" fill="#cce7ff" font-size="8" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="2" paint-order="stroke">SPOTTER</text>"##,
            sy - 10.0
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
    gun_pos: Signal<Option<(f64, f64)>>,
    target_pos: Signal<Option<(f64, f64)>>,
    spotter_pos: Signal<Option<(f64, f64)>>,
    selected_weapon: Option<WeaponData>,
    accuracy_radius_px: Option<f64>,
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
    let gun = *gun_pos.read();
    let target = *target_pos.read();
    let spotter = *spotter_pos.read();

    let svg_content = build_svg_content(gun, target, spotter, &selected_weapon, accuracy_radius_px);
    let svg_html = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none" style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:5;">{}</svg>"#,
        grid::MAP_WIDTH_PX, grid::MAP_HEIGHT_PX, svg_content
    );

    // Snapshot current transform for the render
    let cur_zoom = *zoom.read();
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

                // A mouseup without drag movement = a click → place marker
                if was_dragging && !was_drag {
                    let client = evt.client_coordinates();
                    if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                        client.x, client.y, MAP_CONTAINER_ID,
                        *zoom.read(), *pan_x.read(), *pan_y.read(),
                    ) {
                        match *placement_mode.read() {
                            PlacementMode::Gun => gun_pos.set(Some((img_x, img_y))),
                            PlacementMode::Target => target_pos.set(Some((img_x, img_y))),
                            PlacementMode::Spotter => spotter_pos.set(Some((img_x, img_y))),
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
                if let Some(pos) = gun {
                    span { class: "coord-tag gun-tag",
                        "GUN: {coords::format_px_as_grid(pos.0, pos.1)}"
                    }
                }
                if let Some(pos) = target {
                    span { class: "coord-tag target-tag",
                        "TGT: {coords::format_px_as_grid(pos.0, pos.1)}"
                    }
                }
            }
        }
    }
}

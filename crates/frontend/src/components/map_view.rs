use dioxus::prelude::*;
use dioxus::html::geometry::WheelDelta;
use foxhole_shared::grid;

use crate::api::WeaponData;
use crate::coords;

const MAP_CONTAINER_ID: &str = "artillery-map-container";

/// Drag threshold in pixels — movement below this is treated as a click.
const DRAG_THRESHOLD: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlacementMode {
    Gun,
    Target,
    Spotter,
}

/// Build the full SVG content as a string for reliable rendering.
/// Positions are in native map-image pixel space (1024x888).
fn build_svg_content(
    gun: Option<(f64, f64)>,
    target: Option<(f64, f64)>,
    spotter: Option<(f64, f64)>,
    weapon: &Option<WeaponData>,
    accuracy_radius_px: Option<f64>,
) -> String {
    let mut svg = String::with_capacity(8192);

    // Grid lines - columns
    for col in 0..=grid::GRID_COLS {
        let x = grid::grid_col_px(col);
        svg.push_str(&format!(
            r#"<line x1="{x}" y1="0" x2="{x}" y2="{}" stroke="rgba(255,255,255,0.15)" stroke-width="0.5"/>"#,
            grid::MAP_HEIGHT_PX
        ));
    }

    // Grid lines - rows
    for row in 0..=grid::GRID_ROWS {
        let y = grid::grid_row_px(row);
        svg.push_str(&format!(
            r#"<line x1="0" y1="{y}" x2="{}" y2="{y}" stroke="rgba(255,255,255,0.15)" stroke-width="0.5"/>"#,
            grid::MAP_WIDTH_PX
        ));
    }

    // Grid column labels (A-Q)
    let col_step = grid::MAP_WIDTH_PX / grid::GRID_COLS as f64;
    for col in 0..grid::GRID_COLS {
        let x = col as f64 * col_step + col_step / 2.0;
        let letter = grid::col_letter(col);
        svg.push_str(&format!(
            r#"<text x="{x}" y="12" fill="rgba(255,255,255,0.45)" font-size="9" font-family="monospace" font-weight="600" text-anchor="middle" dominant-baseline="central">{letter}</text>"#
        ));
    }

    // Grid row labels (1-15)
    let row_step = grid::MAP_HEIGHT_PX / grid::GRID_ROWS as f64;
    for row in 0..grid::GRID_ROWS {
        let y = row as f64 * row_step + row_step / 2.0 + 4.0;
        let num = row + 1;
        svg.push_str(&format!(
            r#"<text x="4" y="{y}" fill="rgba(255,255,255,0.45)" font-size="9" font-family="monospace" font-weight="600" text-anchor="start" dominant-baseline="central">{num}</text>"#
        ));
    }

    // Range circles centered on gun
    if let Some((gx, gy)) = gun {
        if let Some(w) = weapon {
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

    // Firing line from gun to target
    if let (Some((gx, gy)), Some((tx, ty))) = (gun, target) {
        svg.push_str(&format!(
            r#"<line x1="{gx}" y1="{gy}" x2="{tx}" y2="{ty}" stroke="rgba(233,69,96,0.7)" stroke-width="1.5" stroke-dasharray="6 4"/>"#
        ));
    }

    // Accuracy/dispersion circle at target
    if let (Some((tx, ty)), Some(acc_r)) = (target, accuracy_radius_px) {
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="{acc_r}" fill="rgba(233,69,96,0.15)" stroke="#e94560" stroke-width="1" stroke-dasharray="3 2"/>"##
        ));
    }

    // Gun marker
    if let Some((gx, gy)) = gun {
        svg.push_str(&format!(
            r##"<circle cx="{gx}" cy="{gy}" r="6" fill="#4ecca3" stroke="white" stroke-width="1.5"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{gx}" y="{}" fill="white" font-size="8" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="2" paint-order="stroke">GUN</text>"##,
            gy - 10.0
        ));
    }

    // Target marker (crosshair)
    if let Some((tx, ty)) = target {
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{ty}" x2="{}" y2="{ty}" stroke="#e94560" stroke-width="1.5"/>"##,
            tx - 8.0, tx + 8.0
        ));
        svg.push_str(&format!(
            r##"<line x1="{tx}" y1="{}" x2="{tx}" y2="{}" stroke="#e94560" stroke-width="1.5"/>"##,
            ty - 8.0, ty + 8.0
        ));
        svg.push_str(&format!(
            r##"<circle cx="{tx}" cy="{ty}" r="4" fill="#e94560" stroke="white" stroke-width="1.5"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{tx}" y="{}" fill="#ffcccc" font-size="8" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="2" paint-order="stroke">TARGET</text>"##,
            ty - 12.0
        ));
    }

    // Spotter marker
    if let Some((sx, sy)) = spotter {
        svg.push_str(&format!(
            r##"<circle cx="{sx}" cy="{sy}" r="5" fill="#7ec8e3" stroke="white" stroke-width="1.5"/>"##
        ));
        svg.push_str(&format!(
            r##"<text x="{sx}" y="{}" fill="#cce7ff" font-size="8" font-family="sans-serif" font-weight="700" text-anchor="middle" stroke="rgba(0,0,0,0.7)" stroke-width="2" paint-order="stroke">SPOTTER</text>"##,
            sy - 10.0
        ));
    }

    svg
}

/// Clamp pan values so the map can't be dragged entirely off-screen.
/// At zoom=1 pan should be 0,0. At higher zoom levels, allow panning
/// within the bounds of the zoomed content.
fn clamp_pan(pan_x: f64, pan_y: f64, zoom: f64, container_w: f64, container_h: f64) -> (f64, f64) {
    // The zoomed content is container_w * zoom wide.
    // Maximum pan left: 0 (can't show whitespace on the left)
    // Maximum pan right: -(container_w * zoom - container_w) (right edge stays in view)
    let max_pan_x = 0.0;
    let min_pan_x = -(container_w * zoom - container_w);
    let max_pan_y = 0.0;
    let min_pan_y = -(container_h * zoom - container_h);

    let px = pan_x.clamp(min_pan_x, max_pan_x);
    let py = pan_y.clamp(min_pan_y, max_pan_y);
    (px, py)
}

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

    // Zoom/pan state (local to this component instance)
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

    let gun = *gun_pos.read();
    let target = *target_pos.read();
    let spotter = *spotter_pos.read();

    let svg_content = build_svg_content(gun, target, spotter, &selected_weapon, accuracy_radius_px);

    let svg_html = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none" style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:5;">{}</svg>"#,
        grid::MAP_WIDTH_PX, grid::MAP_HEIGHT_PX, svg_content
    );

    let current_zoom = *zoom.read();
    let current_pan_x = *pan_x.read();
    let current_pan_y = *pan_y.read();
    let dragging = *is_dragging.read();

    let transform_style = format!(
        "transform: translate({current_pan_x}px, {current_pan_y}px) scale({current_zoom}); transform-origin: 0 0;"
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

            // Scroll-wheel zoom
            onwheel: move |evt: Event<WheelData>| {
                evt.prevent_default();

                let delta = evt.data().delta();
                let delta_y = match delta {
                    WheelDelta::Pixels(d) => d.y,
                    WheelDelta::Lines(d) => d.y * 40.0,
                    WheelDelta::Pages(d) => d.y * 400.0,
                };

                let factor = if delta_y < 0.0 { 1.1 } else { 1.0 / 1.1 };
                let old_zoom = *zoom.read();
                let new_zoom = (old_zoom * factor).clamp(1.0, 10.0);

                if (new_zoom - old_zoom).abs() < 1e-9 {
                    return;
                }

                // Get cursor position relative to container
                let client = evt.data().client_coordinates();
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(el) = document.get_element_by_id(MAP_CONTAINER_ID) {
                        let rect = el.get_bounding_client_rect();
                        let cx = client.x - rect.left();
                        let cy = client.y - rect.top();

                        // The content point under cursor before zoom:
                        // content_x = (cx - pan_x) / old_zoom
                        // After zoom, we want the same content point under cursor:
                        // cx = content_x * new_zoom + new_pan_x
                        // => new_pan_x = cx - content_x * new_zoom
                        let old_pan_x = *pan_x.read();
                        let old_pan_y = *pan_y.read();

                        let content_x = (cx - old_pan_x) / old_zoom;
                        let content_y = (cy - old_pan_y) / old_zoom;

                        let new_pan_x = cx - content_x * new_zoom;
                        let new_pan_y = cy - content_y * new_zoom;

                        let (clamped_px, clamped_py) = clamp_pan(new_pan_x, new_pan_y, new_zoom, rect.width(), rect.height());

                        zoom.set(new_zoom);
                        pan_x.set(clamped_px);
                        pan_y.set(clamped_py);
                    }
                }
            },

            // Mouse down — start potential drag
            onmousedown: move |evt: Event<MouseData>| {
                let client = evt.client_coordinates();
                is_dragging.set(true);
                did_drag.set(false);
                drag_start_x.set(client.x);
                drag_start_y.set(client.y);
                drag_start_pan_x.set(*pan_x.read());
                drag_start_pan_y.set(*pan_y.read());
            },

            // Mouse move — update pan if dragging
            onmousemove: move |evt: Event<MouseData>| {
                if !*is_dragging.read() {
                    return;
                }

                let client = evt.client_coordinates();
                let dx = client.x - *drag_start_x.read();
                let dy = client.y - *drag_start_y.read();

                // Check drag threshold
                if !*did_drag.read() && (dx.abs() > DRAG_THRESHOLD || dy.abs() > DRAG_THRESHOLD) {
                    did_drag.set(true);
                }

                if *did_drag.read() {
                    let new_pan_x = *drag_start_pan_x.read() + dx;
                    let new_pan_y = *drag_start_pan_y.read() + dy;
                    let current_zoom = *zoom.read();

                    // Get container dimensions for clamping
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        if let Some(el) = document.get_element_by_id(MAP_CONTAINER_ID) {
                            let rect = el.get_bounding_client_rect();
                            let (clamped_px, clamped_py) = clamp_pan(new_pan_x, new_pan_y, current_zoom, rect.width(), rect.height());
                            pan_x.set(clamped_px);
                            pan_y.set(clamped_py);
                        }
                    }
                }
            },

            // Mouse up — end drag or place marker
            onmouseup: move |evt: Event<MouseData>| {
                let was_dragging = *is_dragging.read();
                let was_drag = *did_drag.read();
                is_dragging.set(false);

                if was_dragging && !was_drag {
                    // This was a click, not a drag — place a marker
                    let client = evt.client_coordinates();
                    let z = *zoom.read();
                    let px = *pan_x.read();
                    let py = *pan_y.read();
                    if let Some((img_x, img_y)) = coords::click_to_map_px_zoomed(
                        client.x, client.y, MAP_CONTAINER_ID, z, px, py,
                    ) {
                        match *placement_mode.read() {
                            PlacementMode::Gun => gun_pos.set(Some((img_x, img_y))),
                            PlacementMode::Target => target_pos.set(Some((img_x, img_y))),
                            PlacementMode::Spotter => spotter_pos.set(Some((img_x, img_y))),
                        }
                    }
                }
            },

            // Mouse leave — cancel drag
            onmouseleave: move |_| {
                is_dragging.set(false);
            },

            // Double-click — reset zoom and pan
            ondoubleclick: move |evt: Event<MouseData>| {
                evt.prevent_default();
                zoom.set(1.0);
                pan_x.set(0.0);
                pan_y.set(0.0);
            },

            // Inner wrapper with CSS transform for zoom/pan
            div {
                class: "map-inner",
                style: "{transform_style}",

                img {
                    src: "{image_url}",
                    draggable: "false",
                }

                // SVG overlay rendered as raw HTML for correct namespace handling
                div {
                    dangerous_inner_html: "{svg_html}",
                    style: "position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;",
                }
            }

            // Coordinate readout overlay (outside transform so it stays fixed)
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

use dioxus::prelude::*;
use foxhole_shared::grid;

use crate::api::WeaponData;
use crate::coords;

const MAP_CONTAINER_ID: &str = "artillery-map-container";

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

    let gun = *gun_pos.read();
    let target = *target_pos.read();
    let spotter = *spotter_pos.read();

    let svg_content = build_svg_content(gun, target, spotter, &selected_weapon, accuracy_radius_px);

    let svg_html = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none" style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:5;">{}</svg>"#,
        grid::MAP_WIDTH_PX, grid::MAP_HEIGHT_PX, svg_content
    );

    rsx! {
        div {
            id: MAP_CONTAINER_ID,
            class: "map-container",
            onclick: move |evt: Event<MouseData>| {
                let client = evt.client_coordinates();
                if let Some((px_x, px_y)) = coords::click_to_map_px(
                    client.x, client.y, MAP_CONTAINER_ID
                ) {
                    match *placement_mode.read() {
                        PlacementMode::Gun => gun_pos.set(Some((px_x, px_y))),
                        PlacementMode::Target => target_pos.set(Some((px_x, px_y))),
                        PlacementMode::Spotter => spotter_pos.set(Some((px_x, px_y))),
                    }
                }
            },

            img {
                src: "{image_url}",
                draggable: "false",
            }

            // SVG overlay rendered as raw HTML for correct namespace handling
            div {
                dangerous_inner_html: "{svg_html}",
                style: "position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;",
            }

            // Coordinate readout overlay
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

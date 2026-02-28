use foxhole_shared::grid;

/// Convert client (viewport) coordinates to container-relative pixel coordinates.
#[cfg(test)]
pub fn client_to_container(
    client_x: f64,
    client_y: f64,
    rect_left: f64,
    rect_top: f64,
) -> (f64, f64) {
    (client_x - rect_left, client_y - rect_top)
}

/// Pure function: convert container-relative coordinates to native map-image pixels,
/// undoing zoom/pan CSS transform. Usable in unit tests (no web_sys dependency).
///
/// Only `container_w` is needed because the image renders with `width:100%; height:auto`,
/// so both axes share the same scale factor (`MAP_WIDTH_PX / container_w`).
pub fn client_to_map_px_zoomed(
    container_x: f64,
    container_y: f64,
    container_w: f64,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
) -> Option<(f64, f64)> {
    if container_w <= 0.0 || zoom <= 0.0 {
        return None;
    }

    // Undo CSS transform: translate(pan_x, pan_y) scale(zoom)
    let rendered_x = (container_x - pan_x) / zoom;
    let rendered_y = (container_y - pan_y) / zoom;

    // Convert from rendered size to native image pixels.
    // The image preserves aspect ratio (width:100%, height:auto),
    // so both axes use the same scale factor.
    let scale = grid::MAP_WIDTH_PX / container_w;
    let img_x = (rendered_x * scale).clamp(0.0, grid::MAP_WIDTH_PX);
    let img_y = (rendered_y * scale).clamp(0.0, grid::MAP_HEIGHT_PX);

    Some((img_x, img_y))
}

/// Get container-relative click coordinates using web_sys, then convert
/// from rendered pixel space to map-image pixel space, undoing zoom/pan transform.
pub fn click_to_map_px_zoomed(
    client_x: f64,
    client_y: f64,
    container_id: &str,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
) -> Option<(f64, f64)> {
    let document = web_sys::window()?.document()?;
    let element = document.get_element_by_id(container_id)?;
    let rect = element.get_bounding_client_rect();

    let container_x = client_x - rect.left();
    let container_y = client_y - rect.top();

    client_to_map_px_zoomed(container_x, container_y, rect.width(), zoom, pan_x, pan_y)
}

/// Convert map-image pixel coordinates to meters.
pub fn map_px_to_meters(px_x: f64, px_y: f64) -> (f64, f64) {
    grid::px_to_meters(px_x, px_y)
}

/// Convert meter coordinates to map-image pixel coordinates.
pub fn meters_to_map_px(m_x: f64, m_y: f64) -> (f64, f64) {
    grid::meters_to_px(m_x, m_y)
}

/// Format pixel position as grid coordinate string.
pub fn format_px_as_grid(px_x: f64, px_y: f64) -> String {
    let (mx, my) = grid::px_to_meters(px_x, px_y);
    grid::format_grid_coord(mx, my)
}

/// Convert a meter distance to pixels in the native image space.
pub fn meters_to_image_px(meters: f64) -> f64 {
    grid::meters_to_px_distance(meters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_to_container_origin() {
        let (x, y) = client_to_container(100.0, 200.0, 100.0, 200.0);
        assert!((x - 0.0).abs() < 1e-9);
        assert!((y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_client_to_container_offset() {
        let (x, y) = client_to_container(450.0, 350.0, 320.0, 50.0);
        assert!((x - 130.0).abs() < 1e-9);
        assert!((y - 300.0).abs() < 1e-9);
    }

    #[test]
    fn test_map_px_to_meters_and_back() {
        let (mx, my) = map_px_to_meters(512.0, 444.0);
        let (px, py) = meters_to_map_px(mx, my);
        assert!((px - 512.0).abs() < 0.01);
        assert!((py - 444.0).abs() < 0.01);
    }

    #[test]
    fn test_format_px_as_grid_top_left() {
        let coord = format_px_as_grid(1.0, 1.0);
        assert_eq!(coord, "A1k7");
    }

    #[test]
    fn test_format_px_as_grid_center() {
        let coord = format_px_as_grid(1024.0, 888.0);
        assert!(coord.starts_with('I'));
    }

    #[test]
    fn test_meters_to_image_px_sanity() {
        let px = meters_to_image_px(100.0);
        // ~94 pixels for 100m
        assert!(px > 85.0 && px < 105.0);
    }

    #[test]
    fn test_client_to_map_px_zoomed_no_zoom() {
        // At zoom=1, pan=0, should behave like the unzoomed version
        let container_w = 800.0;
        let result = client_to_map_px_zoomed(400.0, 346.875, container_w, 1.0, 0.0, 0.0);
        let (x, y) = result.unwrap();
        assert!((x - 1024.0).abs() < 1.0);
        assert!((y - 888.0).abs() < 1.0);
    }

    #[test]
    fn test_client_to_map_px_zoomed_with_zoom() {
        // At zoom=2 with pan=0, clicking at (400, 347) should map to (512, 444) in image space
        let container_w = 800.0;
        let result = client_to_map_px_zoomed(400.0, 346.875, container_w, 2.0, 0.0, 0.0);
        let (x, y) = result.unwrap();
        assert!((x - 512.0).abs() < 1.0);
        assert!((y - 444.0).abs() < 1.0);
    }

    #[test]
    fn test_client_to_map_px_zoomed_with_pan() {
        // At zoom=1 with pan=(100, 50), clicking at (500, 397) should map to same as (400, 347) unzoomed
        let container_w = 800.0;
        let result = client_to_map_px_zoomed(500.0, 396.875, container_w, 1.0, 100.0, 50.0);
        let (x, y) = result.unwrap();
        assert!((x - 1024.0).abs() < 1.0);
        assert!((y - 888.0).abs() < 1.0);
    }

    #[test]
    fn test_client_to_map_px_zoomed_clamps() {
        let container_w = 800.0;
        // Click far outside (negative after undo) should clamp to 0
        let result = client_to_map_px_zoomed(-100.0, -100.0, container_w, 1.0, 0.0, 0.0);
        let (x, y) = result.unwrap();
        assert!((x - 0.0).abs() < 0.01);
        assert!((y - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_client_to_map_px_zoomed_invalid_container() {
        let result = client_to_map_px_zoomed(400.0, 300.0, 0.0, 1.0, 0.0, 0.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_client_to_map_px_zoomed_container_taller_than_image() {
        // Container is 800×900 but image renders at 800×693.75.
        // The conversion should be the same regardless of container height,
        // because only container_w matters (image uses width:100% height:auto).
        let container_w = 800.0;
        // Click at (400, 346.875) — center of the image
        let result = client_to_map_px_zoomed(400.0, 346.875, container_w, 1.0, 0.0, 0.0);
        let (x, y) = result.unwrap();
        assert!((x - 1024.0).abs() < 1.0);
        assert!((y - 888.0).abs() < 1.0);
    }
}

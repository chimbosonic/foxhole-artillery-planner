/// Foxhole map grid system.
///
/// Each hex region is 2184m wide x 1890m tall.
/// Grid: 17 columns (A-Q) x 15 rows (1-15).
/// Each grid cell has a 3x3 keypad sub-grid (k1-k9).
/// Map images are 1024x888 pixels.
// World dimensions in meters
pub const MAP_WIDTH_M: f64 = 2184.0;
pub const MAP_HEIGHT_M: f64 = 1890.0;

// Map image dimensions in pixels
pub const MAP_WIDTH_PX: f64 = 1024.0;
pub const MAP_HEIGHT_PX: f64 = 888.0;

// Grid dimensions
pub const GRID_COLS: usize = 17; // A through Q
pub const GRID_ROWS: usize = 15; // 1 through 15

// Derived constants
pub const GRID_CELL_WIDTH_M: f64 = MAP_WIDTH_M / GRID_COLS as f64; // ~128.5m
pub const GRID_CELL_HEIGHT_M: f64 = MAP_HEIGHT_M / GRID_ROWS as f64; // 126.0m

pub const METERS_PER_PIXEL_X: f64 = MAP_WIDTH_M / MAP_WIDTH_PX;
pub const METERS_PER_PIXEL_Y: f64 = MAP_HEIGHT_M / MAP_HEIGHT_PX;

/// Convert pixel coordinates to meter coordinates.
pub fn px_to_meters(px_x: f64, px_y: f64) -> (f64, f64) {
    (px_x * METERS_PER_PIXEL_X, px_y * METERS_PER_PIXEL_Y)
}

/// Convert meter coordinates to pixel coordinates.
pub fn meters_to_px(m_x: f64, m_y: f64) -> (f64, f64) {
    (m_x / METERS_PER_PIXEL_X, m_y / METERS_PER_PIXEL_Y)
}

/// Convert a meter distance to pixel distance (using average scale).
pub fn meters_to_px_distance(meters: f64) -> f64 {
    let avg_scale = (MAP_WIDTH_PX / MAP_WIDTH_M + MAP_HEIGHT_PX / MAP_HEIGHT_M) / 2.0;
    meters * avg_scale
}

/// Column letter for a given column index (0-based). A=0, Q=16.
pub fn col_letter(col: usize) -> char {
    (b'A' + col as u8) as char
}

/// Format a meter position as a Foxhole grid coordinate (e.g., "G9k3").
pub fn format_grid_coord(m_x: f64, m_y: f64) -> String {
    // Clamp to map bounds
    let m_x = m_x.clamp(0.0, MAP_WIDTH_M - 0.01);
    let m_y = m_y.clamp(0.0, MAP_HEIGHT_M - 0.01);

    let col = (m_x / GRID_CELL_WIDTH_M) as usize;
    let row = (m_y / GRID_CELL_HEIGHT_M) as usize;

    let col = col.min(GRID_COLS - 1);
    let row = row.min(GRID_ROWS - 1);

    // Sub-grid position within the cell (0.0 to 1.0)
    let sub_x = (m_x - col as f64 * GRID_CELL_WIDTH_M) / GRID_CELL_WIDTH_M;
    let sub_y = (m_y - row as f64 * GRID_CELL_HEIGHT_M) / GRID_CELL_HEIGHT_M;

    // Keypad mapping: 3x3 grid, numpad layout
    // k7 k8 k9  (top)
    // k4 k5 k6  (middle)
    // k1 k2 k3  (bottom)
    let kx = (sub_x * 3.0) as usize; // 0, 1, 2 = left, center, right
    let ky = (sub_y * 3.0) as usize; // 0, 1, 2 = top, middle, bottom

    let kx = kx.min(2);
    let ky = ky.min(2);

    // Convert to keypad number: top-left=7, bottom-right=3
    let keypad = match (kx, ky) {
        (0, 0) => 7,
        (1, 0) => 8,
        (2, 0) => 9,
        (0, 1) => 4,
        (1, 1) => 5,
        (2, 1) => 6,
        (0, 2) => 1,
        (1, 2) => 2,
        (2, 2) => 3,
        _ => 5,
    };

    format!("{}{}k{}", col_letter(col), row + 1, keypad)
}

/// Get the pixel X position for a grid column line (0-based column index).
pub fn grid_col_px(col: usize) -> f64 {
    col as f64 * (MAP_WIDTH_PX / GRID_COLS as f64)
}

/// Get the pixel Y position for a grid row line (0-based row index).
pub fn grid_row_px(row: usize) -> f64 {
    row as f64 * (MAP_HEIGHT_PX / GRID_ROWS as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_px_to_meters_origin() {
        let (mx, my) = px_to_meters(0.0, 0.0);
        assert!((mx - 0.0).abs() < 1e-9);
        assert!((my - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_px_to_meters_corner() {
        let (mx, my) = px_to_meters(MAP_WIDTH_PX, MAP_HEIGHT_PX);
        assert!((mx - MAP_WIDTH_M).abs() < 0.1);
        assert!((my - MAP_HEIGHT_M).abs() < 0.1);
    }

    #[test]
    fn test_meters_to_px_roundtrip() {
        let (mx, my) = px_to_meters(512.0, 444.0);
        let (px, py) = meters_to_px(mx, my);
        assert!((px - 512.0).abs() < 0.01);
        assert!((py - 444.0).abs() < 0.01);
    }

    #[test]
    fn test_col_letter() {
        assert_eq!(col_letter(0), 'A');
        assert_eq!(col_letter(6), 'G');
        assert_eq!(col_letter(16), 'Q');
    }

    #[test]
    fn test_format_grid_coord_top_left() {
        // Top-left corner should be A1k7
        let coord = format_grid_coord(1.0, 1.0);
        assert_eq!(coord, "A1k7");
    }

    #[test]
    fn test_format_grid_coord_center() {
        // Center of the map: col 8 (I), row 7-8
        let cx = MAP_WIDTH_M / 2.0;
        let cy = MAP_HEIGHT_M / 2.0;
        let coord = format_grid_coord(cx, cy);
        // Should be around I8 area
        assert!(coord.starts_with('I'));
    }

    #[test]
    fn test_format_grid_coord_bottom_right() {
        // Near bottom-right should be Q15k3
        let coord = format_grid_coord(MAP_WIDTH_M - 1.0, MAP_HEIGHT_M - 1.0);
        assert_eq!(coord, "Q15k3");
    }

    #[test]
    fn test_format_grid_coord_keypad_layout() {
        // Test specific keypad positions within cell A1
        let cw = GRID_CELL_WIDTH_M;
        let ch = GRID_CELL_HEIGHT_M;

        // Top-left of A1 = k7
        assert_eq!(format_grid_coord(1.0, 1.0), "A1k7");
        // Top-center of A1 = k8
        assert_eq!(format_grid_coord(cw / 2.0, 1.0), "A1k8");
        // Top-right of A1 = k9
        assert_eq!(format_grid_coord(cw - 1.0, 1.0), "A1k9");
        // Middle-left of A1 = k4
        assert_eq!(format_grid_coord(1.0, ch / 2.0), "A1k4");
        // Center of A1 = k5
        assert_eq!(format_grid_coord(cw / 2.0, ch / 2.0), "A1k5");
        // Bottom-right of A1 = k3
        assert_eq!(format_grid_coord(cw - 1.0, ch - 1.0), "A1k3");
    }

    #[test]
    fn test_grid_col_px() {
        assert!((grid_col_px(0) - 0.0).abs() < 1e-9);
        let expected = MAP_WIDTH_PX / GRID_COLS as f64;
        assert!((grid_col_px(1) - expected).abs() < 0.01);
    }

    #[test]
    fn test_grid_row_px() {
        assert!((grid_row_px(0) - 0.0).abs() < 1e-9);
        let expected = MAP_HEIGHT_PX / GRID_ROWS as f64;
        assert!((grid_row_px(1) - expected).abs() < 0.01);
    }

    #[test]
    fn test_meters_to_px_distance() {
        // 100m should convert to a reasonable pixel distance
        let px = meters_to_px_distance(100.0);
        assert!(px > 40.0 && px < 60.0); // roughly 47 pixels
    }
}

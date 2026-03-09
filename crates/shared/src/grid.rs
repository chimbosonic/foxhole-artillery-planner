/// Foxhole map grid system.
///
/// Each hex region is 2184m wide x 1890m tall.
/// Grid: 17 columns (A-Q) x 15 rows (1-15), each cell exactly 125m x 125m.
/// The grid starts at map origin and does NOT span the full region — there's
/// unused space past column Q (2125m) and row 15 (1875m).
/// Each grid cell has a 3x3 keypad sub-grid (k1-k9).
/// Map images are 2048x1776 pixels.
// World dimensions in meters
pub const MAP_WIDTH_M: f64 = 2184.0;
pub const MAP_HEIGHT_M: f64 = 1890.0;

// Map image dimensions in pixels
pub const MAP_WIDTH_PX: f64 = 2048.0;
pub const MAP_HEIGHT_PX: f64 = 1776.0;

// Grid dimensions
pub const GRID_COLS: usize = 17; // A through Q
pub const GRID_ROWS: usize = 15; // 1 through 15

// Small inset to keep exact-boundary positions inside the last grid cell/keypad
const BOUNDARY_EPSILON: f64 = 0.01;

// Grid cell size — empirically verified as exactly 125m × 125m
pub const GRID_CELL_SIZE_M: f64 = 125.0;

// Total grid extent in meters (smaller than the full map)
const GRID_WIDTH_M: f64 = GRID_COLS as f64 * GRID_CELL_SIZE_M; // 2125m
const GRID_HEIGHT_M: f64 = GRID_ROWS as f64 * GRID_CELL_SIZE_M; // 1875m

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
    // Clamp to grid bounds (not full map bounds)
    let m_x = m_x.clamp(0.0, GRID_WIDTH_M - BOUNDARY_EPSILON);
    let m_y = m_y.clamp(0.0, GRID_HEIGHT_M - BOUNDARY_EPSILON);

    let col = (m_x / GRID_CELL_SIZE_M) as usize;
    let row = (m_y / GRID_CELL_SIZE_M) as usize;

    let col = col.min(GRID_COLS - 1);
    let row = row.min(GRID_ROWS - 1);

    // Sub-grid position within the cell (0.0 to 1.0)
    let sub_x = (m_x - col as f64 * GRID_CELL_SIZE_M) / GRID_CELL_SIZE_M;
    let sub_y = (m_y - row as f64 * GRID_CELL_SIZE_M) / GRID_CELL_SIZE_M;

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
        _ => unreachable!("kx and ky are clamped to 0..=2"),
    };

    format!("{}{}k{}", col_letter(col), row + 1, keypad)
}

/// Get the pixel X position for a grid column line (0-based column index).
pub fn grid_col_px(col: usize) -> f64 {
    (col as f64 * GRID_CELL_SIZE_M) / METERS_PER_PIXEL_X
}

/// Get the pixel Y position for a grid row line (0-based row index).
pub fn grid_row_px(row: usize) -> f64 {
    (row as f64 * GRID_CELL_SIZE_M) / METERS_PER_PIXEL_Y
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
        // Center of the grid: 2125/2 = 1062.5m, 1875/2 = 937.5m
        let cx = GRID_WIDTH_M / 2.0;
        let cy = GRID_HEIGHT_M / 2.0;
        let coord = format_grid_coord(cx, cy);
        // col = 1062.5 / 125 = 8 (I), row = 937.5 / 125 = 7 (row 8)
        assert!(coord.starts_with('I'));
        assert!(coord.contains("8k"));
    }

    #[test]
    fn test_format_grid_coord_bottom_right() {
        // Near bottom-right of grid should be Q15k3
        let coord = format_grid_coord(GRID_WIDTH_M - 1.0, GRID_HEIGHT_M - 1.0);
        assert_eq!(coord, "Q15k3");
    }

    #[test]
    fn test_format_grid_coord_past_grid() {
        // Positions past the grid boundary clamp to Q15k3
        let coord = format_grid_coord(MAP_WIDTH_M - 1.0, MAP_HEIGHT_M - 1.0);
        assert_eq!(coord, "Q15k3");
    }

    #[test]
    fn test_format_grid_coord_keypad_layout() {
        // Test specific keypad positions within cell A1
        let cs = GRID_CELL_SIZE_M;

        // Top-left of A1 = k7
        assert_eq!(format_grid_coord(1.0, 1.0), "A1k7");
        // Top-center of A1 = k8
        assert_eq!(format_grid_coord(cs / 2.0, 1.0), "A1k8");
        // Top-right of A1 = k9
        assert_eq!(format_grid_coord(cs - 1.0, 1.0), "A1k9");
        // Middle-left of A1 = k4
        assert_eq!(format_grid_coord(1.0, cs / 2.0), "A1k4");
        // Center of A1 = k5
        assert_eq!(format_grid_coord(cs / 2.0, cs / 2.0), "A1k5");
        // Bottom-right of A1 = k3
        assert_eq!(format_grid_coord(cs - 1.0, cs - 1.0), "A1k3");
    }

    #[test]
    fn test_grid_col_px() {
        assert!((grid_col_px(0) - 0.0).abs() < 1e-9);
        let expected = GRID_CELL_SIZE_M / METERS_PER_PIXEL_X;
        assert!((grid_col_px(1) - expected).abs() < 0.01);
        // Last column line should be less than full image width
        assert!(grid_col_px(GRID_COLS) < MAP_WIDTH_PX);
    }

    #[test]
    fn test_grid_row_px() {
        assert!((grid_row_px(0) - 0.0).abs() < 1e-9);
        let expected = GRID_CELL_SIZE_M / METERS_PER_PIXEL_Y;
        assert!((grid_row_px(1) - expected).abs() < 0.01);
        // Last row line should be less than full image height
        assert!(grid_row_px(GRID_ROWS) < MAP_HEIGHT_PX);
    }

    #[test]
    fn test_meters_to_px_distance() {
        // 100m should convert to a reasonable pixel distance
        let px = meters_to_px_distance(100.0);
        assert!(px > 85.0 && px < 105.0); // roughly 94 pixels
    }
}

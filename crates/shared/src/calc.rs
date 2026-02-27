use crate::models::{FiringSolution, Position, Weapon, WindInput};

/// Meters per wind strength level for lateral drift.
const WIND_DRIFT_PER_LEVEL: f64 = 8.0;

/// Euclidean distance between two positions.
pub fn distance(a: Position, b: Position) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

/// Compass azimuth from `gun` to `target` in degrees [0, 360).
/// Coordinate system: X = east, Y = south (screen coords).
/// Azimuth is clockwise from north.
pub fn azimuth(gun: Position, target: Position) -> f64 {
    let dx = target.x - gun.x;
    let dy = target.y - gun.y;
    // atan2(east, south-negated) gives clockwise-from-north
    let rad = dx.atan2(-dy);
    let deg = rad.to_degrees();
    if deg < 0.0 { deg + 360.0 } else { deg }
}

/// Interpolate accuracy radius for a given distance.
/// acc_radius[0] at min_range, acc_radius[1] at max_range.
pub fn accuracy_radius(weapon: &Weapon, dist: f64) -> f64 {
    let range_span = weapon.max_range - weapon.min_range;
    if range_span <= 0.0 {
        return weapon.acc_radius[0];
    }
    let t = ((dist - weapon.min_range) / range_span).clamp(0.0, 1.0);
    weapon.acc_radius[0] + t * (weapon.acc_radius[1] - weapon.acc_radius[0])
}

/// Compute the wind offset vector in meters (dx_wind, dy_wind).
/// Wind direction is where the wind blows FROM (compass degrees).
/// The drift is perpendicular-ish: wind pushes shells in the direction the wind blows TO.
fn wind_offset(wind: &WindInput) -> (f64, f64) {
    let strength_m = wind.strength as f64 * WIND_DRIFT_PER_LEVEL;
    // Wind blows FROM `direction`, so shells drift TOWARD the opposite direction.
    // Convert "blows from" to "pushes to": add 180 degrees.
    let push_dir = (wind.direction + 180.0) % 360.0;
    let rad = push_dir.to_radians();
    // Convert compass bearing to vector: north=negative Y, east=positive X
    let dx = rad.sin() * strength_m;
    let dy = -rad.cos() * strength_m;
    (dx, dy)
}

/// Compute a full firing solution.
pub fn firing_solution(
    gun: Position,
    target: Position,
    weapon: &Weapon,
    wind: Option<&WindInput>,
) -> FiringSolution {
    let dist = distance(gun, target);
    let az = azimuth(gun, target);
    let in_range = dist >= weapon.min_range && dist <= weapon.max_range;
    let acc = accuracy_radius(weapon, dist);

    let (wind_adjusted_azimuth, wind_adjusted_distance, wind_offset_meters) = match wind {
        Some(w) if w.strength > 0 => {
            let (wdx, wdy) = wind_offset(w);
            let drift_magnitude = (wdx * wdx + wdy * wdy).sqrt();
            // To compensate for wind drift, aim at target - wind_offset
            let compensated = Position {
                x: target.x - wdx,
                y: target.y - wdy,
            };
            let adj_az = azimuth(gun, compensated);
            let adj_dist = distance(gun, compensated);
            (Some(adj_az), Some(adj_dist), Some(drift_magnitude))
        }
        _ => (None, None, None),
    };

    FiringSolution {
        azimuth: az,
        distance: dist,
        in_range,
        accuracy_radius: acc,
        wind_adjusted_azimuth,
        wind_adjusted_distance,
        wind_offset_meters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Faction;

    fn test_weapon() -> Weapon {
        Weapon {
            faction: Faction::Both,
            display_name: "Test Gun".to_string(),
            min_range: 100.0,
            max_range: 300.0,
            acc_radius: [10.0, 30.0],
        }
    }

    #[test]
    fn test_distance_horizontal() {
        let a = Position { x: 0.0, y: 0.0 };
        let b = Position { x: 100.0, y: 0.0 };
        assert!((distance(a, b) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_distance_diagonal() {
        let a = Position { x: 0.0, y: 0.0 };
        let b = Position { x: 3.0, y: 4.0 };
        assert!((distance(a, b) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_azimuth_north() {
        // Target is directly north (negative Y in screen coords)
        let gun = Position { x: 100.0, y: 100.0 };
        let target = Position { x: 100.0, y: 0.0 };
        assert!((azimuth(gun, target) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_azimuth_east() {
        let gun = Position { x: 0.0, y: 0.0 };
        let target = Position { x: 100.0, y: 0.0 };
        assert!((azimuth(gun, target) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn test_azimuth_south() {
        let gun = Position { x: 0.0, y: 0.0 };
        let target = Position { x: 0.0, y: 100.0 };
        assert!((azimuth(gun, target) - 180.0).abs() < 1e-9);
    }

    #[test]
    fn test_azimuth_west() {
        let gun = Position { x: 100.0, y: 0.0 };
        let target = Position { x: 0.0, y: 0.0 };
        assert!((azimuth(gun, target) - 270.0).abs() < 1e-9);
    }

    #[test]
    fn test_accuracy_at_min_range() {
        let w = test_weapon();
        assert!((accuracy_radius(&w, 100.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_accuracy_at_max_range() {
        let w = test_weapon();
        assert!((accuracy_radius(&w, 300.0) - 30.0).abs() < 1e-9);
    }

    #[test]
    fn test_accuracy_midpoint() {
        let w = test_weapon();
        assert!((accuracy_radius(&w, 200.0) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_firing_solution_no_wind() {
        let gun = Position { x: 0.0, y: 0.0 };
        let target = Position { x: 0.0, y: -200.0 };
        let w = test_weapon();
        let sol = firing_solution(gun, target, &w, None);
        assert!((sol.azimuth - 0.0).abs() < 1e-6);
        assert!((sol.distance - 200.0).abs() < 1e-6);
        assert!(sol.in_range);
        assert!(sol.wind_adjusted_azimuth.is_none());
    }

    #[test]
    fn test_firing_solution_out_of_range() {
        let gun = Position { x: 0.0, y: 0.0 };
        let target = Position { x: 500.0, y: 0.0 };
        let w = test_weapon();
        let sol = firing_solution(gun, target, &w, None);
        assert!(!sol.in_range);
    }

    #[test]
    fn test_firing_solution_with_wind() {
        let gun = Position { x: 0.0, y: 0.0 };
        let target = Position { x: 0.0, y: -200.0 };
        let w = test_weapon();
        let wind = WindInput {
            direction: 270.0, // wind from west
            strength: 3,
        };
        let sol = firing_solution(gun, target, &w, Some(&wind));
        // Wind from west pushes east, so compensation aims west of target
        assert!(sol.wind_adjusted_azimuth.is_some());
        let adj_az = sol.wind_adjusted_azimuth.unwrap();
        // Original azimuth is 0 (north), wind pushes east so compensate by aiming slightly west (< 360)
        assert!(adj_az > 350.0 || adj_az < 10.0); // roughly north, slightly west
    }

    #[test]
    fn test_wind_offset_strength_zero() {
        let gun = Position { x: 0.0, y: 0.0 };
        let target = Position { x: 200.0, y: 0.0 };
        let w = test_weapon();
        let wind = WindInput {
            direction: 0.0,
            strength: 0,
        };
        let sol = firing_solution(gun, target, &w, Some(&wind));
        assert!(sol.wind_adjusted_azimuth.is_none());
    }
}

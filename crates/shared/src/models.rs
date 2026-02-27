use serde::{Deserialize, Serialize};

#[cfg(feature = "uuid-support")]
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Colonial,
    Warden,
    Both,
}

impl std::fmt::Display for Faction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Faction::Colonial => write!(f, "Colonial"),
            Faction::Warden => write!(f, "Warden"),
            Faction::Both => write!(f, "Both"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weapon {
    pub faction: Faction,
    pub display_name: String,
    pub min_range: f64,
    pub max_range: f64,
    pub acc_radius: [f64; 2],
    pub wind_drift: [f64; 2],
}

impl Weapon {
    /// Generate a URL-safe slug from the display name.
    pub fn slug(&self) -> String {
        self.display_name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameMap {
    #[serde(rename = "type")]
    pub image_type: String,
    pub display_name: String,
    pub file_name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[cfg(feature = "uuid-support")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub name: String,
    pub map_id: String,
    #[serde(default)]
    pub weapon_ids: Vec<String>,
    /// Legacy single-position field for backwards-compatible deserialization.
    #[serde(default, skip_serializing)]
    pub gun_position: Option<Position>,
    #[serde(default, skip_serializing)]
    pub target_position: Option<Position>,
    #[serde(default, skip_serializing)]
    pub spotter_position: Option<Position>,
    /// Multi-position fields (new canonical format).
    #[serde(default)]
    pub gun_positions: Vec<Position>,
    #[serde(default)]
    pub target_positions: Vec<Position>,
    #[serde(default)]
    pub spotter_positions: Vec<Position>,
    /// Explicit gun→target pairing: one entry per gun, `Some(idx)` = paired with target at that index.
    #[serde(default)]
    pub gun_target_indices: Vec<Option<u32>>,
    pub wind_direction: Option<f64>,
    pub wind_strength: u8,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "uuid-support")]
impl Plan {
    /// Promote legacy single-position fields into the Vec fields if the Vecs
    /// are empty. Call this after deserializing old plans.
    pub fn migrate(&mut self) {
        if self.gun_positions.is_empty() {
            if let Some(pos) = self.gun_position.take() {
                self.gun_positions.push(pos);
            }
        }
        if self.target_positions.is_empty() {
            if let Some(pos) = self.target_position.take() {
                self.target_positions.push(pos);
            }
        }
        if self.spotter_positions.is_empty() {
            if let Some(pos) = self.spotter_position.take() {
                self.spotter_positions.push(pos);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindInput {
    pub direction: f64,
    pub strength: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiringSolution {
    pub azimuth: f64,
    pub distance: f64,
    pub in_range: bool,
    pub accuracy_radius: f64,
    pub wind_adjusted_azimuth: Option<f64>,
    pub wind_adjusted_distance: Option<f64>,
    pub wind_offset_meters: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gun_target_indices_serialization_roundtrip() {
        let indices: Vec<Option<u32>> = vec![Some(0), None, Some(2)];
        let json = serde_json::to_string(&indices).unwrap();
        assert_eq!(json, "[0,null,2]");
        let parsed: Vec<Option<u32>> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, indices);
    }

    #[test]
    fn test_gun_target_indices_empty_default() {
        // Simulates an old plan JSON without the gun_target_indices field
        #[derive(Deserialize)]
        struct PartialPlan {
            #[serde(default)]
            gun_target_indices: Vec<Option<u32>>,
        }
        let json = r#"{}"#;
        let plan: PartialPlan = serde_json::from_str(json).unwrap();
        assert!(plan.gun_target_indices.is_empty());
    }

    #[test]
    fn test_gun_target_indices_with_values() {
        #[derive(Deserialize)]
        struct PartialPlan {
            #[serde(default)]
            gun_target_indices: Vec<Option<u32>>,
        }
        let json = r#"{"gun_target_indices":[0,null,1]}"#;
        let plan: PartialPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.gun_target_indices, vec![Some(0), None, Some(1)]);
    }

    #[test]
    fn test_weapon_slug_generation() {
        let weapon = Weapon {
            faction: Faction::Colonial,
            display_name: "Storm Cannon".to_string(),
            min_range: 400.0,
            max_range: 1000.0,
            acc_radius: [50.0, 50.0],
            wind_drift: [20.0, 50.0],
        };
        assert_eq!(weapon.slug(), "storm-cannon");
    }
}

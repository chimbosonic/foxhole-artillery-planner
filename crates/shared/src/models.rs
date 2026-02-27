use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub name: String,
    pub map_id: String,
    pub weapon_id: String,
    pub gun_position: Option<Position>,
    pub target_position: Option<Position>,
    pub spotter_position: Option<Position>,
    pub wind_direction: Option<f64>,
    pub wind_strength: u8,
    pub created_at: String,
    pub updated_at: String,
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

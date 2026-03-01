use foxhole_shared::models::{GameMap, Weapon};
use std::path::Path;

pub struct Assets {
    pub maps: Vec<GameMap>,
    pub weapons: Vec<Weapon>,
}

impl Assets {
    pub fn load(assets_dir: &Path) -> Self {
        let maps_path = assets_dir.join("maps.json");
        let weapons_path = assets_dir.join("weapons.json");

        let maps_data = std::fs::read_to_string(&maps_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", maps_path.display(), e));
        let weapons_data = std::fs::read_to_string(&weapons_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", weapons_path.display(), e));

        let maps: Vec<GameMap> = serde_json::from_str(&maps_data)
            .unwrap_or_else(|e| panic!("Failed to parse maps.json: {}", e));
        let weapons: Vec<Weapon> = serde_json::from_str(&weapons_data)
            .unwrap_or_else(|e| panic!("Failed to parse weapons.json: {}", e));

        println!("Loaded {} maps, {} weapons", maps.len(), weapons.len());

        Assets { maps, weapons }
    }

    pub fn find_weapon_by_slug(&self, slug: &str) -> Option<&Weapon> {
        self.weapons.iter().find(|w| w.slug() == slug)
    }

    pub fn find_map_by_file_name(&self, file_name: &str) -> Option<&GameMap> {
        self.maps.iter().find(|m| m.file_name == file_name)
    }
}

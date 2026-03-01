use foxhole_shared::models::Plan;
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const PLANS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("plans");
const GUN_PLACEMENTS_TABLE: TableDefinition<&str, u64> = TableDefinition::new("gun_placements");
const MARKER_PLACEMENTS_TABLE: TableDefinition<&str, u64> =
    TableDefinition::new("marker_placements");

pub struct Storage {
    db: Database,
    path: PathBuf,
}

impl Storage {
    pub fn open(path: &Path) -> Arc<Self> {
        let db = Database::create(path)
            .unwrap_or_else(|e| panic!("Failed to open database at {}: {}", path.display(), e));

        // Ensure tables exist
        let write_txn = db.begin_write().expect("Failed to begin write txn");
        {
            let _ = write_txn.open_table(PLANS_TABLE);
            let _ = write_txn.open_table(GUN_PLACEMENTS_TABLE);
            let _ = write_txn.open_table(MARKER_PLACEMENTS_TABLE);
        }
        write_txn.commit().expect("Failed to commit initial txn");

        Arc::new(Storage {
            db,
            path: path.to_path_buf(),
        })
    }

    pub fn save_plan(&self, plan: &Plan) -> Result<(), String> {
        let json = serde_json::to_vec(plan).map_err(|e| e.to_string())?;
        let id_str = plan.id.to_string();

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(PLANS_TABLE)
                .map_err(|e| e.to_string())?;
            table
                .insert(id_str.as_str(), json.as_slice())
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_plan(&self, id: &str) -> Result<Option<Plan>, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(PLANS_TABLE)
            .map_err(|e| e.to_string())?;

        match table.get(id).map_err(|e| e.to_string())? {
            Some(value) => {
                let mut plan: Plan =
                    serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;
                plan.migrate();
                Ok(Some(plan))
            }
            None => Ok(None),
        }
    }

    pub fn count_plans(&self) -> Result<u64, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(PLANS_TABLE)
            .map_err(|e| e.to_string())?;
        table.len().map_err(|e| e.to_string())
    }

    pub fn db_size_bytes(&self) -> Result<u64, String> {
        std::fs::metadata(&self.path)
            .map(|m| m.len())
            .map_err(|e| e.to_string())
    }

    pub fn increment_gun_placement(&self, weapon_slug: &str) -> Result<(), String> {
        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(GUN_PLACEMENTS_TABLE)
                .map_err(|e| e.to_string())?;
            let current = table
                .get(weapon_slug)
                .map_err(|e| e.to_string())?
                .map(|v| v.value())
                .unwrap_or(0);
            table
                .insert(weapon_slug, current + 1)
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn increment_marker_placement(&self, kind: &str) -> Result<(), String> {
        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(MARKER_PLACEMENTS_TABLE)
                .map_err(|e| e.to_string())?;
            let current = table
                .get(kind)
                .map_err(|e| e.to_string())?
                .map(|v| v.value())
                .unwrap_or(0);
            table.insert(kind, current + 1).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_marker_placement_count(&self, kind: &str) -> Result<u64, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(MARKER_PLACEMENTS_TABLE)
            .map_err(|e| e.to_string())?;
        Ok(table
            .get(kind)
            .map_err(|e| e.to_string())?
            .map(|v| v.value())
            .unwrap_or(0))
    }

    pub fn get_gun_placement_counts(&self) -> Result<Vec<(String, u64)>, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(GUN_PLACEMENTS_TABLE)
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for entry in table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            result.push((key.value().to_string(), value.value()));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_storage() -> (Arc<Storage>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.redb");
        let storage = Storage::open(&path);
        (storage, dir)
    }

    #[test]
    fn test_increment_gun_placement_new_slug() {
        let (storage, _dir) = temp_storage();
        storage.increment_gun_placement("mortar").unwrap();
        let counts = storage.get_gun_placement_counts().unwrap();
        assert_eq!(counts, vec![("mortar".to_string(), 1)]);
    }

    #[test]
    fn test_increment_gun_placement_accumulates() {
        let (storage, _dir) = temp_storage();
        for _ in 0..3 {
            storage.increment_gun_placement("storm-cannon").unwrap();
        }
        let counts = storage.get_gun_placement_counts().unwrap();
        assert_eq!(counts, vec![("storm-cannon".to_string(), 3)]);
    }

    #[test]
    fn test_increment_multiple_slugs() {
        let (storage, _dir) = temp_storage();
        storage.increment_gun_placement("mortar").unwrap();
        storage.increment_gun_placement("mortar").unwrap();
        storage.increment_gun_placement("storm-cannon").unwrap();
        let mut counts = storage.get_gun_placement_counts().unwrap();
        counts.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            counts,
            vec![("mortar".to_string(), 2), ("storm-cannon".to_string(), 1),]
        );
    }

    #[test]
    fn test_get_gun_placement_counts_empty() {
        let (storage, _dir) = temp_storage();
        let counts = storage.get_gun_placement_counts().unwrap();
        assert!(counts.is_empty());
    }

    #[test]
    fn test_increment_marker_placement_new_kind() {
        let (storage, _dir) = temp_storage();
        storage.increment_marker_placement("target").unwrap();
        assert_eq!(storage.get_marker_placement_count("target").unwrap(), 1);
    }

    #[test]
    fn test_increment_marker_placement_accumulates() {
        let (storage, _dir) = temp_storage();
        for _ in 0..3 {
            storage.increment_marker_placement("spotter").unwrap();
        }
        assert_eq!(storage.get_marker_placement_count("spotter").unwrap(), 3);
    }

    #[test]
    fn test_get_marker_placement_count_absent() {
        let (storage, _dir) = temp_storage();
        assert_eq!(storage.get_marker_placement_count("target").unwrap(), 0);
    }

    fn test_plan(id: uuid::Uuid, name: &str) -> Plan {
        use foxhole_shared::models::Position;
        Plan {
            id,
            name: name.to_string(),
            map_id: "test-map".to_string(),
            weapon_ids: vec!["mortar".to_string()],
            gun_position: None,
            target_position: None,
            spotter_position: None,
            gun_positions: vec![Position { x: 100.0, y: 200.0 }],
            target_positions: vec![Position { x: 300.0, y: 400.0 }],
            spotter_positions: vec![],
            gun_target_indices: vec![Some(0)],
            wind_direction: Some(90.0),
            wind_strength: 3,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_save_and_get_plan() {
        let (storage, _dir) = temp_storage();
        let id = uuid::Uuid::new_v4();
        let plan = test_plan(id, "Test Plan");
        storage.save_plan(&plan).unwrap();

        let loaded = storage.get_plan(&id.to_string()).unwrap().unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.name, "Test Plan");
        assert_eq!(loaded.map_id, "test-map");
        assert_eq!(loaded.weapon_ids, vec!["mortar".to_string()]);
        assert_eq!(loaded.gun_positions.len(), 1);
        assert!((loaded.gun_positions[0].x - 100.0).abs() < 1e-9);
        assert_eq!(loaded.target_positions.len(), 1);
        assert_eq!(loaded.gun_target_indices, vec![Some(0)]);
        assert_eq!(loaded.wind_direction, Some(90.0));
        assert_eq!(loaded.wind_strength, 3);
    }

    #[test]
    fn test_get_plan_not_found() {
        let (storage, _dir) = temp_storage();
        let result = storage.get_plan("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count_plans() {
        let (storage, _dir) = temp_storage();
        let ids: Vec<uuid::Uuid> = (0..3).map(|_| uuid::Uuid::new_v4()).collect();
        for (i, id) in ids.iter().enumerate() {
            storage
                .save_plan(&test_plan(*id, &format!("Plan {}", i)))
                .unwrap();
        }
        assert_eq!(storage.count_plans().unwrap(), 3);
    }

    #[test]
    fn test_save_plan_overwrites() {
        let (storage, _dir) = temp_storage();
        let id = uuid::Uuid::new_v4();

        let plan1 = test_plan(id, "Original Name");
        storage.save_plan(&plan1).unwrap();

        let mut plan2 = test_plan(id, "Updated Name");
        plan2.wind_strength = 5;
        storage.save_plan(&plan2).unwrap();

        let loaded = storage.get_plan(&id.to_string()).unwrap().unwrap();
        assert_eq!(loaded.name, "Updated Name");
        assert_eq!(loaded.wind_strength, 5);
    }
}

use foxhole_shared::models::Plan;
use redb::{Database, ReadableDatabase, ReadableTableMetadata, TableDefinition};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const PLANS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("plans");

pub struct Storage {
    db: Database,
    path: PathBuf,
}

impl Storage {
    pub fn open(path: &Path) -> Arc<Self> {
        let db = Database::create(path)
            .unwrap_or_else(|e| panic!("Failed to open database at {}: {}", path.display(), e));

        // Ensure table exists
        let write_txn = db.begin_write().expect("Failed to begin write txn");
        {
            let _ = write_txn.open_table(PLANS_TABLE);
        }
        write_txn.commit().expect("Failed to commit initial txn");

        Arc::new(Storage { db, path: path.to_path_buf() })
    }

    pub fn save_plan(&self, plan: &Plan) -> Result<(), String> {
        let json = serde_json::to_vec(plan).map_err(|e| e.to_string())?;
        let id_str = plan.id.to_string();

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn.open_table(PLANS_TABLE).map_err(|e| e.to_string())?;
            table
                .insert(id_str.as_str(), json.as_slice())
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_plan(&self, id: &str) -> Result<Option<Plan>, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn.open_table(PLANS_TABLE).map_err(|e| e.to_string())?;

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
        let table = read_txn.open_table(PLANS_TABLE).map_err(|e| e.to_string())?;
        table.len().map_err(|e| e.to_string())
    }

    pub fn db_size_bytes(&self) -> Result<u64, String> {
        std::fs::metadata(&self.path)
            .map(|m| m.len())
            .map_err(|e| e.to_string())
    }

    pub fn delete_plan(&self, id: &str) -> Result<bool, String> {
        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        let removed = {
            let mut table = write_txn.open_table(PLANS_TABLE).map_err(|e| e.to_string())?;
            let result = table.remove(id).map_err(|e| e.to_string())?;
            result.is_some()
        };
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(removed)
    }
}

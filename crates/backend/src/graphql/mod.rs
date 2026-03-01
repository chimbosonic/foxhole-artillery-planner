use std::sync::Arc;

use async_graphql::{Context, Enum, InputObject, Object, SimpleObject, ID};
use foxhole_shared::{
    calc,
    grid::{MAP_HEIGHT_M, MAP_WIDTH_M},
    models::{self, Faction, Position, WindInput, UNASSIGNED_WEAPON},
};

use crate::assets::Assets;
use crate::storage::Storage;

// Re-export Faction as a GraphQL enum
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlFaction {
    Colonial,
    Warden,
    Both,
}

impl From<Faction> for GqlFaction {
    fn from(f: Faction) -> Self {
        match f {
            Faction::Colonial => GqlFaction::Colonial,
            Faction::Warden => GqlFaction::Warden,
            Faction::Both => GqlFaction::Both,
        }
    }
}

impl From<GqlFaction> for Faction {
    fn from(f: GqlFaction) -> Self {
        match f {
            GqlFaction::Colonial => Faction::Colonial,
            GqlFaction::Warden => Faction::Warden,
            GqlFaction::Both => Faction::Both,
        }
    }
}

// GraphQL output types

#[derive(SimpleObject)]
pub struct GqlGameMap {
    pub display_name: String,
    pub file_name: String,
    pub image_type: String,
    pub active: bool,
}

#[derive(SimpleObject)]
pub struct GqlWeapon {
    pub slug: String,
    pub faction: GqlFaction,
    pub display_name: String,
    pub min_range: f64,
    pub max_range: f64,
    pub acc_radius_min: f64,
    pub acc_radius_max: f64,
    pub wind_drift_min: f64,
    pub wind_drift_max: f64,
}

#[derive(SimpleObject)]
pub struct GqlFiringSolution {
    pub azimuth: f64,
    pub distance: f64,
    pub in_range: bool,
    pub accuracy_radius: f64,
    pub wind_adjusted_azimuth: Option<f64>,
    pub wind_adjusted_distance: Option<f64>,
    pub wind_offset_meters: Option<f64>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(SimpleObject)]
pub struct GqlPlan {
    pub id: ID,
    pub name: String,
    pub map_id: String,
    pub weapon_ids: Vec<String>,
    pub gun_positions: Vec<GqlPosition>,
    pub target_positions: Vec<GqlPosition>,
    pub spotter_positions: Vec<GqlPosition>,
    pub gun_target_indices: Vec<Option<i32>>,
    pub wind_direction: Option<f64>,
    pub wind_strength: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<models::Plan> for GqlPlan {
    fn from(p: models::Plan) -> Self {
        GqlPlan {
            id: ID(p.id.to_string()),
            name: p.name,
            map_id: p.map_id,
            weapon_ids: p.weapon_ids,
            gun_positions: p
                .gun_positions
                .into_iter()
                .map(|pos| GqlPosition { x: pos.x, y: pos.y })
                .collect(),
            target_positions: p
                .target_positions
                .into_iter()
                .map(|pos| GqlPosition { x: pos.x, y: pos.y })
                .collect(),
            spotter_positions: p
                .spotter_positions
                .into_iter()
                .map(|pos| GqlPosition { x: pos.x, y: pos.y })
                .collect(),
            gun_target_indices: p
                .gun_target_indices
                .into_iter()
                .map(|o| o.map(|v| v as i32))
                .collect(),
            wind_direction: p.wind_direction,
            wind_strength: p.wind_strength as u32,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(SimpleObject)]
pub struct GqlWeaponPlacementStat {
    pub weapon_slug: String,
    pub display_name: String,
    pub faction: GqlFaction,
    pub count: u64,
}

#[derive(SimpleObject)]
pub struct GqlFactionPlacementStats {
    pub colonial: u64,
    pub warden: u64,
    pub total: u64,
}

#[derive(SimpleObject)]
pub struct GqlMarkerPlacementStats {
    pub targets: u64,
    pub spotters: u64,
}

#[derive(SimpleObject)]
pub struct GqlStats {
    pub total_plans: u64,
    pub db_size_bytes: u64,
    pub gun_placements: Vec<GqlWeaponPlacementStat>,
    pub gun_placement_totals: GqlFactionPlacementStats,
    pub marker_placements: GqlMarkerPlacementStats,
}

// Input types

#[derive(InputObject)]
pub struct PositionInput {
    pub x: f64,
    pub y: f64,
}

#[derive(InputObject)]
pub struct GqlWindInput {
    pub direction: f64,
    pub strength: u32,
}

#[derive(InputObject)]
pub struct CalculateInput {
    pub gun_position: PositionInput,
    pub target_position: PositionInput,
    pub weapon_id: String,
    pub wind: Option<GqlWindInput>,
}

#[derive(InputObject)]
pub struct CreatePlanInput {
    pub name: String,
    pub map_id: String,
    pub weapon_ids: Vec<String>,
    pub gun_positions: Option<Vec<PositionInput>>,
    pub target_positions: Option<Vec<PositionInput>>,
    pub spotter_positions: Option<Vec<PositionInput>>,
    pub gun_target_indices: Option<Vec<Option<i32>>>,
    pub wind_direction: Option<f64>,
    pub wind_strength: Option<u32>,
}

// Helpers

fn ctx_data<'a, T: Send + Sync + 'static>(ctx: &'a Context<'a>) -> async_graphql::Result<&'a T> {
    ctx.data::<T>().map_err(|_| {
        tracing::error!(type_name = std::any::type_name::<T>(), "Missing context data");
        async_graphql::Error::new("Internal server error: missing context data")
    })
}

fn validate_name(name: &str) -> async_graphql::Result<()> {
    if name.len() > 200 {
        return Err(async_graphql::Error::new(
            "Plan name must be 200 characters or fewer",
        ));
    }
    Ok(())
}

fn validate_map_id(map_id: &str, assets: &Assets) -> async_graphql::Result<()> {
    if assets.find_map_by_file_name(map_id).is_none() {
        return Err(async_graphql::Error::new(format!(
            "Unknown map: {}",
            map_id
        )));
    }
    Ok(())
}

fn validate_weapon_ids(weapon_ids: &[String], assets: &Assets) -> async_graphql::Result<()> {
    for wid in weapon_ids {
        if wid.is_empty() || wid == UNASSIGNED_WEAPON {
            continue;
        }
        if assets.find_weapon_by_slug(wid).is_none() {
            return Err(async_graphql::Error::new(format!(
                "Unknown weapon: {}",
                wid
            )));
        }
    }
    Ok(())
}

fn validate_position(pos: &PositionInput, field_name: &str) -> async_graphql::Result<()> {
    if !pos.x.is_finite() || !pos.y.is_finite() {
        return Err(async_graphql::Error::new(format!(
            "{}: coordinates must be finite numbers",
            field_name
        )));
    }
    if pos.x < 0.0 || pos.x > MAP_WIDTH_M || pos.y < 0.0 || pos.y > MAP_HEIGHT_M {
        return Err(async_graphql::Error::new(format!(
            "{}: coordinates out of bounds (x: 0..{}, y: 0..{})",
            field_name, MAP_WIDTH_M, MAP_HEIGHT_M
        )));
    }
    Ok(())
}

fn validate_positions(
    positions: &[PositionInput],
    field_name: &str,
) -> async_graphql::Result<()> {
    for (i, pos) in positions.iter().enumerate() {
        validate_position(pos, &format!("{}[{}]", field_name, i))?;
    }
    Ok(())
}

fn validate_gun_target_indices(
    indices: &[Option<i32>],
    target_count: usize,
) -> async_graphql::Result<()> {
    for (i, idx) in indices.iter().enumerate() {
        if let Some(v) = idx {
            if *v < 0 {
                return Err(async_graphql::Error::new(format!(
                    "gun_target_indices[{}]: index must be non-negative",
                    i
                )));
            }
            if target_count > 0 && *v as usize >= target_count {
                return Err(async_graphql::Error::new(format!(
                    "gun_target_indices[{}]: index {} out of bounds (target count: {})",
                    i, v, target_count
                )));
            }
        }
    }
    Ok(())
}

fn validate_wind_direction(dir: f64) -> async_graphql::Result<()> {
    if !dir.is_finite() || !(0.0..360.0).contains(&dir) {
        return Err(async_graphql::Error::new(
            "wind_direction must be a finite number in range [0, 360)",
        ));
    }
    Ok(())
}

fn validate_wind_strength(strength: u32) -> async_graphql::Result<()> {
    if strength > 5 {
        return Err(async_graphql::Error::new(
            "wind_strength must be between 0 and 5",
        ));
    }
    Ok(())
}

fn validate_create_plan(input: &CreatePlanInput, assets: &Assets) -> async_graphql::Result<()> {
    validate_name(&input.name)?;
    validate_map_id(&input.map_id, assets)?;
    validate_weapon_ids(&input.weapon_ids, assets)?;
    if let Some(positions) = &input.gun_positions {
        validate_positions(positions, "gun_positions")?;
    }
    if let Some(positions) = &input.target_positions {
        validate_positions(positions, "target_positions")?;
    }
    if let Some(positions) = &input.spotter_positions {
        validate_positions(positions, "spotter_positions")?;
    }
    if let Some(indices) = &input.gun_target_indices {
        let target_count = input
            .target_positions
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(0);
        validate_gun_target_indices(indices, target_count)?;
    }
    if let Some(dir) = input.wind_direction {
        validate_wind_direction(dir)?;
    }
    if let Some(strength) = input.wind_strength {
        validate_wind_strength(strength)?;
    }
    Ok(())
}

// Query root

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn maps(
        &self,
        ctx: &Context<'_>,
        active_only: Option<bool>,
    ) -> async_graphql::Result<Vec<GqlGameMap>> {
        let assets = ctx_data::<Arc<Assets>>(ctx)?;
        Ok(assets
            .maps
            .iter()
            .filter(|m| {
                if active_only.unwrap_or(false) {
                    m.active
                } else {
                    true
                }
            })
            .map(|m| GqlGameMap {
                display_name: m.display_name.clone(),
                file_name: m.file_name.clone(),
                image_type: m.image_type.clone(),
                active: m.active,
            })
            .collect())
    }

    async fn weapons(
        &self,
        ctx: &Context<'_>,
        faction: Option<GqlFaction>,
    ) -> async_graphql::Result<Vec<GqlWeapon>> {
        let assets = ctx_data::<Arc<Assets>>(ctx)?;
        Ok(assets
            .weapons
            .iter()
            .filter(|w| match faction {
                Some(f) => {
                    let f: Faction = f.into();
                    w.faction == f || w.faction == Faction::Both
                }
                None => true,
            })
            .map(|w| GqlWeapon {
                slug: w.slug(),
                faction: w.faction.into(),
                display_name: w.display_name.clone(),
                min_range: w.min_range,
                max_range: w.max_range,
                acc_radius_min: w.acc_radius[0],
                acc_radius_max: w.acc_radius[1],
                wind_drift_min: w.wind_drift[0],
                wind_drift_max: w.wind_drift[1],
            })
            .collect())
    }

    async fn calculate(
        &self,
        ctx: &Context<'_>,
        input: CalculateInput,
    ) -> async_graphql::Result<GqlFiringSolution> {
        let assets = ctx_data::<Arc<Assets>>(ctx)?;
        let weapon = assets
            .find_weapon_by_slug(&input.weapon_id)
            .ok_or_else(|| {
                async_graphql::Error::new(format!("Unknown weapon: {}", input.weapon_id))
            })?;

        let gun = Position {
            x: input.gun_position.x,
            y: input.gun_position.y,
        };
        let target = Position {
            x: input.target_position.x,
            y: input.target_position.y,
        };
        let wind = input.wind.map(|w| WindInput {
            direction: w.direction,
            strength: w.strength as u8,
        });

        let sol = calc::firing_solution(gun, target, weapon, wind.as_ref());

        Ok(GqlFiringSolution {
            azimuth: sol.azimuth,
            distance: sol.distance,
            in_range: sol.in_range,
            accuracy_radius: sol.accuracy_radius,
            wind_adjusted_azimuth: sol.wind_adjusted_azimuth,
            wind_adjusted_distance: sol.wind_adjusted_distance,
            wind_offset_meters: sol.wind_offset_meters,
        })
    }

    async fn plan(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<Option<GqlPlan>> {
        let storage = ctx_data::<Arc<Storage>>(ctx)?;
        let plan = storage.get_plan(&id).map_err(async_graphql::Error::new)?;
        Ok(plan.map(GqlPlan::from))
    }

    async fn stats(&self, ctx: &Context<'_>) -> async_graphql::Result<GqlStats> {
        let storage = ctx_data::<Arc<Storage>>(ctx)?;
        let assets = ctx_data::<Arc<Assets>>(ctx)?;
        let total_plans = storage.count_plans().map_err(async_graphql::Error::new)?;
        let db_size_bytes = storage.db_size_bytes().map_err(async_graphql::Error::new)?;

        let raw_counts = storage
            .get_gun_placement_counts()
            .map_err(async_graphql::Error::new)?;

        let mut colonial_total: u64 = 0;
        let mut warden_total: u64 = 0;
        let mut overall_total: u64 = 0;
        let mut gun_placements = Vec::new();

        for (slug, count) in raw_counts {
            let (display_name, faction): (String, Faction) = if slug == UNASSIGNED_WEAPON {
                ("Unassigned".to_string(), Faction::Both)
            } else {
                match assets.find_weapon_by_slug(&slug) {
                    Some(w) => (w.display_name.clone(), w.faction),
                    None => (slug.clone(), Faction::Both),
                }
            };
            match faction {
                Faction::Colonial => colonial_total += count,
                Faction::Warden => warden_total += count,
                Faction::Both => {
                    colonial_total += count;
                    warden_total += count;
                }
            }
            overall_total += count;
            gun_placements.push(GqlWeaponPlacementStat {
                weapon_slug: slug,
                display_name,
                faction: faction.into(),
                count,
            });
        }

        let target_count = storage
            .get_marker_placement_count("target")
            .map_err(async_graphql::Error::new)?;
        let spotter_count = storage
            .get_marker_placement_count("spotter")
            .map_err(async_graphql::Error::new)?;

        Ok(GqlStats {
            total_plans,
            db_size_bytes,
            gun_placements,
            gun_placement_totals: GqlFactionPlacementStats {
                colonial: colonial_total,
                warden: warden_total,
                total: overall_total,
            },
            marker_placements: GqlMarkerPlacementStats {
                targets: target_count,
                spotters: spotter_count,
            },
        })
    }
}

// Mutation root

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_plan(
        &self,
        ctx: &Context<'_>,
        input: CreatePlanInput,
    ) -> async_graphql::Result<GqlPlan> {
        let assets = ctx_data::<Arc<Assets>>(ctx)?;
        let storage = ctx_data::<Arc<Storage>>(ctx)?;
        if let Err(e) = validate_create_plan(&input, assets) {
            tracing::warn!(error = %e.message, "Plan validation failed");
            return Err(e);
        }
        let now = chrono::Utc::now().to_rfc3339();

        let to_positions = |v: Option<Vec<PositionInput>>| -> Vec<Position> {
            v.unwrap_or_default()
                .into_iter()
                .map(|p| Position { x: p.x, y: p.y })
                .collect()
        };

        let plan = models::Plan {
            id: uuid::Uuid::new_v4(),
            name: input.name,
            map_id: input.map_id,
            weapon_ids: input.weapon_ids,
            gun_position: None,
            target_position: None,
            spotter_position: None,
            gun_positions: to_positions(input.gun_positions),
            target_positions: to_positions(input.target_positions),
            spotter_positions: to_positions(input.spotter_positions),
            gun_target_indices: input
                .gun_target_indices
                .unwrap_or_default()
                .into_iter()
                .map(|o| o.map(|v| v as u32))
                .collect(),
            wind_direction: input.wind_direction,
            wind_strength: input.wind_strength.unwrap_or(0) as u8,
            created_at: now.clone(),
            updated_at: now,
        };

        storage.save_plan(&plan).map_err(|e| {
            tracing::error!(error = %e, "Failed to save plan");
            async_graphql::Error::new(e)
        })?;

        tracing::info!(plan_id = %plan.id, map = %plan.map_id, "Plan created");
        Ok(GqlPlan::from(plan))
    }

    async fn track_target_placement(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let storage = ctx_data::<Arc<Storage>>(ctx)?;
        storage.increment_marker_placement("target").map_err(|e| {
            tracing::warn!(error = %e, "Failed to track target placement");
            async_graphql::Error::new(e)
        })?;
        tracing::info!("Target placement tracked");
        Ok(true)
    }

    async fn track_spotter_placement(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let storage = ctx_data::<Arc<Storage>>(ctx)?;
        storage.increment_marker_placement("spotter").map_err(|e| {
            tracing::warn!(error = %e, "Failed to track spotter placement");
            async_graphql::Error::new(e)
        })?;
        tracing::info!("Spotter placement tracked");
        Ok(true)
    }

    async fn track_gun_placement(
        &self,
        ctx: &Context<'_>,
        weapon_slug: String,
    ) -> async_graphql::Result<bool> {
        let assets = ctx_data::<Arc<Assets>>(ctx)?;
        // Allow empty or "unassigned" for guns placed without a weapon
        if !weapon_slug.is_empty()
            && weapon_slug != UNASSIGNED_WEAPON
            && assets.find_weapon_by_slug(&weapon_slug).is_none()
        {
            return Err(async_graphql::Error::new(format!(
                "Unknown weapon: {}",
                weapon_slug
            )));
        }
        let storage = ctx_data::<Arc<Storage>>(ctx)?;
        storage.increment_gun_placement(&weapon_slug).map_err(|e| {
            tracing::warn!(error = %e, weapon = %weapon_slug, "Failed to track gun placement");
            async_graphql::Error::new(e)
        })?;
        tracing::info!(weapon = %weapon_slug, "Gun placement tracked");
        Ok(true)
    }
}

pub type Schema = async_graphql::Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>;

pub fn build_schema(assets: Arc<Assets>, storage: Arc<Storage>) -> Schema {
    async_graphql::Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
        .data(assets)
        .data(storage)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn test_assets() -> Arc<Assets> {
        Arc::new(Assets {
            maps: vec![foxhole_shared::models::GameMap {
                image_type: "webp".to_string(),
                display_name: "Test Map".to_string(),
                file_name: "test-map".to_string(),
                active: true,
            }],
            weapons: vec![foxhole_shared::models::Weapon {
                faction: Faction::Colonial,
                display_name: "Test Mortar".to_string(),
                min_range: 75.0,
                max_range: 300.0,
                acc_radius: [20.0, 35.0],
                wind_drift: [5.0, 15.0],
            }],
        })
    }

    fn test_storage() -> (Arc<Storage>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.redb");
        let storage = Storage::open(&path);
        (storage, dir)
    }

    fn schema_with_context() -> (Schema, tempfile::TempDir) {
        let assets = test_assets();
        let (storage, dir) = test_storage();
        (build_schema(assets, storage), dir)
    }

    /// Build a schema with NO context data inserted — simulates a misconfigured server.
    fn schema_without_context() -> Schema {
        async_graphql::Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
            .finish()
    }

    // ---- Part 1: Missing context data returns errors instead of panicking ----

    #[tokio::test]
    async fn test_maps_query_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema.execute("{ maps { displayName } }").await;
        assert!(!resp.errors.is_empty(), "expected error, got success");
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_weapons_query_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema.execute("{ weapons { slug } }").await;
        assert!(!resp.errors.is_empty(), "expected error, got success");
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_plan_query_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute(r#"{ plan(id: "nonexistent") { name } }"#)
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_stats_query_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute("{ stats { totalPlans dbSizeBytes } }")
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_calculate_query_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute(
                r#"{ calculate(input: {
                    gunPosition: { x: 100, y: 100 },
                    targetPosition: { x: 200, y: 200 },
                    weaponId: "test-mortar"
                }) { azimuth } }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_create_plan_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: []
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_track_gun_placement_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute(r#"mutation { trackGunPlacement(weaponSlug: "mortar") }"#)
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_track_target_placement_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute("mutation { trackTargetPlacement }")
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    #[tokio::test]
    async fn test_track_spotter_placement_without_context_returns_error() {
        let schema = schema_without_context();
        let resp = schema
            .execute("mutation { trackSpotterPlacement }")
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0]
            .message
            .contains("Internal server error: missing context data"));
    }

    // ---- Part 2: Queries succeed with valid context ----

    #[tokio::test]
    async fn test_maps_query_with_context_succeeds() {
        let (schema, _dir) = schema_with_context();
        let resp = schema.execute("{ maps { displayName } }").await;
        assert!(resp.errors.is_empty(), "unexpected errors: {:?}", resp.errors);
    }

    #[tokio::test]
    async fn test_weapons_query_with_context_succeeds() {
        let (schema, _dir) = schema_with_context();
        let resp = schema.execute("{ weapons { slug } }").await;
        assert!(resp.errors.is_empty(), "unexpected errors: {:?}", resp.errors);
    }

    // ---- Part 3: Input validation returns errors ----

    #[tokio::test]
    async fn test_create_plan_unknown_map_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "nonexistent-map",
                        weaponIds: []
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("Unknown map"));
    }

    #[tokio::test]
    async fn test_create_plan_unknown_weapon_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: ["bogus-weapon"]
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("Unknown weapon"));
    }

    #[tokio::test]
    async fn test_create_plan_unassigned_weapon_allowed() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: ["unassigned"]
                    }) { id }
                }"#,
            )
            .await;
        assert!(resp.errors.is_empty(), "unexpected errors: {:?}", resp.errors);
    }

    #[tokio::test]
    async fn test_create_plan_name_too_long_returns_error() {
        let (schema, _dir) = schema_with_context();
        let long_name = "x".repeat(201);
        let query = format!(
            r#"mutation {{
                createPlan(input: {{
                    name: "{}",
                    mapId: "test-map",
                    weaponIds: []
                }}) {{ id }}
            }}"#,
            long_name
        );
        let resp = schema.execute(&query).await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("200 characters"));
    }

    #[tokio::test]
    async fn test_create_plan_out_of_bounds_position_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: [],
                        gunPositions: [{ x: 9999, y: 100 }]
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("out of bounds"));
    }

    #[tokio::test]
    async fn test_create_plan_negative_position_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: [],
                        targetPositions: [{ x: -1, y: 100 }]
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("out of bounds"));
    }

    #[tokio::test]
    async fn test_create_plan_wind_strength_too_high_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: [],
                        windStrength: 10
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("wind_strength"));
    }

    #[tokio::test]
    async fn test_create_plan_wind_direction_out_of_range_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: [],
                        windDirection: 400.0
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("wind_direction"));
    }

    #[tokio::test]
    async fn test_create_plan_gun_target_index_out_of_bounds_returns_error() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "test",
                        mapId: "test-map",
                        weaponIds: ["test-mortar"],
                        gunPositions: [{ x: 100, y: 100 }],
                        targetPositions: [{ x: 200, y: 200 }],
                        gunTargetIndices: [5]
                    }) { id }
                }"#,
            )
            .await;
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("out of bounds"));
    }

    #[tokio::test]
    async fn test_create_plan_valid_input_succeeds() {
        let (schema, _dir) = schema_with_context();
        let resp = schema
            .execute(
                r#"mutation {
                    createPlan(input: {
                        name: "Valid Plan",
                        mapId: "test-map",
                        weaponIds: ["test-mortar"],
                        gunPositions: [{ x: 100, y: 100 }],
                        targetPositions: [{ x: 200, y: 200 }],
                        gunTargetIndices: [0],
                        windDirection: 90.0,
                        windStrength: 3
                    }) { id name }
                }"#,
            )
            .await;
        assert!(resp.errors.is_empty(), "unexpected errors: {:?}", resp.errors);
    }

}

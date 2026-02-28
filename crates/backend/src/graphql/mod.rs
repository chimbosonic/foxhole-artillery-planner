use std::sync::Arc;

use async_graphql::{Context, Enum, InputObject, Object, SimpleObject, ID};
use foxhole_shared::{
    calc,
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

#[derive(InputObject)]
pub struct UpdatePlanInput {
    pub id: ID,
    pub name: Option<String>,
    pub map_id: Option<String>,
    pub weapon_ids: Option<Vec<String>>,
    pub gun_positions: Option<Vec<PositionInput>>,
    pub target_positions: Option<Vec<PositionInput>>,
    pub spotter_positions: Option<Vec<PositionInput>>,
    pub gun_target_indices: Option<Vec<Option<i32>>>,
    pub wind_direction: Option<f64>,
    pub wind_strength: Option<u32>,
}

// Query root

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn maps(&self, ctx: &Context<'_>, active_only: Option<bool>) -> Vec<GqlGameMap> {
        let assets = ctx.data::<Arc<Assets>>().unwrap();
        assets
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
            .collect()
    }

    async fn weapons(&self, ctx: &Context<'_>, faction: Option<GqlFaction>) -> Vec<GqlWeapon> {
        let assets = ctx.data::<Arc<Assets>>().unwrap();
        assets
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
            .collect()
    }

    async fn calculate(
        &self,
        ctx: &Context<'_>,
        input: CalculateInput,
    ) -> async_graphql::Result<GqlFiringSolution> {
        let assets = ctx.data::<Arc<Assets>>().unwrap();
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
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        let plan = storage.get_plan(&id).map_err(async_graphql::Error::new)?;
        Ok(plan.map(GqlPlan::from))
    }

    async fn stats(&self, ctx: &Context<'_>) -> async_graphql::Result<GqlStats> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        let assets = ctx.data::<Arc<Assets>>().unwrap();
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
            let (display_name, faction) = if slug == UNASSIGNED_WEAPON {
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
        let storage = ctx.data::<Arc<Storage>>().unwrap();
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

        storage
            .save_plan(&plan)
            .map_err(async_graphql::Error::new)?;

        Ok(GqlPlan::from(plan))
    }

    async fn update_plan(
        &self,
        ctx: &Context<'_>,
        input: UpdatePlanInput,
    ) -> async_graphql::Result<GqlPlan> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();

        let mut plan = storage
            .get_plan(&input.id)
            .map_err(async_graphql::Error::new)?
            .ok_or_else(|| async_graphql::Error::new("Plan not found"))?;

        if let Some(name) = input.name {
            plan.name = name;
        }
        if let Some(map_id) = input.map_id {
            plan.map_id = map_id;
        }
        if let Some(weapon_ids) = input.weapon_ids {
            plan.weapon_ids = weapon_ids;
        }
        if let Some(positions) = input.gun_positions {
            plan.gun_positions = positions
                .into_iter()
                .map(|p| Position { x: p.x, y: p.y })
                .collect();
        }
        if let Some(positions) = input.target_positions {
            plan.target_positions = positions
                .into_iter()
                .map(|p| Position { x: p.x, y: p.y })
                .collect();
        }
        if let Some(positions) = input.spotter_positions {
            plan.spotter_positions = positions
                .into_iter()
                .map(|p| Position { x: p.x, y: p.y })
                .collect();
        }
        if let Some(indices) = input.gun_target_indices {
            plan.gun_target_indices = indices.into_iter().map(|o| o.map(|v| v as u32)).collect();
        }
        if let Some(dir) = input.wind_direction {
            plan.wind_direction = Some(dir);
        }
        if let Some(strength) = input.wind_strength {
            plan.wind_strength = strength as u8;
        }

        plan.updated_at = chrono::Utc::now().to_rfc3339();

        storage
            .save_plan(&plan)
            .map_err(async_graphql::Error::new)?;

        Ok(GqlPlan::from(plan))
    }

    async fn delete_plan(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<bool> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        storage.delete_plan(&id).map_err(async_graphql::Error::new)
    }

    async fn track_target_placement(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        storage
            .increment_marker_placement("target")
            .map_err(async_graphql::Error::new)?;
        Ok(true)
    }

    async fn track_spotter_placement(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        storage
            .increment_marker_placement("spotter")
            .map_err(async_graphql::Error::new)?;
        Ok(true)
    }

    async fn track_gun_placement(
        &self,
        ctx: &Context<'_>,
        weapon_slug: String,
    ) -> async_graphql::Result<bool> {
        let assets = ctx.data::<Arc<Assets>>().unwrap();
        // Allow "unassigned" for guns placed without a weapon
        if weapon_slug != UNASSIGNED_WEAPON && assets.find_weapon_by_slug(&weapon_slug).is_none() {
            return Err(async_graphql::Error::new(format!(
                "Unknown weapon: {}",
                weapon_slug
            )));
        }
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        storage
            .increment_gun_placement(&weapon_slug)
            .map_err(async_graphql::Error::new)?;
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

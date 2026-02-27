use std::sync::Arc;

use async_graphql::{Context, Enum, InputObject, Object, SimpleObject, ID};
use foxhole_shared::{
    calc,
    models::{self, Faction, Position, WindInput},
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

#[derive(SimpleObject)]
pub struct GqlPlan {
    pub id: ID,
    pub name: String,
    pub map_id: String,
    pub weapon_id: String,
    pub gun_x: Option<f64>,
    pub gun_y: Option<f64>,
    pub target_x: Option<f64>,
    pub target_y: Option<f64>,
    pub spotter_x: Option<f64>,
    pub spotter_y: Option<f64>,
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
            weapon_id: p.weapon_id,
            gun_x: p.gun_position.map(|pos| pos.x),
            gun_y: p.gun_position.map(|pos| pos.y),
            target_x: p.target_position.map(|pos| pos.x),
            target_y: p.target_position.map(|pos| pos.y),
            spotter_x: p.spotter_position.map(|pos| pos.x),
            spotter_y: p.spotter_position.map(|pos| pos.y),
            wind_direction: p.wind_direction,
            wind_strength: p.wind_strength as u32,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
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
    pub weapon_id: String,
    pub gun_x: Option<f64>,
    pub gun_y: Option<f64>,
    pub target_x: Option<f64>,
    pub target_y: Option<f64>,
    pub spotter_x: Option<f64>,
    pub spotter_y: Option<f64>,
    pub wind_direction: Option<f64>,
    pub wind_strength: Option<u32>,
}

#[derive(InputObject)]
pub struct UpdatePlanInput {
    pub id: ID,
    pub name: Option<String>,
    pub map_id: Option<String>,
    pub weapon_id: Option<String>,
    pub gun_x: Option<f64>,
    pub gun_y: Option<f64>,
    pub target_x: Option<f64>,
    pub target_y: Option<f64>,
    pub spotter_x: Option<f64>,
    pub spotter_y: Option<f64>,
    pub wind_direction: Option<f64>,
    pub wind_strength: Option<u32>,
}

// Query root

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn maps(
        &self,
        ctx: &Context<'_>,
        active_only: Option<bool>,
    ) -> Vec<GqlGameMap> {
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

    async fn weapons(
        &self,
        ctx: &Context<'_>,
        faction: Option<GqlFaction>,
    ) -> Vec<GqlWeapon> {
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
            .ok_or_else(|| async_graphql::Error::new(format!("Unknown weapon: {}", input.weapon_id)))?;

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

    async fn plan(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> async_graphql::Result<Option<GqlPlan>> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        let plan = storage
            .get_plan(&id)
            .map_err(async_graphql::Error::new)?;
        Ok(plan.map(GqlPlan::from))
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

        let plan = models::Plan {
            id: uuid::Uuid::new_v4(),
            name: input.name,
            map_id: input.map_id,
            weapon_id: input.weapon_id,
            gun_position: match (input.gun_x, input.gun_y) {
                (Some(x), Some(y)) => Some(Position { x, y }),
                _ => None,
            },
            target_position: match (input.target_x, input.target_y) {
                (Some(x), Some(y)) => Some(Position { x, y }),
                _ => None,
            },
            spotter_position: match (input.spotter_x, input.spotter_y) {
                (Some(x), Some(y)) => Some(Position { x, y }),
                _ => None,
            },
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
        if let Some(weapon_id) = input.weapon_id {
            plan.weapon_id = weapon_id;
        }
        if input.gun_x.is_some() || input.gun_y.is_some() {
            plan.gun_position = match (input.gun_x, input.gun_y) {
                (Some(x), Some(y)) => Some(Position { x, y }),
                _ => plan.gun_position,
            };
        }
        if input.target_x.is_some() || input.target_y.is_some() {
            plan.target_position = match (input.target_x, input.target_y) {
                (Some(x), Some(y)) => Some(Position { x, y }),
                _ => plan.target_position,
            };
        }
        if input.spotter_x.is_some() || input.spotter_y.is_some() {
            plan.spotter_position = match (input.spotter_x, input.spotter_y) {
                (Some(x), Some(y)) => Some(Position { x, y }),
                _ => plan.spotter_position,
            };
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

    async fn delete_plan(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> async_graphql::Result<bool> {
        let storage = ctx.data::<Arc<Storage>>().unwrap();
        storage
            .delete_plan(&id)
            .map_err(async_graphql::Error::new)
    }
}

pub type Schema = async_graphql::Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>;

pub fn build_schema(assets: Arc<Assets>, storage: Arc<Storage>) -> Schema {
    async_graphql::Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
        .data(assets)
        .data(storage)
        .finish()
}

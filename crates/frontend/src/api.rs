use serde::{Deserialize, Serialize};

/// Build the variables JSON for a calculate query.
pub fn build_calculate_variables(
    gun_x: f64,
    gun_y: f64,
    target_x: f64,
    target_y: f64,
    weapon_id: &str,
    wind_direction: Option<f64>,
    wind_strength: Option<u32>,
) -> serde_json::Value {
    let wind = match (wind_direction, wind_strength) {
        (Some(dir), Some(str)) if str > 0 => {
            serde_json::json!({ "direction": dir, "strength": str })
        }
        _ => serde_json::Value::Null,
    };

    serde_json::json!({
        "input": {
            "gunPosition": { "x": gun_x, "y": gun_y },
            "targetPosition": { "x": target_x, "y": target_y },
            "weaponId": weapon_id,
            "wind": wind
        }
    })
}

/// Build the variables JSON for a create plan mutation.
#[allow(clippy::too_many_arguments)]
pub fn build_create_plan_variables(
    name: &str,
    map_id: &str,
    weapon_ids: &[String],
    gun_positions: &[(f64, f64)],
    target_positions: &[(f64, f64)],
    spotter_positions: &[(f64, f64)],
    gun_target_indices: &[Option<usize>],
    wind_direction: Option<f64>,
    wind_strength: Option<u32>,
) -> serde_json::Value {
    let to_json = |positions: &[(f64, f64)]| -> serde_json::Value {
        positions
            .iter()
            .map(|(x, y)| serde_json::json!({"x": x, "y": y}))
            .collect()
    };
    let indices_json: serde_json::Value = gun_target_indices
        .iter()
        .map(|o| match o {
            Some(v) => serde_json::json!(*v as i32),
            None => serde_json::Value::Null,
        })
        .collect();
    serde_json::json!({
        "input": {
            "name": name,
            "mapId": map_id,
            "weaponIds": weapon_ids,
            "gunPositions": to_json(gun_positions),
            "targetPositions": to_json(target_positions),
            "spotterPositions": to_json(spotter_positions),
            "gunTargetIndices": indices_json,
            "windDirection": wind_direction,
            "windStrength": wind_strength
        }
    })
}

/// Build a shareable plan URL from origin and plan ID.
pub fn build_plan_url(origin: &str, plan_id: &str) -> String {
    format!("{}/plan/{}", origin, plan_id)
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphQLRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

fn api_url() -> String {
    // In production, same origin. In dev, might be different.
    let window = web_sys::window().unwrap();
    let origin = window.location().origin().unwrap();
    format!("{}/graphql", origin)
}

async fn query<T: for<'de> Deserialize<'de>>(
    query_str: &str,
    variables: Option<serde_json::Value>,
) -> Result<T, String> {
    let req = GraphQLRequest {
        query: query_str.to_string(),
        variables,
    };

    let resp = reqwest::Client::new()
        .post(api_url())
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let gql_resp: GraphQLResponse<T> = resp.json().await.map_err(|e| e.to_string())?;

    if let Some(errors) = gql_resp.errors {
        if !errors.is_empty() {
            return Err(errors[0].message.clone());
        }
    }

    gql_resp.data.ok_or_else(|| "No data returned".to_string())
}

// Types mirroring the GraphQL schema

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapData {
    pub display_name: String,
    pub file_name: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponData {
    pub slug: String,
    pub faction: String,
    pub display_name: String,
    pub min_range: f64,
    pub max_range: f64,
    pub acc_radius_min: f64,
    pub acc_radius_max: f64,
    pub wind_drift_min: f64,
    pub wind_drift_max: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiringSolutionData {
    pub azimuth: f64,
    pub distance: f64,
    pub in_range: bool,
    pub accuracy_radius: f64,
    pub wind_adjusted_azimuth: Option<f64>,
    pub wind_adjusted_distance: Option<f64>,
    pub wind_offset_meters: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionData {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanData {
    pub id: String,
    pub name: String,
    pub map_id: String,
    #[serde(default)]
    pub weapon_ids: Vec<String>,
    pub gun_positions: Vec<PositionData>,
    pub target_positions: Vec<PositionData>,
    pub spotter_positions: Vec<PositionData>,
    #[serde(default)]
    pub gun_target_indices: Vec<Option<i32>>,
    pub wind_direction: Option<f64>,
    pub wind_strength: u32,
}

// API functions

#[derive(Deserialize)]
pub struct MapsResponse {
    pub maps: Vec<MapData>,
}

pub async fn fetch_maps() -> Result<Vec<MapData>, String> {
    let resp: MapsResponse = query(
        r#"query { maps(activeOnly: true) { displayName fileName active } }"#,
        None,
    )
    .await?;
    Ok(resp.maps)
}

#[derive(Deserialize)]
pub struct WeaponsResponse {
    pub weapons: Vec<WeaponData>,
}

pub async fn fetch_weapons() -> Result<Vec<WeaponData>, String> {
    let resp: WeaponsResponse = query(
        r#"query { weapons { slug faction displayName minRange maxRange accRadiusMin accRadiusMax windDriftMin windDriftMax } }"#,
        None,
    )
    .await?;
    Ok(resp.weapons)
}

#[derive(Deserialize)]
pub struct CalculateResponse {
    pub calculate: FiringSolutionData,
}

pub async fn calculate(
    gun_x: f64,
    gun_y: f64,
    target_x: f64,
    target_y: f64,
    weapon_id: &str,
    wind_direction: Option<f64>,
    wind_strength: Option<u32>,
) -> Result<FiringSolutionData, String> {
    let variables = build_calculate_variables(
        gun_x,
        gun_y,
        target_x,
        target_y,
        weapon_id,
        wind_direction,
        wind_strength,
    );

    let resp: CalculateResponse = query(
        r#"query Calculate($input: CalculateInput!) {
            calculate(input: $input) {
                azimuth distance inRange accuracyRadius
                windAdjustedAzimuth windAdjustedDistance windOffsetMeters
            }
        }"#,
        Some(variables),
    )
    .await?;
    Ok(resp.calculate)
}

#[derive(Deserialize)]
pub struct CreatePlanResponse {
    #[serde(rename = "createPlan")]
    pub create_plan: PlanData,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_plan(
    name: &str,
    map_id: &str,
    weapon_ids: &[String],
    gun_positions: &[(f64, f64)],
    target_positions: &[(f64, f64)],
    spotter_positions: &[(f64, f64)],
    gun_target_indices: &[Option<usize>],
    wind_direction: Option<f64>,
    wind_strength: Option<u32>,
) -> Result<PlanData, String> {
    let variables = build_create_plan_variables(
        name,
        map_id,
        weapon_ids,
        gun_positions,
        target_positions,
        spotter_positions,
        gun_target_indices,
        wind_direction,
        wind_strength,
    );

    let resp: CreatePlanResponse = query(
        r#"mutation CreatePlan($input: CreatePlanInput!) {
            createPlan(input: $input) {
                id name mapId weaponIds
                gunPositions { x y } targetPositions { x y } spotterPositions { x y }
                gunTargetIndices windDirection windStrength
            }
        }"#,
        Some(variables),
    )
    .await?;
    Ok(resp.create_plan)
}

#[derive(Deserialize)]
pub struct FetchPlanResponse {
    pub plan: Option<PlanData>,
}

#[derive(Deserialize)]
pub struct TrackGunPlacementResponse {
    #[serde(rename = "trackGunPlacement")]
    pub track_gun_placement: bool,
}

/// Fire-and-forget gun placement tracking. Maps empty slugs to "unassigned".
pub fn track_gun_placement_fire(weapon_slug: &str) {
    let slug = if weapon_slug.is_empty() {
        foxhole_shared::models::UNASSIGNED_WEAPON.to_string()
    } else {
        weapon_slug.to_string()
    };
    wasm_bindgen_futures::spawn_local(async move {
        let _ = track_gun_placement(&slug).await;
    });
}

pub async fn track_gun_placement(weapon_slug: &str) -> Result<bool, String> {
    let variables = serde_json::json!({ "weaponSlug": weapon_slug });
    let resp: TrackGunPlacementResponse = query(
        r#"mutation TrackGunPlacement($weaponSlug: String!) {
            trackGunPlacement(weaponSlug: $weaponSlug)
        }"#,
        Some(variables),
    )
    .await?;
    Ok(resp.track_gun_placement)
}

pub async fn fetch_plan(id: &str) -> Result<Option<PlanData>, String> {
    let variables = serde_json::json!({ "id": id });

    let resp: FetchPlanResponse = query(
        r#"query FetchPlan($id: ID!) {
            plan(id: $id) {
                id name mapId weaponIds
                gunPositions { x y } targetPositions { x y } spotterPositions { x y }
                gunTargetIndices windDirection windStrength
            }
        }"#,
        Some(variables),
    )
    .await?;
    Ok(resp.plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GraphQL request serialization ---

    #[test]
    fn test_graphql_request_serializes_with_variables() {
        let req = GraphQLRequest {
            query: "query { maps { displayName } }".to_string(),
            variables: Some(serde_json::json!({"activeOnly": true})),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["query"], "query { maps { displayName } }");
        assert_eq!(json["variables"]["activeOnly"], true);
    }

    #[test]
    fn test_graphql_request_omits_null_variables() {
        let req = GraphQLRequest {
            query: "query { weapons { slug } }".to_string(),
            variables: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("variables").is_none());
    }

    // --- Response deserialization ---

    #[test]
    fn test_maps_response_deserializes() {
        let json = r#"{"maps":[{"displayName":"Deadlands","fileName":"deadlands","active":true}]}"#;
        let resp: MapsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.maps.len(), 1);
        assert_eq!(resp.maps[0].display_name, "Deadlands");
        assert_eq!(resp.maps[0].file_name, "deadlands");
        assert!(resp.maps[0].active);
    }

    #[test]
    fn test_weapons_response_deserializes() {
        let json = r#"{"weapons":[{"slug":"storm-cannon","faction":"BOTH","displayName":"Storm Cannon","minRange":400.0,"maxRange":1000.0,"accRadiusMin":50.0,"accRadiusMax":50.0,"windDriftMin":20.0,"windDriftMax":50.0}]}"#;
        let resp: WeaponsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.weapons.len(), 1);
        assert_eq!(resp.weapons[0].slug, "storm-cannon");
        assert_eq!(resp.weapons[0].min_range, 400.0);
    }

    #[test]
    fn test_firing_solution_deserializes() {
        let json = r#"{"calculate":{"azimuth":45.0,"distance":200.0,"inRange":true,"accuracyRadius":15.0,"windAdjustedAzimuth":44.5,"windAdjustedDistance":201.0,"windOffsetMeters":8.0}}"#;
        let resp: CalculateResponse = serde_json::from_str(json).unwrap();
        assert!((resp.calculate.azimuth - 45.0).abs() < 1e-9);
        assert!(resp.calculate.in_range);
        assert_eq!(resp.calculate.wind_adjusted_azimuth, Some(44.5));
    }

    #[test]
    fn test_firing_solution_deserializes_no_wind() {
        let json = r#"{"calculate":{"azimuth":90.0,"distance":150.0,"inRange":true,"accuracyRadius":10.0,"windAdjustedAzimuth":null,"windAdjustedDistance":null,"windOffsetMeters":null}}"#;
        let resp: CalculateResponse = serde_json::from_str(json).unwrap();
        assert!(resp.calculate.wind_adjusted_azimuth.is_none());
        assert!(resp.calculate.wind_offset_meters.is_none());
    }

    #[test]
    fn test_plan_data_deserializes() {
        let json = r#"{"plan":{"id":"abc-123","name":"Test Plan","mapId":"deadlands","weaponIds":["storm-cannon"],"gunPositions":[{"x":100.0,"y":200.0}],"targetPositions":[{"x":300.0,"y":400.0}],"spotterPositions":[],"gunTargetIndices":[0],"windDirection":90.0,"windStrength":3}}"#;
        let resp: FetchPlanResponse = serde_json::from_str(json).unwrap();
        let plan = resp.plan.unwrap();
        assert_eq!(plan.id, "abc-123");
        assert_eq!(plan.weapon_ids, vec!["storm-cannon"]);
        assert_eq!(plan.gun_positions.len(), 1);
        assert_eq!(plan.gun_positions[0].x, 100.0);
        assert!(plan.spotter_positions.is_empty());
        assert_eq!(plan.gun_target_indices, vec![Some(0)]);
        assert_eq!(plan.wind_strength, 3);
    }

    #[test]
    fn test_plan_data_deserializes_without_gun_target_indices() {
        // Old plans without gunTargetIndices field should default to empty vec
        let json = r#"{"plan":{"id":"abc-123","name":"Old Plan","mapId":"deadlands","weaponIds":["mortar"],"gunPositions":[{"x":10.0,"y":20.0}],"targetPositions":[{"x":30.0,"y":40.0}],"spotterPositions":[],"windDirection":null,"windStrength":0}}"#;
        let resp: FetchPlanResponse = serde_json::from_str(json).unwrap();
        let plan = resp.plan.unwrap();
        assert!(plan.gun_target_indices.is_empty());
    }

    #[test]
    fn test_plan_data_deserializes_mixed_gun_target_indices() {
        let json = r#"{"plan":{"id":"abc-123","name":"Plan","mapId":"deadlands","weaponIds":["mortar","mortar"],"gunPositions":[{"x":10.0,"y":20.0},{"x":50.0,"y":60.0}],"targetPositions":[{"x":30.0,"y":40.0}],"spotterPositions":[],"gunTargetIndices":[0,null],"windDirection":null,"windStrength":0}}"#;
        let resp: FetchPlanResponse = serde_json::from_str(json).unwrap();
        let plan = resp.plan.unwrap();
        assert_eq!(plan.gun_target_indices, vec![Some(0), None]);
    }

    #[test]
    fn test_plan_data_null() {
        let json = r#"{"plan":null}"#;
        let resp: FetchPlanResponse = serde_json::from_str(json).unwrap();
        assert!(resp.plan.is_none());
    }

    #[test]
    fn test_graphql_error_response() {
        let json = r#"{"data":null,"errors":[{"message":"Unknown weapon: foo"}]}"#;
        let resp: GraphQLResponse<CalculateResponse> = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_none());
        assert_eq!(resp.errors.unwrap()[0].message, "Unknown weapon: foo");
    }

    // --- Variable builders ---

    #[test]
    fn test_build_calculate_variables_no_wind() {
        let vars = build_calculate_variables(10.0, 20.0, 30.0, 40.0, "storm-cannon", None, None);
        assert_eq!(vars["input"]["gunPosition"]["x"], 10.0);
        assert_eq!(vars["input"]["gunPosition"]["y"], 20.0);
        assert_eq!(vars["input"]["targetPosition"]["x"], 30.0);
        assert_eq!(vars["input"]["weaponId"], "storm-cannon");
        assert!(vars["input"]["wind"].is_null());
    }

    #[test]
    fn test_build_calculate_variables_with_wind() {
        let vars =
            build_calculate_variables(0.0, 0.0, 100.0, 100.0, "mortar", Some(270.0), Some(3));
        assert_eq!(vars["input"]["wind"]["direction"], 270.0);
        assert_eq!(vars["input"]["wind"]["strength"], 3);
    }

    #[test]
    fn test_build_calculate_variables_zero_strength_wind_is_null() {
        let vars = build_calculate_variables(0.0, 0.0, 100.0, 100.0, "mortar", Some(90.0), Some(0));
        assert!(vars["input"]["wind"].is_null());
    }

    #[test]
    fn test_build_create_plan_variables() {
        let vars = build_create_plan_variables(
            "My Plan",
            "deadlands",
            &["storm-cannon".to_string()],
            &[(10.0, 20.0)],
            &[(30.0, 40.0)],
            &[],
            &[Some(0)],
            Some(180.0),
            Some(2),
        );
        assert_eq!(vars["input"]["name"], "My Plan");
        assert_eq!(vars["input"]["mapId"], "deadlands");
        assert_eq!(vars["input"]["weaponIds"][0], "storm-cannon");
        assert_eq!(vars["input"]["gunPositions"][0]["x"], 10.0);
        assert_eq!(vars["input"]["gunPositions"][0]["y"], 20.0);
        assert_eq!(vars["input"]["targetPositions"][0]["x"], 30.0);
        assert_eq!(vars["input"]["gunTargetIndices"][0], 0);
        assert_eq!(vars["input"]["windStrength"], 2);
    }

    #[test]
    fn test_build_create_plan_variables_empty() {
        let vars = build_create_plan_variables(
            "Empty Plan",
            "deadlands",
            &["mortar".to_string()],
            &[],
            &[],
            &[],
            &[],
            None,
            None,
        );
        assert_eq!(vars["input"]["gunPositions"].as_array().unwrap().len(), 0);
        assert_eq!(
            vars["input"]["targetPositions"].as_array().unwrap().len(),
            0
        );
        assert_eq!(
            vars["input"]["gunTargetIndices"].as_array().unwrap().len(),
            0
        );
        assert!(vars["input"]["windDirection"].is_null());
    }

    #[test]
    fn test_build_create_plan_variables_mixed_pairings() {
        let vars = build_create_plan_variables(
            "Mixed",
            "deadlands",
            &["mortar".to_string(), "mortar".to_string()],
            &[(10.0, 20.0), (50.0, 60.0)],
            &[(30.0, 40.0)],
            &[],
            &[Some(0), None],
            None,
            None,
        );
        assert_eq!(vars["input"]["gunTargetIndices"][0], 0);
        assert!(vars["input"]["gunTargetIndices"][1].is_null());
    }

    // --- URL builder ---

    #[test]
    fn test_build_plan_url() {
        assert_eq!(
            build_plan_url("http://localhost:8080", "abc-123"),
            "http://localhost:8080/plan/abc-123"
        );
    }

    #[test]
    fn test_build_plan_url_production() {
        assert_eq!(
            build_plan_url(
                "https://artillery.example.com",
                "550e8400-e29b-41d4-a716-446655440000"
            ),
            "https://artillery.example.com/plan/550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // --- Gun placement tracking ---

    #[test]
    fn test_track_gun_placement_response_deserializes() {
        let json = r#"{"trackGunPlacement": true}"#;
        let resp: TrackGunPlacementResponse = serde_json::from_str(json).unwrap();
        assert!(resp.track_gun_placement);
    }

    #[test]
    fn test_track_gun_placement_response_deserializes_false() {
        let json = r#"{"trackGunPlacement": false}"#;
        let resp: TrackGunPlacementResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.track_gun_placement);
    }
}

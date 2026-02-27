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
    weapon_id: &str,
    gun_x: Option<f64>,
    gun_y: Option<f64>,
    target_x: Option<f64>,
    target_y: Option<f64>,
    wind_direction: Option<f64>,
    wind_strength: Option<u32>,
) -> serde_json::Value {
    serde_json::json!({
        "input": {
            "name": name,
            "mapId": map_id,
            "weaponId": weapon_id,
            "gunX": gun_x,
            "gunY": gun_y,
            "targetX": target_x,
            "targetY": target_y,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanData {
    pub id: String,
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
        r#"query { weapons { slug faction displayName minRange maxRange accRadiusMin accRadiusMax } }"#,
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
        gun_x, gun_y, target_x, target_y, weapon_id, wind_direction, wind_strength,
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
    weapon_id: &str,
    gun_x: Option<f64>,
    gun_y: Option<f64>,
    target_x: Option<f64>,
    target_y: Option<f64>,
    wind_direction: Option<f64>,
    wind_strength: Option<u32>,
) -> Result<PlanData, String> {
    let variables = build_create_plan_variables(
        name, map_id, weapon_id, gun_x, gun_y, target_x, target_y, wind_direction, wind_strength,
    );

    let resp: CreatePlanResponse = query(
        r#"mutation CreatePlan($input: CreatePlanInput!) {
            createPlan(input: $input) {
                id name mapId weaponId gunX gunY targetX targetY
                windDirection windStrength
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

pub async fn fetch_plan(id: &str) -> Result<Option<PlanData>, String> {
    let variables = serde_json::json!({ "id": id });

    let resp: FetchPlanResponse = query(
        r#"query FetchPlan($id: ID!) {
            plan(id: $id) {
                id name mapId weaponId gunX gunY targetX targetY
                spotterX spotterY windDirection windStrength
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
        let json = r#"{"weapons":[{"slug":"storm-cannon","faction":"BOTH","displayName":"Storm Cannon","minRange":400.0,"maxRange":1000.0,"accRadiusMin":50.0,"accRadiusMax":50.0}]}"#;
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
        let json = r#"{"plan":{"id":"abc-123","name":"Test Plan","mapId":"deadlands","weaponId":"storm-cannon","gunX":100.0,"gunY":200.0,"targetX":300.0,"targetY":400.0,"spotterX":null,"spotterY":null,"windDirection":90.0,"windStrength":3}}"#;
        let resp: FetchPlanResponse = serde_json::from_str(json).unwrap();
        let plan = resp.plan.unwrap();
        assert_eq!(plan.id, "abc-123");
        assert_eq!(plan.gun_x, Some(100.0));
        assert!(plan.spotter_x.is_none());
        assert_eq!(plan.wind_strength, 3);
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
        let vars = build_calculate_variables(0.0, 0.0, 100.0, 100.0, "mortar", Some(270.0), Some(3));
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
            "My Plan", "deadlands", "storm-cannon",
            Some(10.0), Some(20.0), Some(30.0), Some(40.0),
            Some(180.0), Some(2),
        );
        assert_eq!(vars["input"]["name"], "My Plan");
        assert_eq!(vars["input"]["mapId"], "deadlands");
        assert_eq!(vars["input"]["gunX"], 10.0);
        assert_eq!(vars["input"]["windStrength"], 2);
    }

    #[test]
    fn test_build_create_plan_variables_nulls() {
        let vars = build_create_plan_variables(
            "Empty Plan", "deadlands", "mortar",
            None, None, None, None, None, None,
        );
        assert!(vars["input"]["gunX"].is_null());
        assert!(vars["input"]["targetX"].is_null());
        assert!(vars["input"]["windDirection"].is_null());
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
            build_plan_url("https://artillery.example.com", "550e8400-e29b-41d4-a716-446655440000"),
            "https://artillery.example.com/plan/550e8400-e29b-41d4-a716-446655440000"
        );
    }
}

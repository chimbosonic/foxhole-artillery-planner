use serde::{Deserialize, Serialize};

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
        .post(&api_url())
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
    let wind = match (wind_direction, wind_strength) {
        (Some(dir), Some(str)) if str > 0 => {
            serde_json::json!({ "direction": dir, "strength": str })
        }
        _ => serde_json::Value::Null,
    };

    let variables = serde_json::json!({
        "input": {
            "gunPosition": { "x": gun_x, "y": gun_y },
            "targetPosition": { "x": target_x, "y": target_y },
            "weaponId": weapon_id,
            "wind": wind
        }
    });

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
    let variables = serde_json::json!({
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
    });

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

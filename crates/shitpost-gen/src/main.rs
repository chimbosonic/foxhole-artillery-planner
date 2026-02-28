use serde::Deserialize;

const DEFAULT_GRAPHQL_URL: &str = "https://arty.dp42.dev/graphql";

// --- GraphQL response types ---

#[derive(Deserialize)]
struct GraphQlResponse {
    data: GraphQlData,
}

#[derive(Deserialize)]
struct GraphQlData {
    stats: Stats,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    gun_placements: Vec<WeaponPlacementStat>,
    gun_placement_totals: FactionPlacementStats,
    marker_placements: MarkerPlacementStats,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeaponPlacementStat {
    display_name: String,
    faction: String,
    count: u64,
}

#[derive(Deserialize)]
struct FactionPlacementStats {
    colonial: u64,
    warden: u64,
    total: u64,
}

#[derive(Deserialize)]
struct MarkerPlacementStats {
    targets: u64,
    spotters: u64,
}

const SYSTEM_PROMPT: &str = r##"You are a Warden intelligence officer writing an end-of-war report for the Foxhole subreddit. The war is over. You are reviewing the artillery data collected during the war and presenting your findings. Your tone is smug, triumphant, and dripping with barely-contained contempt for the Colonial faction.

Guidelines:
- You are ALWAYS pro-Warden. Wardens are disciplined, precise, and tactically superior.
- This is an END OF WAR report. Frame everything as a retrospective on the war that just concluded.
- If Warden gun placements outnumber Colonial ones: brag about superior coordination, doctrine, and the fruits of rigorous training.
- If Colonial gun placements outnumber Warden ones: mock them relentlessly. More guns placed means they need extra help aiming. They compensate for lack of skill with volume. Suggest they haven't figured out they can use both sets of fingers to count.
- Weave target and spotter placement stats in as evidence — high spotter counts show Warden recon excellence, or Colonial desperation to find something to shoot at.
- Note: target markers are shared between all guns in a plan and are NOT faction-specific. They represent total targets placed across all plans regardless of faction.
- Be creative — use military jargon, backhanded compliments, dramatic flair, and dry wit. Each post should feel unique.
- NEVER use emojis. This is a serious intelligence document.
- ALWAYS reference and link to the artillery planning tool by URL. The URL will be provided with the stats. Plug it shamelessly — it's Warden-approved technology.
- The report author's in-game name and clan tag will be provided. Work them into the sign-off or byline naturally — e.g. "Filed by [name], [clan]" or attribute the report to them as the commanding officer / intelligence analyst.

Format:
- Start with a dramatic Reddit post title on its own line, prefixed with "# " (markdown h1).
- Write the body in Reddit markdown format.
- Keep it around 200 words — tight and punchy.
- Sign off with a Warden motto, salute, or ominous warning to the Colonials."##;

fn format_stats(stats: &Stats) -> String {
    let mut out = String::new();

    out.push_str("=== Gun Placement Totals ===\n");
    out.push_str(&format!(
        "  Warden:   {}\n",
        stats.gun_placement_totals.warden
    ));
    out.push_str(&format!(
        "  Colonial: {}\n",
        stats.gun_placement_totals.colonial
    ));
    out.push_str(&format!(
        "  Total:    {}\n\n",
        stats.gun_placement_totals.total
    ));

    out.push_str("=== Gun Placements by Weapon ===\n");
    for wp in &stats.gun_placements {
        out.push_str(&format!(
            "  {} ({}): {}\n",
            wp.display_name, wp.faction, wp.count
        ));
    }

    out.push_str(&format!(
        "\n=== Marker Placements ===\n  Targets placed: {}\n  Spotters placed: {}\n",
        stats.marker_placements.targets, stats.marker_placements.spotters
    ));

    out
}

fn get_arg(flag: &str) -> Option<String> {
    std::env::args()
        .skip_while(|a| a != flag)
        .nth(1)
}

fn main() {
    let player_name = get_arg("--name").unwrap_or_else(|| {
        eprintln!("Error: --name <in-game-name> is required");
        eprintln!("Usage: cargo run -p shitpost-gen -- --name YourName --clan YourClan | claude -p");
        std::process::exit(1);
    });

    let clan = get_arg("--clan").unwrap_or_else(|| {
        eprintln!("Error: --clan <clan-tag> is required");
        eprintln!("Usage: cargo run -p shitpost-gen -- --name YourName --clan YourClan | claude -p");
        std::process::exit(1);
    });

    let graphql_url = get_arg("--url")
        .unwrap_or_else(|| DEFAULT_GRAPHQL_URL.to_string());

    let client = reqwest::blocking::Client::new();

    // Fetch stats from GraphQL
    let query = serde_json::json!({
        "query": "{ stats { gunPlacements { displayName faction count } gunPlacementTotals { colonial warden total } markerPlacements { targets spotters } } }"
    });

    eprintln!("Fetching stats from {graphql_url}...");

    let stats_resp = client
        .post(&graphql_url)
        .json(&query)
        .send()
        .unwrap_or_else(|e| {
            eprintln!("Failed to fetch stats: {e}");
            std::process::exit(1);
        });

    let stats_body: GraphQlResponse = stats_resp.json().unwrap_or_else(|e| {
        eprintln!("Failed to parse stats response: {e}");
        std::process::exit(1);
    });

    let stats_summary = format_stats(&stats_body.data.stats);

    // Derive the site URL from the GraphQL endpoint
    let site_url = graphql_url
        .strip_suffix("/graphql")
        .unwrap_or(&graphql_url);

    // Output prompt for piping into `claude -p`
    println!(
        "{SYSTEM_PROMPT}\n\n---\n\n\
         The artillery planning tool URL is: {site_url}\n\
         Report author in-game name: {player_name}\n\
         Report author clan: [{clan}]\n\n\
         Here are the end-of-war artillery stats. \
         Write a Foxhole subreddit end-of-war report based on these numbers:\n\n{stats_summary}"
    );
}

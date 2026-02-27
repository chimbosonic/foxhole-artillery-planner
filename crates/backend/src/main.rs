mod assets;
mod graphql;
mod storage;

use std::path::PathBuf;
use std::sync::Arc;

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{extract::State, response::Html, routing::get, Router};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use graphql::Schema;

async fn graphql_handler(
    State(schema): State<Schema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql() -> Html<String> {
    Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .finish(),
    )
}

#[tokio::main]
async fn main() {
    let assets_dir = PathBuf::from("assets");
    let loaded_assets = Arc::new(assets::Assets::load(&assets_dir));

    let db_path = PathBuf::from("data/plans.redb");
    std::fs::create_dir_all("data").expect("Failed to create data directory");
    let storage = storage::Storage::open(&db_path);

    let schema = graphql::build_schema(loaded_assets, storage);

    let app = Router::new()
        .route("/graphql", get(graphiql).post(graphql_handler))
        .nest_service("/static", ServeDir::new("assets"))
        .nest_service("/dist", ServeDir::new("dist"))
        .route("/", get(serve_index))
        .route("/plan/{id}", get(serve_index))
        .layer(CorsLayer::permissive())
        .with_state(schema);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("Server running at http://localhost:{}", port);
    println!("GraphiQL playground at http://localhost:{}/graphql", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> Html<String> {
    // Try to serve the built frontend, fall back to a simple message
    match std::fs::read_to_string("dist/index.html") {
        Ok(html) => Html(html),
        Err(_) => Html(
            r#"<!DOCTYPE html>
<html>
<head><title>Foxhole Artillery Planner</title></head>
<body>
<h1>Foxhole Artillery Planner</h1>
<p>Frontend not built yet. Visit <a href="/graphql">GraphiQL</a> to explore the API.</p>
</body>
</html>"#
                .to_string(),
        ),
    }
}

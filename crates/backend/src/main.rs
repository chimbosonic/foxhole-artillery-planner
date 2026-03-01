mod assets;
mod graphql;
mod storage;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::http::HeaderValue;
use axum::{extract::State, response::Html, routing::get, Router};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

use graphql::Schema;

async fn graphql_handler(State(schema): State<Schema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql() -> Html<String> {
    Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .finish(),
    )
}

/// Build a cache-controlled static file router.
///
/// Separated so tests can exercise the caching layer with arbitrary directories.
fn cached_static_router(dir: &Path, cache_header: &'static str) -> Router {
    let layer = SetResponseHeaderLayer::overriding(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static(cache_header),
    );
    Router::new()
        .fallback_service(ServeDir::new(dir))
        .layer(layer)
}

const CACHE_1DAY: &str = "public, max-age=86400, must-revalidate";
const CACHE_IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Build the full application router.
fn build_app(schema: Schema) -> Router {
    // Static file routers are stateless — merge them before adding app state
    let static_files = Router::new()
        .nest(
            "/static",
            cached_static_router(Path::new("assets"), CACHE_1DAY),
        )
        .nest(
            "/dist",
            cached_static_router(Path::new("dist"), CACHE_IMMUTABLE),
        )
        .nest(
            "/assets",
            cached_static_router(Path::new("dist/assets"), CACHE_IMMUTABLE),
        );

    Router::new()
        .route("/graphql", get(graphiql).post(graphql_handler))
        .route("/", get(serve_index))
        .route("/plan/{id}", get(serve_index))
        .with_state(schema)
        .merge(static_files)
        .layer(CorsLayer::permissive())
}

#[tokio::main]
async fn main() {
    let assets_dir =
        PathBuf::from(std::env::var("ASSETS_DIR").unwrap_or_else(|_| "assets".to_string()));
    let loaded_assets = Arc::new(assets::Assets::load(&assets_dir));

    let db_path =
        PathBuf::from(std::env::var("DB_PATH").unwrap_or_else(|_| "data/plans.redb".to_string()));
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }
    let storage = storage::Storage::open(&db_path);

    let schema = graphql::build_schema(loaded_assets, storage);
    let app = build_app(schema);

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    /// Build a test app that serves files from the given temp directories.
    fn test_app(assets_dir: &Path, dist_dir: &Path, dist_assets_dir: &Path) -> Router {
        Router::new()
            .nest("/static", cached_static_router(assets_dir, CACHE_1DAY))
            .nest("/dist", cached_static_router(dist_dir, CACHE_IMMUTABLE))
            .nest(
                "/assets",
                cached_static_router(dist_assets_dir, CACHE_IMMUTABLE),
            )
    }

    /// Create a temp dir with a test file and return the dir path.
    fn temp_dir_with_file(file_name: &str, content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(file_name), content).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_static_assets_have_1day_cache() {
        let assets_dir = temp_dir_with_file("maps.json", "[]");
        let dist_dir = temp_dir_with_file("index.html", "<html></html>");
        let dist_assets_dir = temp_dir_with_file("app.js", "console.log()");

        let app = test_app(assets_dir.path(), dist_dir.path(), dist_assets_dir.path());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/maps.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("cache-control").unwrap(),
            "public, max-age=86400, must-revalidate"
        );
    }

    #[tokio::test]
    async fn test_dist_bundles_have_immutable_cache() {
        let assets_dir = temp_dir_with_file("maps.json", "[]");
        let dist_dir = temp_dir_with_file("app-abc123.js", "bundle()");
        let dist_assets_dir = temp_dir_with_file("style.css", "body{}");

        let app = test_app(assets_dir.path(), dist_dir.path(), dist_assets_dir.path());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/dist/app-abc123.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("cache-control").unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn test_dist_assets_have_immutable_cache() {
        let assets_dir = temp_dir_with_file("maps.json", "[]");
        let dist_dir = temp_dir_with_file("index.html", "<html></html>");
        let dist_assets_dir = temp_dir_with_file("style-xyz.css", "body{}");

        let app = test_app(assets_dir.path(), dist_dir.path(), dist_assets_dir.path());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/style-xyz.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("cache-control").unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn test_missing_static_file_returns_404() {
        let assets_dir = temp_dir_with_file("maps.json", "[]");
        let dist_dir = temp_dir_with_file("index.html", "<html></html>");
        let dist_assets_dir = temp_dir_with_file("app.js", "");

        let app = test_app(assets_dir.path(), dist_dir.path(), dist_assets_dir.path());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/nonexistent.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_static_and_dist_have_different_cache_policies() {
        let assets_dir = temp_dir_with_file("data.json", "{}");
        let dist_dir = temp_dir_with_file("bundle.js", "x");
        let dist_assets_dir = temp_dir_with_file("a.css", "");

        let app = test_app(assets_dir.path(), dist_dir.path(), dist_assets_dir.path());

        let static_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/static/data.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let dist_resp = app
            .oneshot(
                Request::builder()
                    .uri("/dist/bundle.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let static_cc = static_resp
            .headers()
            .get("cache-control")
            .unwrap()
            .to_str()
            .unwrap();
        let dist_cc = dist_resp
            .headers()
            .get("cache-control")
            .unwrap()
            .to_str()
            .unwrap();

        assert_ne!(static_cc, dist_cc);
        assert!(static_cc.contains("max-age=86400"));
        assert!(dist_cc.contains("max-age=31536000"));
    }
}

use summer_web::Router;
use std::time::Duration;
use summer_web::middleware::cors::CorsLayer;
use summer_web::axum::http::{header, HeaderValue, Method};

const CORS_ALLOW_ORIGINS: &str = env!("CORS_ALLOW_ORIGINS");

pub fn install(router: Router) -> Router {
    let cors = build_cors_layer().unwrap_or_else(|_| CorsLayer::permissive());
    router.layer(cors)
}

fn build_cors_layer() -> anyhow::Result<CorsLayer> {
    let mut allow_origins: Vec<HeaderValue> = Vec::new();
    for origin in CORS_ALLOW_ORIGINS.split(',') {
        allow_origins.push(origin.parse()?);
    }
    Ok(CorsLayer::new()
        .allow_origin(allow_origins)
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_private_network(true)
        .max_age(Duration::from_secs(3600)))
}

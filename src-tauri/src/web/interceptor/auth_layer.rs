use std::sync::Arc;
use summer_web::Router;
use std::sync::OnceLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use summer_web::axum::middleware::{self, Next};
use summer_web::axum::extract::{Request, State};
use crate::service::service_base_api::BaseApiService;
use summer_web::axum::response::{IntoResponse, Response};
use crate::model::resp::resp_model::{ResultCode, ResultModel};
use crate::service::service_account_token::AccountTokenService;

const AUTH_REQUIRE_PATHS: &str = env!("AUTH_REQUIRE_PATHS");
const AUTH_REQUIRE_PREFIXES: &str = env!("AUTH_REQUIRE_PREFIXES");
const AUTH_WARMUP_ALLOW_PATHS: &str = env!("AUTH_WARMUP_ALLOW_PATHS");

static REQUIRE_PATHS: OnceLock<HashSet<&'static str>> = OnceLock::new();
static REQUIRE_PREFIXES: OnceLock<Vec<&'static str>> = OnceLock::new();
static WARMUP_ALLOW_PATHS: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn require_paths() -> &'static HashSet<&'static str> {
    REQUIRE_PATHS.get_or_init(|| AUTH_REQUIRE_PATHS.split(',').filter(|item| !item.is_empty()).collect())
}

fn require_prefixes() -> &'static Vec<&'static str> {
    REQUIRE_PREFIXES.get_or_init(|| AUTH_REQUIRE_PREFIXES.split(',').filter(|item| !item.is_empty()).collect())
}

fn warmup_allow_paths() -> &'static HashSet<&'static str> {
    WARMUP_ALLOW_PATHS.get_or_init(|| AUTH_WARMUP_ALLOW_PATHS.split(',').filter(|item| !item.is_empty()).collect())
}

fn requires_auth(path: &str) -> bool {
    require_paths().contains(path) || require_prefixes().iter().any(|prefix| path.starts_with(prefix))
}

#[derive(Clone)]
pub struct AuthState {
    pub ready: Arc<AtomicBool>,
}

pub fn install(router: Router, ready: Arc<AtomicBool>) -> Router {
    let state = AuthState { ready };
    router.layer(middleware::from_fn_with_state(state, gate))
}

async fn gate(State(state): State<AuthState>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    if !state.ready.load(Ordering::Acquire) && !warmup_allow_paths().contains(path) {
        return ResultModel::<()> { code: 503, message: Some("system warming up".to_string()), data: None }.into_response();
    }
    if !requires_auth(path) {
        return next.run(req).await;
    }
    let header = req.headers().get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    if header.is_none() || AccountTokenService::api_token().as_deref() != header {
        return ResultModel::<()>::fail_code(ResultCode::Unauthorized).into_response();
    }
    next.run(req).await
}

use axum::extract::FromRef;
use axum::{routing, Router};
use reqwest::Client;

use crate::utils::DbPool;

use super::signup::SignupManager;
use super::{redirect, signup};

#[derive(Clone)]
pub struct AppState {
    pub signup_manager: SignupManager,
    pub db_pool: DbPool,
    pub http_client: Client,
    pub app_variables: AppVariables,
}

#[derive(Clone)]
pub struct AppVariables {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl FromRef<AppState> for DbPool {
    fn from_ref(input: &AppState) -> Self {
        input.db_pool.clone()
    }
}

impl FromRef<AppState> for SignupManager {
    fn from_ref(input: &AppState) -> Self {
        input.signup_manager.clone()
    }
}

pub fn app(db_pool: DbPool, app_variables: AppVariables) -> Router {
    Router::new()
        .route("/auth/signup", routing::get(signup::handle_signup))
        .route("/auth/redirect", routing::get(redirect::handle_redirect))
        .with_state(AppState {
            signup_manager: SignupManager::new(),
            db_pool,
            http_client: reqwest::Client::new(),
            app_variables,
        })
}

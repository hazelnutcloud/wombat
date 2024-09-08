use axum::{routing, Router};

use super::signup::SignupManager;

pub fn get_app() -> Router {
    Router::new()
        .route("/auth/signup", routing::get(super::signup::handle_signup))
        .route(
            "/auth/redirect",
            routing::get(super::redirect::handle_redirect),
        )
        .with_state(SignupManager::new())
}

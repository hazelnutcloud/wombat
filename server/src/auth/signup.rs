use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use axum::{extract::State, response::Redirect};
use rand::{distributions::Alphanumeric, Rng};
use serenity::all::Permissions;

use super::app::AppState;

#[derive(Clone)]
pub struct SignupManager {
    signup_requests: Arc<Mutex<HashSet<String>>>,
}

impl SignupManager {
    pub fn new() -> SignupManager {
        SignupManager {
            signup_requests: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn add_request(&self) -> String {
        let digest: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let mut signup_requests = self.signup_requests.lock().unwrap();
        signup_requests.insert(digest.clone());

        digest
    }

    pub fn consume_request(&self, digest: &str) -> bool {
        let mut signup_requests = self.signup_requests.lock().unwrap();
        signup_requests.remove(digest)
    }
}

impl Default for SignupManager {
    fn default() -> Self {
        SignupManager::new()
    }
}

pub async fn handle_signup(State(app_state): State<AppState>) -> Redirect {
    let digest = app_state.signup_manager.add_request();
    let app_variables = app_state.app_variables;
    let auth_url = get_auth_url(app_variables.client_id, app_variables.redirect_uri, digest);

    Redirect::to(&auth_url)
}

fn get_auth_url(client_id: String, redirect_uri: String, digest: String) -> String {
    let permissions = Permissions::from_bits_truncate(
        Permissions::SEND_MESSAGES.bits()
            | Permissions::SEND_MESSAGES_IN_THREADS.bits()
            | Permissions::ATTACH_FILES.bits(),
    );

    format!(
      "https://discord.com/oauth2/authorize?client_id={}&permissions={}&response_type=code&redirect_uri={}&integration_type=0&scope=identify+bot&state={}",
      client_id,
      permissions.bits(),
      urlencoding::encode(&redirect_uri),
      urlencoding::encode(&digest)
    )
}

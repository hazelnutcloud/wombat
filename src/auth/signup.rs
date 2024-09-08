use std::{
    collections::HashSet,
    env,
    sync::{Arc, Mutex},
};

use axum::{extract::State, response::Redirect};
use rand::{distributions::Alphanumeric, Rng};
use serenity::all::Permissions;

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
}

pub async fn handle_signup(State(signup_manager): State<SignupManager>) -> Redirect {
    let digest = signup_manager.add_request();
    let auth_url = get_auth_url(&digest);

    Redirect::to(&auth_url)
}

fn get_auth_url(digest: &str) -> String {
    let client_id = env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID not set");
    let redirect_uri =
        env::var("REDIRECT_URI").unwrap_or("http://localhost:8080/auth/redirect".into());
    let permissions = Permissions::from_bits_truncate(
        Permissions::SEND_MESSAGES.bits()
            | Permissions::SEND_MESSAGES_IN_THREADS.bits()
            | Permissions::ATTACH_FILES.bits(),
    );

    format!(
      "https://discord.com/oauth2/authorize?client_id={}&permissions={}&response_type=code&redirect_uri={}&integration_type=0&scope=identify+email+bot&state={}",
      client_id,
      permissions.bits(),
      urlencoding::encode(&redirect_uri),
      urlencoding::encode(digest)
    )
}

use crate::{models::*, protocol::KEY_WIDTH_BYTES, utils};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::app::AppState;

#[derive(Deserialize)]
pub struct RedirectParams {
    state: String,
    code: String,
}

#[derive(Serialize)]
struct TokenRequestBody {
    grant_type: String,
    code: String,
    redirect_uri: String,
}

impl TokenRequestBody {
    fn from_code(code: String, redirect_uri: String) -> TokenRequestBody {
        TokenRequestBody {
            grant_type: "authorization_code".into(),
            code,
            redirect_uri,
        }
    }
}

#[derive(Deserialize)]
struct TokenResponseBody {
    access_token: String,
    guild: Guild,
}

#[derive(Deserialize)]
struct Guild {
    id: String,
}

#[derive(Deserialize, Debug)]
struct UserResponseBody {
    id: String,
}

pub async fn handle_redirect(
    State(app_state): State<AppState>,
    Query(RedirectParams { state, code }): Query<RedirectParams>,
) -> Result<Html<String>, StatusCode> {
    if !app_state.signup_manager.consume_request(&state) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let TokenResponseBody {
        access_token,
        guild,
    } = exchange_token(&app_state, &code).await?;
    let user = fetch_user_info(&app_state, &access_token).await?;

    let mut db_conn = match app_state.db_pool.get() {
        Ok(db_conn) => db_conn,
        Err(e) => return Err(log_error("getting connection from pool", e)),
    };

    use crate::schema::discord_guilds::dsl::*;
    use crate::schema::secret_keys::dsl::*;
    use crate::schema::users::dsl::*;

    let secret_key = db_conn
        .transaction::<String, diesel::result::Error, _>(|conn| {
            let existing_user: Vec<User> = users
                .select(User::as_select())
                .filter(discord_id.eq(user.id.clone()))
                .load(conn)?;

            let new_or_existing_user_id = if existing_user.is_empty() {
                let new_user_id = Ulid::new().to_string();
                diesel::insert_into(users)
                    .values(&NewUser {
                        id: new_user_id.clone(),
                        discord_id: user.id,
                    })
                    .execute(conn)?;
                new_user_id
            } else {
                existing_user.first().unwrap().id.to_owned()
            };

            let existing_guild: Vec<DiscordGuild> = discord_guilds
                .select(DiscordGuild::as_select())
                .filter(guild_id.eq(guild.id.clone()))
                .load(conn)?;
            if existing_guild.is_empty() {
                diesel::insert_into(discord_guilds)
                    .values(&NewDiscordGuild {
                        id: Ulid::new().to_string(),
                        guild_id: guild.id,
                        manager_user_id: new_or_existing_user_id.clone(),
                    })
                    .execute(conn)?;
            }

            let secret_key = utils::generate_secret_key(KEY_WIDTH_BYTES);
            let key_hash = utils::hash_key(&secret_key);

            diesel::insert_into(secret_keys)
                .values(&NewSecretKey {
                    id: Ulid::new().to_string(),
                    secret_key_hash: key_hash,
                    user_id: new_or_existing_user_id,
                })
                .execute(conn)?;

            Ok(secret_key)
        })
        .map_err(|e| log_error("inserting user", e))?;

    Ok(generate_key_html(secret_key))
}

async fn fetch_user_info(
    app_state: &AppState,
    access_token: &str,
) -> Result<UserResponseBody, StatusCode> {
    app_state
        .http_client
        .get("https://discord.com/api/v10/users/@me")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| log_error("sending user info request", e))?
        .error_for_status()
        .map_err(|e| log_error("user info request status", e))?
        .json()
        .await
        .map_err(|e| log_error("parsing user info response", e))
}

async fn exchange_token(app_state: &AppState, code: &str) -> Result<TokenResponseBody, StatusCode> {
    app_state
        .http_client
        .post("https://discord.com/api/oauth2/token")
        .basic_auth(
            app_state.app_variables.client_id.clone(),
            Some(app_state.app_variables.client_secret.clone()),
        )
        .form(&TokenRequestBody::from_code(
            code.to_string(),
            app_state.app_variables.redirect_uri.clone(),
        ))
        .send()
        .await
        .map_err(|e| log_error("sending token request", e))?
        .error_for_status()
        .map_err(|e| log_error("token request status", e))?
        .json()
        .await
        .map_err(|e| log_error("parsing token response", e))
}

fn log_error(context: &str, error: impl std::fmt::Display) -> StatusCode {
    tracing::error!("Error while {context}: {error}");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn generate_key_html(secret_key: String) -> Html<String> {
    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Secret Key</title>
  <style>
      body {{
          font-family: Arial, sans-serif;
          display: flex;
          justify-content: center;
          align-items: center;
          height: 100vh;
          margin: 0;
          background-color: #1e1e1e;
          color: #e0e0e0;
      }}
      .container {{
          background-color: #2d2d2d;
          padding: 2rem;
          border-radius: 8px;
          box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
          text-align: center;
      }}
      .secret-key {{
          font-family: monospace;
          font-size: 1.2rem;
          background-color: #3d3d3d;
          padding: 0.5rem;
          border-radius: 4px;
          margin: 1rem 0;
      }}
      .warning {{
          color: #ff6b6b;
          font-weight: bold;
      }}
  </style>
</head>
<body>
  <div class="container">
      <h1>Your Secret Key</h1>
      <div class="secret-key">{}</div>
      <p class="warning">Copy and paste this into your terminal. It won't be shown again.</p>
  </div>
</body>
</html>
"#,
        secret_key
    ))
}

use crate::{connection::DiscordFetchRequest, utils::DbPool};

use anyhow::bail;
use axum::body::Bytes;
use diesel::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Uri};
use poise::command;
use serde_json::Value;
use tokio::sync::{mpsc::Sender, oneshot};

pub struct Data {
    pub req_tx: Sender<DiscordFetchRequest>,
    pub db_pool: DbPool,
}

type Error = anyhow::Error;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, poise::ChoiceParameter)]
enum MethodInput {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

impl From<MethodInput> for Method {
    fn from(value: MethodInput) -> Self {
        match value {
            MethodInput::GET => Method::GET,
            MethodInput::POST => Method::POST,
            MethodInput::PUT => Method::PUT,
            MethodInput::DELETE => Method::DELETE,
            MethodInput::PATCH => Method::PATCH,
        }
    }
}

#[command(prefix_command)]
pub async fn fetch(
    ctx: Context<'_>,
    url: String,
    method: Option<MethodInput>,
    body: Option<poise::CodeBlock>,
) -> Result<(), Error> {
    let uri = url.parse::<Uri>();

    let uri = match uri {
        Ok(uri) => uri,
        Err(e) => {
            return ctx
                .reply(format!("Invalid url: {e}"))
                .await
                .and(Ok(()))
                .map_err(|e| e.into());
        }
    };

    let authority = uri.authority().unwrap().clone();

    let body = match body {
        Some(body) => match serde_json::from_str::<Value>(&body.code) {
            Ok(_) => Full::from(Bytes::from(body.code)),
            Err(e) => {
                return ctx
                    .reply(format!("Invalid body: {e}"))
                    .await
                    .and(Ok(()))
                    .map_err(|e| e.into())
            }
        },
        None => Full::new(Bytes::new()),
    };

    let request = Request::builder()
        .uri(uri)
        .header(hyper::header::HOST, authority.as_str())
        .method(match method {
            Some(method) => method,
            None => MethodInput::GET,
        })
        .body(body)?;

    let user_id = match ctx.guild_id() {
        Some(this_guild_id) => {
            use crate::schema::discord_guilds::dsl::*;

            let mut db_conn = { ctx.data().db_pool.get()? };

            let user_id: Option<String> = discord_guilds
                .select(manager_user_id)
                .filter(guild_id.eq(this_guild_id.get().to_string()))
                .first(&mut db_conn)
                .optional()?;

            match user_id {
                Some(user_id) => user_id,
                None => {
                    ctx.reply("unexpected error occured :(").await?;
                    bail!("Unexpected guild id not found in db: {this_guild_id}")
                }
            }
        }
        None => ctx.author().id.get().to_string(),
    };

    let (req_tx, req_rx) = oneshot::channel();

    {
        ctx.data().req_tx.send((user_id, request, req_tx)).await?;
    }

    let response = req_rx.await?;

    let response = match response {
        Some(response) => {
            let body = response.collect().await?;
            String::from_utf8(body.to_bytes().to_vec())?
        }
        None => {
            return ctx
                .reply("Wombat client not connected!")
                .await
                .and(Ok(()))
                .map_err(|e| e.into())
        }
    };

    ctx.reply(response).await?;

    Ok(())
}

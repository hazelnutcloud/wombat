use axum::body::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, header::CONTENT_TYPE, Method, Request, Response, Uri};
use jsonpath_lib::Compiled;
use poise::command;
use serde_json::Value;
use tokio::sync::{mpsc::Sender, oneshot};

pub type DiscordFetchRequest = (
    Request<Full<Bytes>>,
    oneshot::Sender<Result<Response<Incoming>, String>>,
);

pub struct Data {
    pub req_tx: Sender<DiscordFetchRequest>,
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
    headers: Option<poise::KeyValueArgs>,
    body: Option<poise::CodeBlock>,
    json_path: Option<String>,
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

    let mut request_builder = Request::builder()
        .uri(uri)
        .header(hyper::header::HOST, authority.as_str())
        .method(match method {
            Some(method) => method,
            None => MethodInput::GET,
        });

    if let Some(headers) = headers {
        for (key, value) in headers.0.iter() {
            request_builder = request_builder.header(key, value);
        }
    }

    let request = request_builder.body(body)?;

    let (res_tx, res_rx) = oneshot::channel();

    {
        ctx.data().req_tx.send((request, res_tx)).await?;
    }

    let response = res_rx.await?;

    let response = match response {
        Ok(response) => response,
        Err(e) => return ctx.reply(e).await.and(Ok(())).map_err(|e| e.into()),
    };

    if !response.status().is_success() {
        return ctx
            .reply(format!(
                "Fetch unsuccessful, received code {}",
                response.status().as_str()
            ))
            .await
            .and(Ok(()))
            .map_err(|e| e.into());
    }

    let content_type = response.headers().get(CONTENT_TYPE);

    match content_type {
        Some(content_type) => {
            let mime_type: mime::Mime = content_type.to_str()?.parse()?;

            match (mime_type.type_(), mime_type.subtype()) {
                (mime::TEXT, _) => handle_text(response, ctx).await,
                (mime::APPLICATION, mime::JSON) => handle_json(response, ctx, json_path).await,
                _ => ctx
                    .reply("Unsupported response format!")
                    .await
                    .and(Ok(()))
                    .map_err(|e| e.into()),
            }
        }
        None => todo!(),
    }
}

async fn extract_text(mut response: Response<Incoming>) -> anyhow::Result<String> {
    String::from_utf8(response.body_mut().collect().await?.to_bytes().to_vec())
        .map_err(|e| e.into())
}

async fn handle_text(response: Response<Incoming>, ctx: Context<'_>) -> anyhow::Result<()> {
    let text = extract_text(response).await?;

    ctx.reply(text).await.and(Ok(())).map_err(|e| e.into())
}

async fn handle_json(
    response: Response<Incoming>,
    ctx: Context<'_>,
    json_path: Option<String>,
) -> anyhow::Result<()> {
    let text = extract_text(response).await?;
    let value: Value = serde_json::from_str(&text)?;
    let value = if let Some(path) = json_path {
        let path = Compiled::compile(&path).map_err(anyhow::Error::msg)?;
        let result = path.select(&value)?;
        if result.is_empty() {
            return ctx
                .reply("No value found for the given selector")
                .await
                .and(Ok(()))
                .map_err(|e| e.into());
        }
        serde_json::to_value(result)?
    } else {
        value
    };

    let text = serde_json::to_string_pretty(&value)?;
    let text = format!("```json\n{text}```");

    ctx.reply(text).await.and(Ok(())).map_err(|e| e.into())
}

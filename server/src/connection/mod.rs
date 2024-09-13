use std::sync::Arc;

use axum::body::Bytes;
use dashmap::DashMap;
use http_body_util::Full;
use hyper::{body::Incoming, client::conn::http2::SendRequest, Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

pub struct TunnelManager {
    connection_map: Arc<DashMap<String, SendRequest<Full<Bytes>>>>,
    conn_rx: Receiver<(String, TcpStream)>,
    req_rx: Receiver<DiscordFetchRequest>,
}

pub type DiscordFetchRequest = (
    String,
    Request<Full<Bytes>>,
    oneshot::Sender<Option<Response<Incoming>>>,
);

impl TunnelManager {
    pub fn new() -> (
        TunnelManager,
        Sender<(String, TcpStream)>,
        Sender<DiscordFetchRequest>,
    ) {
        let (conn_tx, conn_rx) = mpsc::channel(100);
        let (req_tx, req_rx) = mpsc::channel(100);
        (
            TunnelManager {
                connection_map: Arc::new(DashMap::new()),
                conn_rx,
                req_rx,
            },
            conn_tx,
            req_tx,
        )
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let connection_map = self.connection_map.clone();

        let conn_loop_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            while let Some((user_id, conn)) = self.conn_rx.recv().await {
                let map = connection_map.clone();

                tokio::spawn(async move {
                    let io = TokioIo::new(conn);
                    let exec = TokioExecutor::new();
                    let user_id_clone = user_id.clone();

                    match hyper::client::conn::http2::handshake(exec, io).await {
                        Ok((sender, conn)) => {
                            tokio::spawn(async move {
                                if let Err(e) = conn.await {
                                    tracing::error!(
                                        "error with http connection from user {user_id_clone}: {e}"
                                    );
                                }
                            });

                            map.insert(user_id, sender);
                        }
                        Err(e) => {
                            tracing::error!("error while performing http handshake: {e}")
                        }
                    }
                });
            }
            Ok(())
        });

        let connection_map = self.connection_map;

        let req_loop_handle: JoinHandle<()> = tokio::spawn(async move {
            while let Some((user_id, req, tx)) = self.req_rx.recv().await {
                let mut sender = {
                    match connection_map.get(&user_id) {
                        Some(sender) => sender.clone(),
                        None => {
                            tx.send(None).unwrap();
                            continue;
                        }
                    }
                };

                match sender.send_request(req).await {
                    Ok(response) => tx.send(Some(response)).unwrap(),
                    Err(e) => {
                        tracing::error!("error sending request for {}: {}", &user_id, e);
                        tx.send(None).unwrap();
                    }
                }
            }
        });

        tokio::select! {
          conn_loop_handle = conn_loop_handle => {
            match conn_loop_handle {
                Ok(res) => res,
                Err(e) => Err(e.into()),
            }
          },
          req_loop_handle = req_loop_handle => req_loop_handle.map_err(|e| e.into())
        }
    }
}

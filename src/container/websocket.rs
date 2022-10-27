use std::{collections::HashMap, sync::Arc};

use async_tungstenite::{
    tokio::accept_async,
    tungstenite::{self, Message},
};
use futures::StreamExt;
use parking_lot::Mutex;
use sai::{Component, ComponentLifecycle};
use tokio::{
    net::{unix::SocketAddr, TcpListener, TcpStream},
    sync::mpsc,
};

#[derive(Component)]
#[lifecycle]
pub struct WebSocket {}

#[async_trait::async_trait]
impl ComponentLifecycle for WebSocket {
    async fn start(&mut self) {
        async fn handle_connection(
            peers: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Message>>>>,
            peer: SocketAddr,
            stream: TcpStream,
        ) -> tungstenite::Result<()> {
            let mut stream = accept_async(stream).await.unwrap();

            let (outgoing, incoming) = stream.split();

            Ok(())
            // 1. 클라이언트에서 구독(각 작품)을 하면 주는 방식
            // 2. 클라이언트에서 연결을 하면 전부 쏴주는 방식
            // 2-1. 클라이언트에서 이거 좀 주세요 하고 요청을 할 수도 있나?
        }
    }

    async fn stop(&mut self) {}
}

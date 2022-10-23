use sai::{Component, ComponentLifecycle};
use tokio::sync::{mpsc, Mutex};

use crate::container;

/// id, page, total_page, error
type ErrMsg = (Option<u32>, Option<usize>, Option<usize>, crate::Error);

#[derive(Component)]
#[lifecycle]
pub struct Channel {
    id_tx: Option<mpsc::Sender<u32>>,
    id_rx: Option<Mutex<mpsc::Receiver<u32>>>,

    about_tx: Option<mpsc::Sender<crawler::model::Gallery>>,
    about_rx: Option<Mutex<mpsc::Receiver<crawler::model::Gallery>>>,

    sync_tx: Option<mpsc::Sender<container::SyncKind>>,
    sync_rx: Option<Mutex<mpsc::Receiver<container::SyncKind>>>,

    // id, page, error
    err_tx: Option<mpsc::Sender<ErrMsg>>,
    err_rx: Option<Mutex<mpsc::Receiver<ErrMsg>>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Channel {
    async fn start(&mut self) {
        // TODO: channel 생성
    }
}

impl Channel {
    pub fn id_tx(&self) -> mpsc::Sender<u32> {
        self.id_tx.clone().unwrap()
    }

    pub async fn id_recv(&self) -> u32 {
        let mut rx = self.id_rx.as_ref().unwrap().lock().await;
        rx.recv().await.expect("closed channel")
    }

    pub fn about_tx(&self) -> mpsc::Sender<crawler::model::Gallery> {
        self.about_tx.clone().unwrap()
    }

    pub async fn about_recv(&self) -> crawler::model::Gallery {
        let mut rx = self.about_rx.as_ref().unwrap().lock().await;
        rx.recv().await.expect("closed channel")
    }

    pub fn sync_tx(&self) -> mpsc::Sender<container::SyncKind> {
        self.sync_tx.clone().unwrap()
    }

    pub async fn sync_recv(&self) -> container::SyncKind {
        let mut rx = self.sync_rx.as_ref().unwrap().lock().await;
        rx.recv().await.expect("closed channel")
    }

    pub fn err_tx(&self) -> mpsc::Sender<ErrMsg> {
        self.err_tx.clone().unwrap()
    }

    pub async fn err_recv(&self) -> ErrMsg {
        let mut rx = self.err_rx.as_ref().unwrap().lock().await;
        rx.recv().await.expect("closed channel")
    }
}

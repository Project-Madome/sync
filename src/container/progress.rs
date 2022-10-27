use std::{collections::HashMap, fmt::Debug, hash::Hash};

use sai::{Component, ComponentLifecycle, Injected};
use tokio::sync::{mpsc, oneshot};

use crate::container::{self, SyncKind};

#[derive(Component)]
#[lifecycle]
pub struct Progress {
    #[injected]
    channel: Injected<container::Channel>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Progress {
    async fn start(&mut self) {
        let (stop_sender, rx) = oneshot::channel();
        let (tx, mut stop_receiver) = mpsc::channel(1);

        self.tx.replace(tx);
        self.rx.replace(rx);

        let channel = self.channel.clone();

        tokio::spawn(async move {
            let mut store = HashMap::<Key, usize>::new();

            loop {
                let received = tokio::select! {
                    _ = stop_receiver.recv() => {
                        break;
                    }
                    received = channel.progress_recv() => {
                        received
                    }
                };

                match received {
                    ProgressKind::Image(id, _page, total) => {
                        let count = *store
                            .entry(Key::Image(id))
                            .and_modify(|x| *x += 1)
                            .or_insert(1);

                        if count >= total {
                            store.remove(&Key::Image(id));

                            channel.sync_tx().send(SyncKind::Release(id)).await.unwrap();
                        }

                        let percentage = count as f32 / total as f32;

                        log::info!(
                            "image_progress;id={id};count={count}/{total};{:.2}%",
                            percentage * 100.0
                        );
                        // TODO: progress가 필요한 곳에 쏴주거나 아니면 서버에 전송?
                        // 필요한 곳이 서버 말고는 없는지 생각해보기
                    }
                }
            }

            log::debug!("shutdown_progress");

            stop_sender.send(()).unwrap();
        });
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}

pub enum ProgressKind {
    // Image(id, page, total_page, image)
    Image(u32, usize, usize),
}

impl Debug for ProgressKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Self::Image(id, page, total) => format!("Image({id}, {page}, {total})"),
        };

        write!(f, "ProgressKind::{x}")
    }
}

enum Key {
    Image(u32),
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Image(x) => {
                matches!(other, Self::Image(y) if x == y)
            }
        }
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Image(id) => {
                "image".hash(state);
                id.hash(state);
            }
        }
    }
}

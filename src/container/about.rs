use sai::{Component, ComponentLifecycle, Injected};
use tokio::sync::{mpsc, oneshot};

use crate::{container, SendError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Crawler: {0}")]
    Crawler(#[from] crawler::Error),
}

#[derive(Component)]
#[lifecycle]
pub struct About {
    #[injected]
    channel: Injected<container::Channel>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for About {
    async fn start(&mut self) {
        let (stop_sender, rx) = oneshot::channel();
        let (tx, mut stop_receiver) = mpsc::channel(1);

        self.tx.replace(tx);
        self.rx.replace(rx);

        let channel = self.channel.clone();

        tokio::spawn(async move {
            loop {
                let id = tokio::select! {
                    _ = stop_receiver.recv() => {
                        break;
                    }
                    id = channel.id_recv() => {
                        id
                    }
                };

                log::info!("parse_about;id={id}");

                if let Some(about) = parse_gallery(id).to(id, channel.err_tx()).await {
                    log::debug!("parse_about;send_about;id={id}");
                    channel
                        .sync_tx()
                        .send(container::SyncKind::About(about))
                        .await
                        .expect("closed channel");
                }
            }

            log::debug!("shutdown_about");

            stop_sender.send(()).unwrap();
        });
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}

async fn parse_gallery(id: u32) -> Result<crawler::model::Gallery, Error> {
    let r = crawler::gallery::parse(id).await?;
    Ok(r)
}

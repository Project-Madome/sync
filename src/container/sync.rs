use std::fmt::Debug;

use bytes::Bytes;
use madome_sdk::api::{file, library};
use sai::{Component, ComponentLifecycle, Injected};
use tokio::sync::{mpsc, oneshot};

use crate::{container, SendError};

#[derive(Component)]
#[lifecycle]
pub struct Sync {
    #[injected]
    channel: Injected<container::Channel>,

    #[injected]
    token: Injected<container::Token>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Sync {
    async fn start(&mut self) {
        let (stop_sender, rx) = oneshot::channel();
        let (tx, mut stop_receiver) = mpsc::channel(1);

        self.tx.replace(tx);
        self.rx.replace(rx);

        let Self { channel, token, .. } = self;

        loop {
            let received = tokio::select! {
                _ = stop_receiver.recv() => {
                    break;
                }
                received = channel.sync_recv() => {
                    received
                }
            };

            match received {
                // TODO: 이미지를 업로드 하기 전에 작품 정보를 업로드 하는데
                // library서버에서 해당 작품이 이미지 업로드가 된 작품인지 아닌지를 구별할 방법이 필요함
                //
                // 처음에 올릴 때는 pre-release 같은 느낌으로 외부에 변경을 덜 주는 방식으로 library 서버에서 변경하고,
                // 이후에 이미지 업로드가 다 됐따 하면 draft하는 방식
                //
                // draft 하는 도중에도 에러가 날 수 있으니까 이것도 저장해놨따가 아무것도 안할때 틈틈이 시도
                SyncKind::About(about) => {
                    let r = sync_about(token, &about)
                        .to(about.id, channel.err_tx())
                        .await
                        .is_some();

                    if r {
                        channel.about_tx().send(about).await.unwrap();
                    }
                }

                // TODO: progress 구현
                SyncKind::Image(id, page, total_page, image, buf) => match image.kind() {
                    crawler::image::ImageKind::Thumbnail => {
                        let _r = sync_thumbnail(token, id, image, buf)
                            .too(id, 0, total_page, channel.err_tx())
                            .await
                            .is_some();
                    }

                    crawler::image::ImageKind::Original => {
                        let _r = sync_image(token, id, page, image, buf)
                            .too(id, page, total_page, channel.err_tx())
                            .await
                            .is_some();
                    }
                },
            }
        }

        stop_sender.send(()).unwrap()
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}

pub enum SyncKind {
    About(crawler::model::Gallery),
    // id, page, total_page, image, buf
    Image(u32, usize, usize, crawler::image::Image, Bytes),
}

impl Debug for SyncKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Self::About(_) => "About",
            Self::Image(_, _, _, _, _) => "Image",
        };

        write!(f, "SyncKind::{x}")
    }
}

async fn sync_about(
    token: &container::Token,
    about: &crawler::model::Gallery,
) -> crate::Result<()> {
    let tags = about
        .tags
        .clone()
        .into_iter()
        .map(|tag| (tag.kind.to_string(), tag.name))
        .collect::<Vec<_>>();

    library::add_book(
        "https://beta.api.madome.app",
        token,
        about.id,
        about.title.clone(),
        about.kind.clone(),
        about.files.len(),
        about.language.clone().unwrap_or_default(),
        about.date.clone(),
        tags,
    )
    .await?;

    // TODO: 어딘가에 내역을 기록해야함

    Ok(())
}

async fn sync_image(
    token: &container::Token,
    id: u32,
    page: usize,
    image: crawler::image::Image,
    buf: Bytes,
) -> crate::Result<()> {
    let path = format!("image/library/{id}/{page}.{}", image.ext());

    file::upload("https://beta.api.madome.app", token, path, buf).await?;

    // TODO: 어딘가에 내역을 기록해야함

    Ok(())
}

async fn sync_thumbnail(
    token: &container::Token,
    id: u32,
    image: crawler::image::Image,
    buf: Bytes,
) -> crate::Result<()> {
    let path = format!("image/library/{id}/thumbnail.{}", image.ext());

    file::upload("https://beta.api.madome.app", token, path, buf).await?;

    // TODO: 어딘가에 내역을 기록해야함

    Ok(())
}

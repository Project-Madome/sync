use std::fmt::Debug;

use bytes::Bytes;
use madome_sdk::api::{file, library};
use sai::{Component, ComponentLifecycle, Injected};
use tokio::sync::{mpsc, oneshot};

use crate::{
    container::{self, ProgressKind},
    SendError,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    LibrarySdk(#[from] library::Error),

    #[error("{0}")]
    FileSdk(#[from] file::Error),
}

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

        let channel = self.channel.clone();
        let token = self.token.clone();

        tokio::spawn(async move {
            let token = &token;

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
                    // 이후에 이미지 업로드가 다 됐따 하면 release하는 방식
                    //
                    // release 하는 도중에도 에러가 날 수 있으니까 이것도 저장해놨따가 아무것도 안할때 틈틈이 시도
                    SyncKind::About(about) => {
                        log::info!("sync_about;id={}", about.id);

                        let r = sync_about(token, &about)
                            .to(about.id, channel.err_tx())
                            .await
                            .is_some();

                        if r {
                            // log::debug!("sync_about;send_about");
                            channel.about_tx().send(about).await.unwrap();
                        }
                    }

                    SyncKind::Image(id, page, total_page, image, buf) => match image.kind() {
                        crawler::image::ImageKind::Thumbnail => {
                            log::info!("sync_thumbnail;id={id}");

                            let _r = sync_thumbnail(token, id, image, buf)
                                .too(id, 0, total_page, channel.err_tx())
                                .await
                                .is_some();
                        }

                        crawler::image::ImageKind::Original => {
                            log::info!("sync_image;id={id};page={page}/{total_page}");

                            let r = sync_image(token, id, page, image, buf)
                                .too(id, page, total_page, channel.err_tx())
                                .await
                                .is_some();

                            if r {
                                // progress 갱신
                                channel
                                    .progress_tx()
                                    .send(ProgressKind::Image(id, page, total_page))
                                    .await
                                    .unwrap()
                            }
                        }
                    },

                    SyncKind::Release(id) => {
                        log::info!("release_book;id={id}");

                        let _r = release_book(token, id)
                            .to(id, channel.err_tx())
                            .await
                            .is_some();
                    }
                }
            }

            log::debug!("shutdown_sync");

            stop_sender.send(()).unwrap()
        });
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}

pub enum SyncKind {
    About(crawler::model::Gallery),
    /// id, page, total_page, image, buf
    Image(u32, usize, usize, crawler::image::Image, Bytes),
    Release(u32),
}

impl Debug for SyncKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            Self::About(x) => format!("About({})", x.id),
            Self::Image(id, page, total, _, _) => format!("Image({id}, {page}, {total})"),
            Self::Release(id) => format!("Release({id})"),
        };

        write!(f, "SyncKind::{x}")
    }
}

#[allow(clippy::await_holding_lock)]
async fn sync_about(
    token: &container::Token,
    about: &crawler::model::Gallery,
) -> Result<(), Error> {
    let (_lock, token) = token.as_behavior();

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

    Ok(())
}

#[allow(clippy::await_holding_lock)]
async fn sync_image(
    token: &container::Token,
    id: u32,
    page: usize,
    image: crawler::image::Image,
    buf: Bytes,
) -> Result<(), Error> {
    let (_lock, token) = token.as_behavior();

    let path = format!("image/library/{id}/{page}.{}", image.ext());

    file::upload("https://beta.api.madome.app", token, path, buf).await?;

    Ok(())
}

#[allow(clippy::await_holding_lock)]
async fn sync_thumbnail(
    token: &container::Token,
    id: u32,
    image: crawler::image::Image,
    buf: Bytes,
) -> Result<(), Error> {
    let (_lock, token) = token.as_behavior();

    let path = format!("image/library/{id}/thumbnail.{}", image.ext());

    file::upload("https://beta.api.madome.app", token, path, buf).await?;

    Ok(())
}

#[allow(clippy::await_holding_lock)]
async fn release_book(token: &container::Token, id: u32) -> Result<(), Error> {
    let (_lock, token) = token.as_behavior();

    library::release_book("https://beta.api.madome.app", token, id).await?;

    Ok(())
}

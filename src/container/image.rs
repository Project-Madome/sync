use sai::{Component, ComponentLifecycle, Injected};
use tokio::sync::{mpsc, oneshot};

use crate::{container, SendError};

#[derive(Component)]
#[lifecycle]
pub struct Image {
    #[injected]
    channel: Injected<container::Channel>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Image {
    async fn start(&mut self) {
        let (stop_sender, rx) = oneshot::channel();
        let (tx, mut stop_receiver) = mpsc::channel(1);

        self.tx.replace(tx);
        self.rx.replace(rx);

        let Self { channel, .. } = self;

        // stop_receiver를 비동기로 받아야 하는 이유
        // 동기식으로 받게 되면 한 번 훑고 기다리지 않기 때문에 멈추는 데 오랜 시간이 걸릴 수도 있다
        // 그리고 멈추지 않을 수도 있는데, 아래 코드와 같이 about_recv에서 값을 받지 못 하면 영영 멈추지 못 함
        // tokio::select! { _ = stop_recv => break, r = about_recv => r }
        loop {
            let about = tokio::select! {
                _ = stop_receiver.recv() => {
                    break;
                }
                about = channel.about_recv() => {
                    about
                }
            };

            let total_page = about.files.len();

            'b: for (page, file) in about.files.iter().enumerate().map(|(i, f)| (i + 1, f)) {
                log::info!("download_image;id={};page={page}/{total_page}", about.id);

                let image = {
                    let x = crawler::image::Image::new(
                        about.id,
                        file,
                        crawler::image::ImageKind::Original,
                    )
                    .too(about.id, page, total_page, channel.err_tx())
                    .await;

                    match x {
                        Some(x) => x,
                        None => break 'b,
                    }
                };

                match image
                    .download()
                    .too(about.id, page, total_page, channel.err_tx())
                    .await
                {
                    Some(buf) => {
                        let _r = channel
                            .sync_tx()
                            .send(container::SyncKind::Image(
                                about.id, page, total_page, image, buf,
                            ))
                            .await
                            .unwrap();
                        // TODO: channel이 닫혔을 때 unwrap이 아니라 다른 방법으로 종료시키거나 아니면 채널을 새로 생성하거나 해야 한다
                    }
                    None => {
                        break 'b;
                    }
                }
            }
        }

        log::debug!("shutdown_image");

        stop_sender.send(()).unwrap();
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}

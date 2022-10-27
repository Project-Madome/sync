use std::{io, path::Path};

use chrono::{DateTime, Utc};
use sai::{Component, ComponentLifecycle, Injected};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot},
};

use crate::{container, Error};

#[derive(Component)]
#[lifecycle]
pub struct ErrorManager {
    #[injected]
    channel: Injected<container::Channel>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for ErrorManager {
    async fn start(&mut self) {
        let (stop_sender, rx) = oneshot::channel();
        let (tx, mut stop_receiver) = mpsc::channel(1);

        self.tx.replace(tx);
        self.rx.replace(rx);

        let channel = self.channel.clone();

        tokio::spawn(async move {
            loop {
                let err_info = tokio::select! {
                    _ = stop_receiver.recv() => {
                        break;
                    }
                    err_info = channel.err_recv() => {
                        err_info
                    }
                };

                {
                    let (_, _, _, err) = &err_info;
                    log::error!("{err}");
                }

                // TODO: 사용자에게 에러가 뭔지를 보여줄 거기 때문에
                //
                // 기본적으로는 실시간으로 웹소켓으로 쏴주고
                // 클라이언트에서 어떤 작품이 release: false일 때 이것이 에러때문인지 아니면 진행 중인지 알 수가 없으니까
                // 클라이언트에서 요청해서 특정 작품에 대한 에러메세지를 요구하면 있으면 json으로 주고 아니면 null로 줌
                //
                // 이거랑 관련해서 progress도 크롤러 웹소켓을 통해서 주는 게 나을 듯?
                let (id, page, total_page, err) = err_info;

                let mut json = ErrorJson {
                    id,
                    page,
                    total_page,
                    kind: ErrorKind::About,
                    err: err.to_string(),
                    created_at: Utc::now(),
                };

                match err {
                    Error::About(_err) => {
                        json.kind = ErrorKind::About;
                    }

                    Error::Sync(_err) => {
                        json.kind = ErrorKind::Sync;
                    }

                    Error::Token(_err) => {
                        json.kind = ErrorKind::Token;
                    }

                    Error::Nozomi(_err) => {
                        json.kind = ErrorKind::Nozomi;
                    }

                    Error::Image(_err) => {
                        json.kind = ErrorKind::Image;
                    }
                }

                if let Err(err) = json.write_error_file().await {
                    log::error!("ErrorManager: {err}");
                }
            }

            stop_sender.send(()).unwrap();
        });
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}

#[derive(Serialize, Deserialize)]
enum ErrorKind {
    About,
    Sync,
    Token,
    Nozomi,
    Image,
}

#[derive(Serialize, Deserialize)]
struct ErrorJson {
    id: Option<u32>,
    page: Option<usize>,
    total_page: Option<usize>,
    kind: ErrorKind,
    err: String,
    created_at: DateTime<Utc>,
}

impl ErrorJson {
    async fn write_error_file(self) -> io::Result<()> {
        fs::create_dir_all("error/").await?;

        let p = match self.id {
            Some(id) => format!("error/{id}.json"),
            None => "error/_.json".to_string(),
        };
        let p: &Path = p.as_ref();

        let mut xs = if p.exists() {
            let mut file = File::open(p).await?;
            let mut buf = Vec::new();

            file.read_to_end(&mut buf).await?;

            serde_json::from_slice::<Vec<Self>>(&buf).unwrap_or_default()
        } else {
            Vec::new()
        };

        xs.push(self);

        let mut file = File::create(p).await?;

        let r = serde_json::to_string(&xs).unwrap();

        file.write_all(r.as_bytes()).await?;

        Ok(())
    }
}

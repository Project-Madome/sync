use std::{io, ops::Deref, time::Duration};

use chrono::{DateTime, Utc};
use http_util::{Cookie, SetCookie};
use madome_sdk::api::{
    auth,
    cookie::{MADOME_ACCESS_TOKEN, MADOME_REFRESH_TOKEN},
    TokenBehavior,
};
use parking_lot::RwLock;
use sai::{Component, ComponentLifecycle, Injected};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot},
};

use crate::container;

#[derive(Component)]
#[lifecycle]
pub struct Token {
    #[injected]
    channel: Injected<container::Channel>,

    inner: Option<TokenRwLock>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Token {
    async fn start(&mut self) {
        self.initialize().await.expect("initialize token");

        let (stop_sender, a_rx) = oneshot::channel();
        let (b_tx, mut stop_receiver) = mpsc::channel(1);

        self.rx.replace(a_rx);
        self.tx.replace(b_tx);

        loop {
            let now = Utc::now().timestamp();
            let created_at = { *self.inner.as_ref().unwrap().created_at.read() };

            if now - created_at.timestamp() > 10800 {
                let token = self.as_behavior();

                match auth::refresh_token_pair("https://beta.api.madome.app", token).await {
                    Ok(_) => {
                        // self.behavior.replace(token);
                        // self.pair = token.pair.read().clone();
                        // self.created_at = Some(*token.created_at.read());

                        if let Err(err) = self.sync().await {
                            // TODO: send error to channel.err_tx()
                            log::error!("sync_token_pair: {err}");
                        }
                    }
                    Err(err) => {
                        // TODO: send error to channel.err_tx()
                        log::error!("refresh_token_pair: {err}");
                        // TODO: send stop signal?
                        // panic!("refresh_token_pair: {err}");
                    }
                }
            }

            tokio::select! {
                // 2. 멈추라는 신호를 받음
                _ = stop_receiver.recv() => {
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    continue;
                }
            }
        }

        log::debug!("shutdown_token");

        // 3. 멈추었음을 알려줌
        stop_sender.send(()).unwrap();
    }

    async fn stop(&mut self) {
        // 1. 멈추라는 신호를 보냄
        self.tx.take().unwrap().send(()).await.unwrap();

        // 4. 멈추었음을 확인함
        self.rx.take().unwrap().await.unwrap();
    }
}

impl Token {
    async fn initialize(&mut self) -> io::Result<()> {
        if let Ok(mut f) = File::open("./.token.json").await {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).await?;

            let t = serde_json::from_slice::<TokenJson>(&buf).unwrap();

            self.inner.replace(TokenRwLock {
                pair: RwLock::new((t.access, t.refresh)),
                created_at: RwLock::new(t.created_at),
            });
        } else {
            // Token::new()
            panic!("please write the `.token.json`")
        }

        Ok(())
    }

    async fn sync(&self) -> io::Result<()> {
        let serialized = serde_json::to_string_pretty(&self.to_json()).unwrap();

        let mut f = File::open("./.token.json").await?;
        f.write_all(serialized.as_bytes()).await?;

        Ok(())
    }

    fn to_json(&self) -> TokenJson {
        let x = self.inner.as_ref().unwrap();

        let (access, refresh) = x.pair.read().clone();
        let created_at = *x.created_at.read();

        TokenJson {
            access,
            refresh,
            created_at,
        }
    }

    /* fn to_behavior(&self) -> TokenRwLock {
        TokenRwLock {
            pair: RwLock::new(self.pair.clone()),
            created_at: RwLock::new(self.created_at.unwrap()),
        }
    } */

    pub fn as_behavior(&self) -> &dyn TokenBehavior {
        &**self
    }
}

impl Deref for Token {
    type Target = TokenRwLock;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct TokenJson {
    pub(crate) access: String,
    pub(crate) refresh: String,
    pub(crate) created_at: DateTime<Utc>,
}

pub struct TokenRwLock {
    pair: RwLock<(String, String)>,
    created_at: RwLock<DateTime<Utc>>,
}

impl TokenBehavior for TokenRwLock {
    fn update(&self, headers: &http::HeaderMap) {
        let mut set_cookie = SetCookie::from_headers(headers);

        let x = set_cookie.take(MADOME_ACCESS_TOKEN);
        let y = set_cookie.take(MADOME_REFRESH_TOKEN);

        if let (Some(x), Some(y)) = (x, y) {
            *self.pair.write() = (x, y);
            *self.created_at.write() = Utc::now();
        }
    }

    fn as_cookie(&self) -> Cookie {
        let x = { self.pair.read() };
        Cookie::from_iter([
            (MADOME_ACCESS_TOKEN, x.0.as_str()),
            (MADOME_REFRESH_TOKEN, x.1.as_str()),
        ])
    }
}

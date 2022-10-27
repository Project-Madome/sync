use std::{io, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use http_util::{Cookie, SetCookie};
use madome_sdk::api::{
    auth,
    cookie::{MADOME_ACCESS_TOKEN, MADOME_REFRESH_TOKEN},
    TokenBehavior,
};
use parking_lot::{RwLock, RwLockReadGuard};
use sai::{Component, ComponentLifecycle, Injected};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot},
};

use crate::{container, SendError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Io: {0}")]
    Io(#[from] io::Error),

    #[error("Auth Sdk: {0}")]
    AuthSdk(#[from] auth::Error),
}

#[derive(Component)]
#[lifecycle]
pub struct Token {
    #[injected]
    channel: Injected<container::Channel>,

    inner: Option<Arc<TokenRwLock>>,
    // 사용 중일 때는 read(), 갱신 중일 때는 write()를 사용함
    lock: Option<Arc<RwLock<()>>>,

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

        let inner = self.inner.clone().unwrap();
        let lock = self.lock.clone().unwrap();
        let channel = self.channel.clone();

        tokio::spawn(async move {
            loop {
                let now = Utc::now().timestamp();
                let created_at = { *inner.created_at.read() };

                // if now - created_at.timestamp() > 7days {} panic!(토큰 발급 필요함)

                if now - created_at.timestamp() > 10800 {
                    // 갱신 중에는 사용하지 못 하도록 함
                    let _lock = lock.write();
                    let token = &*inner;

                    match refresh_token_pair(token).await {
                        Ok(_) => {
                            /* *inner.pair.write() = token.pair.read().clone();
                             *inner.created_at.write() = *token.created_at.read(); */

                            let _r = inner.sync().to(None, channel.err_tx()).await.is_some();
                        }
                        Err(err) => {
                            // TODO: send stop signal?

                            let _r = channel
                                .err_tx()
                                .send((None, None, None, err.into()))
                                .await
                                .unwrap();
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
        });
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
        self.lock.replace(Arc::new(RwLock::new(())));

        if let Ok(mut f) = File::open("./.token.json").await {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).await?;

            let t = serde_json::from_slice::<TokenJson>(&buf).unwrap();

            self.inner.replace(Arc::new(TokenRwLock {
                pair: RwLock::new((t.access, t.refresh)),
                created_at: RwLock::new(t.created_at),
            }));
        } else {
            // Token::new()
            panic!("please write the `.token.json`")
        }

        Ok(())
    }

    /// lock_guard로 사용 중에 갱신 되는 것을 막음
    ///
    /// 대신 사용하는 곳에서 소유권 잘 생각해서 써야함
    pub fn as_behavior(&self) -> (RwLockReadGuard<()>, &dyn TokenBehavior) {
        let lock = self.lock.as_ref().unwrap().read();

        (lock, self.inner.as_deref().unwrap())
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

impl TokenRwLock {
    fn to_json(&self) -> TokenJson {
        let (access, refresh) = self.pair.read().clone();
        let created_at = *self.created_at.read();

        TokenJson {
            access,
            refresh,
            created_at,
        }
    }

    async fn sync(&self) -> Result<(), Error> {
        let serialized = serde_json::to_string_pretty(&self.to_json()).unwrap();

        let mut f = File::open("./.token.json").await?;
        f.write_all(serialized.as_bytes()).await?;

        Ok(())
    }
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

async fn refresh_token_pair(token: &dyn TokenBehavior) -> Result<(), Error> {
    auth::refresh_token_pair("https://beta.api.madome.app", token).await?;

    Ok(())
}

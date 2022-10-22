use std::{io, time::Duration};

use chrono::{DateTime, Utc};
use http_util::{Cookie, SetCookie};
use madome_sdk::api::{
    auth,
    cookie::{MADOME_ACCESS_TOKEN, MADOME_REFRESH_TOKEN},
    TokenBehavior,
};
use parking_lot::RwLock;
use sai::{Component, ComponentLifecycle};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};

#[derive(Debug, Component)]
#[lifecycle]
pub struct Token {
    pub(crate) pair: RwLock<(String, String)>,
    pub(crate) created_at: RwLock<Option<DateTime<Utc>>>,

    tx: Option<oneshot::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Token {
    async fn start(&mut self) {
        *self = Self::initialize().await.expect("initialize token");

        let (stop_sender, a_rx) = oneshot::channel();
        let (b_tx, mut stop_receiver) = oneshot::channel();

        self.rx.replace(a_rx);
        self.tx.replace(b_tx);

        loop {
            let now = Utc::now().timestamp();
            let created_at = self.created_at.read().unwrap();

            if now - created_at.timestamp() > 10800 {
                let token = Token {
                    pair: RwLock::new(self.pair.read().clone()),
                    created_at: RwLock::new(Some(created_at)),
                    tx: None,
                    rx: None,
                };

                match auth::refresh_token_pair("https://beta.api.madome.app", &token).await {
                    Ok(_) => {
                        *self.pair.write() = token.pair.read().clone();
                        *self.created_at.write() = *token.created_at.read();

                        if let Err(err) = self.sync().await {
                            log::error!("sync_token_pair: {err}");
                        }
                    }
                    Err(err) => {
                        log::error!("refresh_token_pair: {err}");
                        // TODO: send stop signal
                        // panic!("refresh_token_pair: {err}");
                    }
                }
            }

            // 2. 멈추라는 신호를 받음
            if stop_receiver.try_recv().is_ok() {
                break;
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }

        // 3. 멈추었음을 알려줌
        stop_sender.send(()).unwrap();
    }

    async fn stop(&mut self) {
        // 1. 멈추라는 신호를 보냄
        self.tx.take().unwrap().send(()).unwrap();

        // 4. 멈추었음을 확인함
        self.rx.take().unwrap().await.unwrap();
    }
}

#[derive(Serialize, Deserialize)]
struct TokenJson {
    pub access: String,
    pub refresh: String,
    pub created_at: DateTime<Utc>,
}

impl Token {
    /* fn new() -> Self {
        Self {
            pair: Default::default(),
            created_at: Utc::now(),
        }
    } */

    pub async fn initialize() -> io::Result<Self> {
        let token = if let Ok(mut f) = File::open("./.token.json").await {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).await?;

            let t = serde_json::from_slice::<TokenJson>(&buf).unwrap();

            Token {
                pair: RwLock::new((t.access, t.refresh)),
                created_at: RwLock::new(Some(t.created_at)),
                tx: None,
                rx: None,
            }
        } else {
            // Token::new()
            panic!("please write the `.token.json`")
        };

        Ok(token)
    }

    pub async fn sync(&self) -> io::Result<()> {
        let serialized = serde_json::to_string_pretty(&self.to_json()).unwrap();

        let mut f = File::open("./.token.json").await?;
        f.write_all(serialized.as_bytes()).await?;

        Ok(())
    }

    fn to_json(&self) -> TokenJson {
        let (access, refresh) = self.pair.read().clone();
        let created_at = self.created_at.read().expect("read token.created_at");

        TokenJson {
            access,
            refresh,
            created_at,
        }
    }
}

impl TokenBehavior for Token {
    fn update(&self, headers: &http::HeaderMap) {
        let mut set_cookie = SetCookie::from_headers(headers);

        let x = set_cookie.take(MADOME_ACCESS_TOKEN);
        let y = set_cookie.take(MADOME_REFRESH_TOKEN);

        if let (Some(x), Some(y)) = (x, y) {
            *self.pair.write() = (x, y);
            *self.created_at.write() = Some(Utc::now());
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

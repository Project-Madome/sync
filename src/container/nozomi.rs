use std::time::Duration;

use madome_sdk::api::library;
use sai::{Component, ComponentLifecycle, Injected};
use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};

use crate::{config::Config, container, SendError};

/// # Nozomi
///
/// 해당 컨테이너는 작품의 id를 히토미로부터 가져와서, 마도메에 존재하지 않는 id만 걸러서 다른 컨테이너에게 보냅니다.
///
/// ## 실패 시 동작
///
/// 아직 정해진 건 없음
#[derive(Component)]
#[lifecycle]
pub struct Nozomi {
    #[injected]
    config: Injected<Config>,

    #[injected]
    channel: Injected<container::Channel>,

    #[injected]
    token: Injected<container::Token>,

    tx: Option<mpsc::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Nozomi {
    async fn start(&mut self) {
        let (stop_sender, rx) = oneshot::channel();
        let (tx, mut stop_receiver) = mpsc::channel(1);

        self.tx.replace(tx);
        self.rx.replace(rx);

        let Self {
            config,
            channel,
            token,
            ..
        } = self;

        let mut store = Vec::<u32>::new();
        let mut state = State::new(config.per_page());

        let mut empty_count = 0;

        loop {
            let mut ids = tokio::select! {
                _ = stop_receiver.recv() => {
                    break;
                }
                ids = get_ids_from_not_contains(token.as_ref(), &mut state).to(None, channel.err_tx()) => {
                    ids.unwrap_or_default()
                }
            };

            if ids.is_empty() {
                empty_count += 1;
            } else {
                store.append(&mut ids);
            }

            if empty_count >= 3 {
                // asc
                store.sort();

                for id in store.drain(..) {
                    channel.id_tx().send(id).await.expect("closed id channel");
                }

                store = Vec::new();
                empty_count = 0;
                state.clear();

                tokio::select! {
                    _ = stop_receiver.recv() => {
                        break;
                    }
                    _ = sleep(Duration::from_secs(180)) => {
                        continue;
                    }
                };
            }
        }

        stop_sender.send(()).unwrap();
    }

    async fn stop(&mut self) {
        self.tx.take().unwrap().send(()).await.unwrap();

        self.rx.take().unwrap().await.unwrap();
    }
}
#[derive(Debug)]
struct State {
    page: usize,
    per_page: usize,
}

impl State {
    pub fn new(per_page: usize) -> Self {
        Self { page: 0, per_page }
    }

    pub fn next_page(&mut self) -> usize {
        self.page += 1;
        self.page
    }

    pub fn per_page(&self) -> usize {
        self.per_page
    }

    pub fn clear(&mut self) {
        self.page = 0;
    }
}

async fn get_ids_from_not_contains(
    token: &container::Token,
    state: &mut State,
) -> crate::Result<Vec<u32>> {
    let ids = crawler::nozomi::parse(state.next_page(), state.per_page()).await?;

    let xs = library::get_books_by_ids("https://beta.api.madome.app", token, ids.clone()).await?;
    let xs = xs.iter().map(|x| x.id).collect::<Vec<_>>();

    let ids = ids.into_iter().filter(|id| !xs.contains(id)).collect();

    Ok(ids)
}

use std::future::Future;

use madome_sdk::api::{file, library};
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Crawler: {0}")]
    Crawler(#[from] crawler::Error),

    #[error("Library Sdk: {0}")]
    LibrarySdk(#[from] library::Error),

    #[error("File Sdk: {0}")]
    FileSdk(#[from] file::Error),
}

#[async_trait::async_trait]
pub trait SendError<T> {
    async fn to(self, tx: mpsc::Sender<Error>) -> Option<T>;
}

#[async_trait::async_trait]
impl<T, E, F> SendError<T> for F
where
    T: Send,
    E: Into<Error> + Send,
    F: Future<Output = Result<T, E>> + Send,
{
    async fn to(self, tx: mpsc::Sender<Error>) -> Option<T> {
        match self.await {
            Ok(r) => Some(r),
            Err(err) => {
                tx.send(err.into()).await.unwrap();
                None
            }
        }
    }
}

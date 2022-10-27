use std::future::Future;

use tokio::sync::mpsc;

use crate::container;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Token: {0}")]
    Token(#[from] container::token::Error),

    #[error("Sync: {0}")]
    Sync(#[from] container::sync::Error),

    #[error("About: {0}")]
    About(#[from] container::about::Error),

    #[error("Image: {0}")]
    Image(#[from] container::image::Error),

    #[error("Nozomi: {0}")]
    Nozomi(#[from] container::nozomi::Error),
    /* #[error("Auth Sdk: {0}")]
    AuthSdk(#[from] auth::Error),

    #[error("Crawler: {0}")]
    Crawler(#[from] crawler::Error),

    #[error("Library Sdk: {0}")]
    LibrarySdk(#[from] library::Error),

    #[error("File Sdk: {0}")]
    FileSdk(#[from] file::Error),
    */
}

#[async_trait::async_trait]
pub trait SendError<T> {
    async fn to(
        self,
        id: impl Into<Option<u32>> + Send,
        tx: mpsc::Sender<(Option<u32>, Option<usize>, Option<usize>, Error)>,
    ) -> Option<T>;

    async fn too(
        self,
        id: impl Into<Option<u32>> + Send,
        page: impl Into<Option<usize>> + Send,
        total_page: impl Into<Option<usize>> + Send,
        tx: mpsc::Sender<(Option<u32>, Option<usize>, Option<usize>, Error)>,
    ) -> Option<T>;
}

#[async_trait::async_trait]
impl<T, E, F> SendError<T> for F
where
    T: Send,
    E: Into<Error> + Send,
    F: Future<Output = Result<T, E>> + Send,
{
    async fn to(
        self,
        id: impl Into<Option<u32>> + Send,
        tx: mpsc::Sender<(Option<u32>, Option<usize>, Option<usize>, Error)>,
    ) -> Option<T> {
        self.too(id, None, None, tx).await
    }

    async fn too(
        self,
        id: impl Into<Option<u32>> + Send,
        page: impl Into<Option<usize>> + Send,
        total_page: impl Into<Option<usize>> + Send,
        tx: mpsc::Sender<(Option<u32>, Option<usize>, Option<usize>, Error)>,
    ) -> Option<T> {
        match self.await {
            Ok(r) => Some(r),
            Err(err) => {
                tx.send((id.into(), page.into(), total_page.into(), err.into()))
                    .await
                    .unwrap();
                None
            }
        }
    }
}

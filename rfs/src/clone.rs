use crate::{
    cache::Cache,
    fungi::{meta::Block, Reader, Result},
    store::{BlockStore, Store},
};
use std::sync::Arc;
use tokio::{fs::File, io::AsyncReadExt};

const WORKERS: usize = 10;

pub async fn clone<S: Store>(reader: Reader, store: S, cache: Cache<S>) -> Result<()> {
    let downloader = BlobDownloader::new(cache);
    let mut download_pool = workers::WorkerPool::new(downloader, WORKERS);

    let uploader = BlobUploader::new(store.into());
    let mut upload_pool = workers::WorkerPool::new(uploader, WORKERS);

    let blocks = reader.all_blocks().await?;
    for block in blocks {
        let worker = download_pool.get().await;
        // we wait on output here to make sure there is something to upload
        // and let the uploader run in the background.
        let file = worker.run(block).await??;

        let worker = upload_pool.get().await;
        worker.send(file)?;
    }

    download_pool.close().await;
    upload_pool.close().await;

    Ok(())
}

struct BlobDownloader<S>
where
    S: Store,
{
    cache: Arc<Cache<S>>,
}

impl<S> Clone for BlobDownloader<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}

impl<S> BlobDownloader<S>
where
    S: Store,
{
    fn new(cache: Cache<S>) -> Self {
        Self {
            cache: Arc::new(cache),
        }
    }
}

#[async_trait::async_trait]
impl<S> workers::Work for BlobDownloader<S>
where
    S: Store,
{
    type Input = Block;
    type Output = Result<File>;

    async fn run(&mut self, block: Self::Input) -> Self::Output {
        let (_, file) = self.cache.get(&block).await?;
        Ok(file)
    }
}
struct BlobUploader<S>
where
    S: Store,
{
    store: Arc<BlockStore<S>>,
}

impl<S> Clone for BlobUploader<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

impl<S> BlobUploader<S>
where
    S: Store,
{
    fn new(store: BlockStore<S>) -> Self {
        Self {
            store: Arc::new(store),
        }
    }
}

#[async_trait::async_trait]
impl<S> workers::Work for BlobUploader<S>
where
    S: Store,
{
    type Input = File;
    type Output = ();

    async fn run(&mut self, mut file: Self::Input) -> Self::Output {
        let mut data = Vec::new();
        if let Err(err) = file.read_to_end(&mut data).await {
            log::error!("failed to read blob: {:#}", err);
        }
        if let Err(err) = self.store.set(&data).await {
            log::error!("failed to store blob: {:#}", err);
        }
    }
}

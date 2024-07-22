use crate::{
    cache::Cache,
    fungi::{meta::Block, Reader, Result},
    store::{BlockStore, Store},
};
use anyhow::Context;
use std::sync::Arc;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};

const WORKERS: usize = 10;
const BUFFER: usize = 10;

pub async fn clone<S: Store>(reader: Reader, store: S, cache: Cache<S>) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(BUFFER);

    let downloader = BlobDownloader::new(cache, tx);
    let mut download_pool = workers::WorkerPool::new(downloader, WORKERS);

    let uploader = BlobUploader::new(store.into());
    let mut upload_pool = workers::WorkerPool::new(uploader, WORKERS);

    let upload_handle = tokio::spawn(async move {
        loop {
            let file = match rx.recv().await {
                Some(f) => f,
                None => break,
            };

            let worker = upload_pool.get().await;
            if let Err(err) = worker.send(file) {
                log::error!("failed to schedule file upload: {:#}", err);
            }
        }
        upload_pool.close().await
    });

    let blocks = reader.all_blocks().await?;
    for block in blocks {
        let worker = download_pool.get().await;
        worker.send(block)?;
    }

    download_pool.close().await;
    upload_handle
        .await
        .context("waiting on upload workers to finish")?;

    Ok(())
}

struct BlobDownloader<S>
where
    S: Store,
{
    cache: Arc<Cache<S>>,
    tx: mpsc::Sender<File>,
}

impl<S> Clone for BlobDownloader<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            tx: self.tx.clone(),
        }
    }
}

impl<S> BlobDownloader<S>
where
    S: Store,
{
    fn new(cache: Cache<S>, tx: mpsc::Sender<File>) -> Self {
        Self {
            cache: Arc::new(cache),
            tx,
        }
    }
}

#[async_trait::async_trait]
impl<S> workers::Work for BlobDownloader<S>
where
    S: Store,
{
    type Input = Block;
    type Output = ();

    async fn run(&mut self, block: Self::Input) -> Self::Output {
        let file = match self.cache.get(&block).await {
            Ok((_, f)) => f,
            Err(err) => {
                log::error!("failed to download block: {:#}", err);
                return;
            }
        };
        if let Err(err) = self.tx.send(file).await {
            log::error!("failed to send file for upload: {:#}", err);
        }
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

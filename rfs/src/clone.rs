use crate::{
    cache::Cache,
    fungi::{meta::Block, Reader, Result},
    store::{BlockStore, Store},
};
use anyhow::Error;
use futures::lock::Mutex;
use hex::ToHex;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

const WORKERS: usize = 10;

pub async fn clone<S: Store>(reader: Reader, store: S, cache: Cache<S>) -> Result<()> {
    let failures = Arc::new(Mutex::new(Vec::new()));
    let cloner = BlobCloner::new(cache, store.into(), failures.clone());
    let mut workers = workers::WorkerPool::new(cloner, WORKERS);

    let mut offset = 0;
    loop {
        if !failures.lock().await.is_empty() {
            break;
        }
        let blocks = reader.all_blocks(1000, offset).await?;
        if blocks.is_empty() {
            break;
        }
        for block in blocks {
            offset += 1;
            let worker = workers.get().await;
            worker.send(block)?;
        }
    }

    workers.close().await;
    let failures = failures.lock().await;

    if failures.is_empty() {
        return Ok(());
    }

    log::error!("failed to clone one or more blocks");
    for (block, error) in failures.iter() {
        log::error!("  - failed to clone block {}: {}", block, error);
    }

    Err(crate::fungi::Error::Anyhow(anyhow::anyhow!(
        "failed to clone ({}) blocks",
        failures.len()
    )))
}

struct BlobCloner<S>
where
    S: Store,
{
    cache: Arc<Cache<S>>,
    store: Arc<BlockStore<S>>,
    failures: Arc<Mutex<Vec<(String, Error)>>>,
}

impl<S> Clone for BlobCloner<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            store: self.store.clone(),
            failures: self.failures.clone(),
        }
    }
}

impl<S> BlobCloner<S>
where
    S: Store,
{
    fn new(
        cache: Cache<S>,
        store: BlockStore<S>,
        failures: Arc<Mutex<Vec<(String, Error)>>>,
    ) -> Self {
        Self {
            cache: Arc::new(cache),
            store: Arc::new(store),
            failures,
        }
    }
}

#[async_trait::async_trait]
impl<S> workers::Work for BlobCloner<S>
where
    S: Store,
{
    type Input = Block;
    type Output = ();

    async fn run(&mut self, block: Self::Input) -> Self::Output {
        let mut file = match self.cache.get(&block).await {
            Ok((_, f)) => f,
            Err(err) => {
                self.failures
                    .lock()
                    .await
                    .push((block.id.as_slice().encode_hex(), err));
                return;
            }
        };

        let mut data = Vec::new();
        if let Err(err) = file.read_to_end(&mut data).await {
            self.failures
                .lock()
                .await
                .push((block.id.as_slice().encode_hex(), err.into()));
            return;
        }
        if let Err(err) = self.store.set(&data).await {
            self.failures
                .lock()
                .await
                .push((block.id.as_slice().encode_hex(), err.into()));
            return;
        }
    }
}

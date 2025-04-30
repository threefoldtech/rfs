use crate::{
    cache::Cache,
    fungi::{meta::{FileType, Inode, Walk, WalkVisitor}, Reader, Result, Writer},
    store::{get_router, BlockStore, Router, Store, Stores},
};
use std::path::Path;
use hex::ToHex;
use tokio::io::AsyncReadExt;
use std::collections::HashSet;

pub async fn merge<S: Store>(
    writer: Writer,
    store: BlockStore<S>,
    target_flists: Vec<String>,
    cache: String
) -> Result<()> {
    
    for target_flist in target_flists {
        let reader = Reader::new(&target_flist).await?;
        let router = get_router(&reader).await?;
        let cache = Cache::new(cache.clone(), router);

        let mut visitor = MergeVisitor::new(writer.clone(), reader.clone(), &store, cache);
        reader.walk(&mut visitor).await?;
    }
    Ok(())
}

struct MergeVisitor<'a, S>
where
    S: Store,
{
    writer: Writer,
    reader: Reader,
    store: &'a BlockStore<S>,
    cache: Cache<Router<Stores>>,
    visited: HashSet<u64>
}

impl<'a, S> MergeVisitor<'a, S>
where
    S: Store,
{
    pub fn new(writer: Writer, reader: Reader, store: &'a BlockStore<S>, cache: Cache<Router<Stores>>) -> Self {
        Self {
            writer,
            reader,
            store,
            cache,
            visited: HashSet::new(),
        }
    }
}

#[async_trait::async_trait]
impl<'a, S> WalkVisitor for MergeVisitor<'a, S> 
where
    S: Store,
{
    async fn visit(&mut self, _path: &Path, node: &Inode) -> Result<Walk> {
        if self.visited.contains(&node.ino) {
            return Ok(Walk::Break);
        }
        self.visited.insert(node.ino);

        match node.mode.file_type() {
            FileType::Dir => {
                self.writer.inode(node.clone()).await?;
                return Ok(Walk::Continue);
            }
            FileType::Regular => {
                let ino = self.writer.inode(node.clone()).await?;
                let blocks = self.reader.blocks(node.ino).await?;

                for block in &blocks {
                    self.writer.block(ino, &block.id, &block.key).await?;
                }

                let mut blocks_to_store = Vec::new();
                for block in blocks {
                    if self.store.get(&block).await.is_err() {
                        blocks_to_store.push(block);
                    }
                }
                for block in blocks_to_store {

                    let (_, mut file) = self.cache.get(&block).await?;
                    let mut data = Vec::new();
                    if let Err(e) = file.read_to_end(&mut data).await {
                        log::error!("failed to read block {}: {}", block.id.as_slice().encode_hex::<String>(), e);
                        return Err(e.into());
                    }
                    if let Err(e) = self.store.set(&data).await {
                        log::error!("failed to set block {}: {}", block.id.as_slice().encode_hex::<String>(), e);
                        return Err(e.into());
                    }
                }

            }
            _ => {
                log::warn!("Unknown file type for node: {:?}", node);
            }
        }
        
        Ok(Walk::Continue) 
    }
}
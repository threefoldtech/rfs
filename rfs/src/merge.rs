use crate::{
    cache::Cache,
    fungi::{
        self,
        meta::{FileType, Inode, Mode, Walk, WalkVisitor},
        Reader, Result, Writer,
    },
    store::{self, get_router, BlockStore, Router, Store, Stores}, upload,
};
use anyhow::Context;
use hex::ToHex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

const ROOT_PATH: &str = "/";

pub async fn merge(
    flist_path: String,
    server_url: String,
    token: &str,
    target_flists: Vec<String>,
    cache: String,
) -> Result<()> {
    let writer = fungi::Writer::new(flist_path.clone(), true).await?;

    let store = store::parse_router(&[format!(
        "{}://{}?token={}",
        store::server::SCHEME,
        server_url.clone(),
        token
    )])
    .await
    .context("Failed to create store")?;

    let store = BlockStore::from(store);

    let mut path_to_inode_map = HashMap::new();
    let root_path = PathBuf::from(ROOT_PATH);

    let root_inode = Inode {
        name: ROOT_PATH.into(),
        mode: Mode::new(FileType::Dir, 0o755),
        ..Default::default()
    };
    let root_ino = writer.inode(root_inode).await?;
    path_to_inode_map.insert(root_path, root_ino);

    for target_flist in target_flists {
        let reader = Reader::new(&target_flist).await?;
        let router = get_router(&reader).await?;
        let cache_instance = Cache::new(cache.clone(), router);

        let mut visited = HashSet::new();
        let mut visitor = MergeVisitor {
            writer: writer.clone(),
            reader: reader.clone(),
            store: &store,
            cache: cache_instance,
            path_to_inode: &mut path_to_inode_map,
            visited: &mut visited,
        };

        reader.walk(&mut visitor).await?;
    }

    let flist_hash = upload(&flist_path, server_url.clone(), None, token)
        .await
        .context("Failed to upload flist file")?;

    info!("Flist uploaded successfully. Hash: {}", flist_hash);

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
    path_to_inode: &'a mut HashMap<PathBuf, u64>,
    visited: &'a mut HashSet<u64>,
}

impl<'a, S> MergeVisitor<'a, S>
where
    S: Store,
{
    async fn ensure_parent_directory(&mut self, path: &Path) -> Result<u64> {
        if path.to_str() == Some(ROOT_PATH) {
            return Ok(*self.path_to_inode.get(path).unwrap_or(&1));
        }

        if let Some(ino) = self.path_to_inode.get(path) {
            return Ok(*ino);
        }

        let parent_path = path.parent().unwrap_or(Path::new(ROOT_PATH));
        let parent_ino = Box::pin(self.ensure_parent_directory(parent_path)).await?;

        let dir_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();

        let dir_inode = Inode {
            parent: parent_ino,
            name: dir_name,
            mode: Mode::new(FileType::Dir, 0o755),
            ..Default::default()
        };

        let new_ino = self.writer.inode(dir_inode).await?;
        self.path_to_inode.insert(path.to_path_buf(), new_ino);

        Ok(new_ino)
    }

    async fn copy_blocks(&mut self, source_ino: u64, dest_ino: u64) -> Result<()> {
        let blocks = self.reader.blocks(source_ino).await?;

        for block in &blocks {
            self.writer.block(dest_ino, &block.id, &block.key).await?;
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
                log::error!(
                    "failed to read block {}: {}",
                    block.id.as_slice().encode_hex::<String>(),
                    e
                );
                return Err(e.into());
            }
            if let Err(e) = self.store.set(&data).await {
                log::error!(
                    "failed to set block {}: {}",
                    block.id.as_slice().encode_hex::<String>(),
                    e
                );
                return Err(e.into());
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<'a, S> WalkVisitor for MergeVisitor<'a, S>
where
    S: Store,
{
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
        if !self.visited.insert(node.ino) {
            return Ok(Walk::Continue);
        }

        match node.mode.file_type() {
            FileType::Dir => {
                if path.to_str() == Some(ROOT_PATH) {
                    return Ok(Walk::Continue);
                }

                let dir_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();

                let parent_path = path.parent().unwrap_or(Path::new(ROOT_PATH));
                let parent_ino = self.ensure_parent_directory(parent_path).await?;

                let dir_node = Inode {
                    parent: parent_ino,
                    name: dir_name,
                    mode: node.mode.clone(),
                    uid: node.uid,
                    gid: node.gid,
                    rdev: node.rdev,
                    ctime: node.ctime,
                    mtime: node.mtime,
                    data: node.data.clone(),
                    ..Default::default()
                };

                let dir_ino = self.writer.inode(dir_node).await?;
                self.path_to_inode.insert(path.to_path_buf(), dir_ino);
            }
            FileType::Regular => {
                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();

                let parent_path = path.parent().unwrap_or(Path::new(ROOT_PATH));
                let parent_ino = self.ensure_parent_directory(parent_path).await?;

                let file_node = Inode {
                    parent: parent_ino,
                    name: file_name,
                    size: node.size,
                    uid: node.uid,
                    gid: node.gid,
                    mode: node.mode.clone(),
                    rdev: node.rdev,
                    ctime: node.ctime,
                    mtime: node.mtime,
                    data: node.data.clone(),
                    ..Default::default()
                };

                let ino = self.writer.inode(file_node).await?;
                self.copy_blocks(node.ino, ino).await?;
            }
            _ => {
                log::warn!("Unknown file type for node: {:?}", node);
            }
        }

        Ok(Walk::Continue)
    }
}

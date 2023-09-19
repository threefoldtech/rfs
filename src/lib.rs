#[macro_use]
extern crate log;
use anyhow::Context;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::{ffi::OsStr, fs};
use store::Store;

pub mod cache;
pub mod fungi;
pub mod store;

use cache::Cache;
use fungi::{
    meta::{FileType, Inode, Walk, WalkVisitor},
    Reader,
};

pub struct CopyVisitor<'a, S>
where
    S: store::Store,
{
    meta: &'a fungi::Reader,
    cache: &'a cache::Cache<S>,
    root: &'a Path,
}

impl<'a, S> CopyVisitor<'a, S>
where
    S: store::Store,
{
    pub fn new(meta: &'a fungi::Reader, cache: &'a Cache<S>, root: &'a Path) -> Self {
        Self { meta, cache, root }
    }
}

#[async_trait::async_trait]
impl<'a, S> WalkVisitor for CopyVisitor<'a, S>
where
    S: Store,
{
    async fn visit(&mut self, path: &Path, node: &Inode) -> fungi::meta::Result<Walk> {
        use tokio::fs::OpenOptions;

        let rooted = self.root.join(path.strip_prefix("/").unwrap());

        match node.mode.file_type() {
            FileType::Dir => {
                fs::create_dir_all(&rooted)
                    .with_context(|| format!("failed to create directory '{:?}'", rooted))?;
            }
            FileType::Regular => {
                let mut fd = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .mode(node.mode.mode())
                    .open(&rooted)
                    .await
                    .with_context(|| format!("failed to create file '{:?}'", rooted))?;

                let blocks = self.meta.blocks(node.ino).await?;
                self.cache
                    .direct(&blocks, &mut fd)
                    .await
                    .with_context(|| format!("failed to create download file '{:?}'", rooted))?;
            }
            FileType::Link => {
                let target = node
                    .data
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("link has no target path"))?;

                let target = Path::new(OsStr::from_bytes(target));
                let target = if target.is_relative() {
                    target.to_owned()
                } else {
                    self.root.join(target)
                };

                std::os::unix::fs::symlink(target, &rooted)
                    .with_context(|| format!("failed to create symlink '{:?}'", rooted))?;
            }
            _ => {
                debug!("unknown file kind: {:?}", node.mode.file_type());
            }
        };

        Ok(Walk::Continue)
    }
}

pub async fn extract<P: AsRef<Path>, S: Store>(
    meta: &Reader,
    cache: &Cache<S>,
    root: P,
) -> Result<(), fungi::meta::Error> {
    let mut visitor = CopyVisitor::new(meta, cache, root.as_ref());

    meta.walk(&mut visitor).await
}

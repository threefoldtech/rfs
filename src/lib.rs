#[macro_use]
extern crate log;
use anyhow::Context;
use fungi::meta::Ino;
use fungi::Writer;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::{ffi::OsStr, fs};
use store::Store;

pub mod cache;
pub mod fungi;
pub mod store;

use cache::Cache;
use fungi::{
    meta::{FileType, Inode, Result, Walk, WalkVisitor},
    Reader,
};

struct CopyVisitor<'a, S>
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
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
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
) -> Result<()> {
    let mut visitor = CopyVisitor::new(meta, cache, root.as_ref());

    meta.walk(&mut visitor).await
}

pub async fn pack<P: AsRef<Path>>(meta: &Writer, root: P) -> Result<()> {
    use tokio::fs;

    let m = fs::metadata(&root)
        .await
        .context("failed to get root stats")?;

    scan(meta, 0, "/", root.as_ref(), &m).await
}

#[async_recursion::async_recursion]
pub async fn scan(meta: &Writer, parent: Ino, name: &str, path: &Path, m: &Metadata) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    use tokio::fs;

    let data = if m.is_symlink() {
        let target = fs::read_link(&path).await?;
        Some(target.as_os_str().as_bytes().into())
    } else {
        None
    };

    let ino = meta
        .inode(Inode {
            name: name.into(),
            parent,
            size: m.size(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode().into(),
            rdev: m.rdev(),
            ctime: m.ctime(),
            mtime: m.mtime(),
            data: data,
            ..Default::default()
        })
        .await?;

    if !m.is_dir() {
        return Ok(());
    }

    // sub files
    let mut children = fs::read_dir(path).await?;
    while let Some(child) = children.next_entry().await? {
        scan(
            meta,
            ino,
            &child.file_name().to_string_lossy(),
            &path.join(child.file_name()),
            &child.metadata().await?,
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::fungi::meta;

    use super::*;

    // TODO: create a directory on the fly, pack it then walk it again
    // from meta.
    #[ignore]
    #[tokio::test]
    async fn create_meta() {
        let writer = meta::Writer::new("/tmp/build.fl").await.unwrap();

        pack(&writer, "/home/azmy/Documents").await.unwrap();
        drop(writer);

        let reader = meta::Reader::new("/tmp/build.fl").await.unwrap();
        reader.walk(&mut WalkTest).await.unwrap();
    }

    struct WalkTest;

    #[async_trait::async_trait]
    impl WalkVisitor for WalkTest {
        async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
            println!("{} = {:?}", node.ino, path);
            Ok(Walk::Continue)
        }
    }
}

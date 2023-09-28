use crate::cache::Cache;
use crate::fungi::meta::{Block, Mode};
use crate::fungi::{
    meta::{FileType, Inode, Result, Walk, WalkVisitor},
    Reader,
};
use crate::store::Store;
use anyhow::Context;
use nix::unistd::{fchownat, FchownatFlags, Gid, Uid};
use std::fs::Permissions;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{ffi::OsStr, fs};
use tokio::fs::OpenOptions;

/// unpack an FL to the given root location. it will download the files and reconstruct
/// the filesystem.
pub async fn unpack<P: AsRef<Path>, S: Store>(
    meta: &Reader,
    cache: &Cache<S>,
    root: P,
    preserve: bool,
) -> Result<()> {
    let mut visitor = CopyVisitor::new(meta, cache, root.as_ref(), preserve);

    meta.walk(&mut visitor).await
}

struct CopyVisitor<'a, S>
where
    S: Store,
{
    preserve: bool,
    meta: &'a Reader,
    cache: &'a Cache<S>,
    root: &'a Path,
}

impl<'a, S> CopyVisitor<'a, S>
where
    S: Store,
{
    pub fn new(meta: &'a Reader, cache: &'a Cache<S>, root: &'a Path, preserve: bool) -> Self {
        Self {
            meta,
            cache,
            root,
            preserve,
        }
    }
}

#[async_trait::async_trait]
impl<'a, S> WalkVisitor for CopyVisitor<'a, S>
where
    S: Store,
{
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
        let rooted = self.root.join(path.strip_prefix("/").unwrap());

        match node.mode.file_type() {
            FileType::Dir => {
                fs::create_dir_all(&rooted)
                    .with_context(|| format!("failed to create directory '{:?}'", rooted))?;
            }
            FileType::Regular => {
                let mut fd = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .truncate(true)
                    .open(&rooted)
                    .await
                    .with_context(|| format!("failed to create file '{:?}'", rooted))?;

                let blocks = self.meta.blocks(node.ino).await?;
                self.cache
                    .direct(&blocks, &mut fd)
                    .await
                    .with_context(|| format!("failed to download file '{:?}'", rooted))?;

                fd.set_permissions(Permissions::from_mode(node.mode.mode()))
                    .await?;
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
                warn!("unknown file kind: {:?}", node.mode.file_type());
                return Ok(Walk::Continue);
            }
        };

        if self.preserve {
            fchownat(
                None,
                &rooted,
                Some(Uid::from_raw(node.uid)),
                Some(Gid::from_raw(node.gid)),
                FchownatFlags::NoFollowSymlink,
            )
            .with_context(|| format!("failed to change ownership of '{:?}'", &rooted))?;
        }

        Ok(Walk::Continue)
    }
}

struct Downloader<S>
where
    S: Store,
{
    cache: Arc<Cache<S>>,
}

impl<S> Downloader<S>
where
    S: Store,
{
    async fn download(&self, path: &Path, blocks: &[Block], mode: u32) -> Result<()> {
        let mut fd = OpenOptions::new()
            .create_new(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await
            .with_context(|| format!("failed to create file '{:?}'", path))?;

        self.cache
            .direct(&blocks, &mut fd)
            .await
            .with_context(|| format!("failed to download file '{:?}'", path))?;

        fd.set_permissions(Permissions::from_mode(mode)).await?;

        Ok(())
    }
}

impl<S> Clone for Downloader<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}

#[async_trait::async_trait]
impl<S> workers::Work for Downloader<S>
where
    S: Store,
{
    type Input = (PathBuf, Vec<Block>, Mode);
    type Output = ();

    async fn run(&mut self, (path, blocks, mode): Self::Input) -> Self::Output {
        if let Err(err) = self.download(&path, &blocks, mode.mode()).await {
            log::error!("failed to download file {:?}: {}", path, err);
        }
    }
}

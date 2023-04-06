#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate log;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub mod schema_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema_capnp.rs"));
}

pub mod cache;
pub mod meta;

use cache::Cache;
use meta::{EntryKind, Metadata};

pub struct CopyVisitor<'a> {
    meta: &'a Metadata,
    cache: &'a Cache,
    root: &'a Path,
}

impl<'a> CopyVisitor<'a> {
    pub fn new(meta: &'a Metadata, cache: &'a Cache, root: &'a Path) -> Self {
        Self { meta, cache, root }
    }
}

#[async_trait::async_trait]
impl<'a> meta::WalkVisitor for CopyVisitor<'a> {
    async fn visit<P: AsRef<Path> + Send + Sync>(
        &mut self,
        path: P,
        entry: &meta::Entry,
    ) -> Result<meta::Walk> {
        use tokio::fs::OpenOptions;

        let rooted = self.root.join(path.as_ref().strip_prefix("/")?);
        let acl = self
            .meta
            .aci(&entry.node.acl)
            .await
            .map(|a| a.mode & 0o777)
            .unwrap_or(0o666);

        match &entry.kind {
            EntryKind::Dir(_) => {
                fs::create_dir_all(&rooted)
                    .with_context(|| format!("failed to create directory '{:?}'", rooted))?;
            }
            EntryKind::File(file) => {
                let mut fd = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .mode(acl)
                    .open(&rooted)
                    .await
                    .with_context(|| format!("failed to create file '{:?}'", rooted))?;

                self.cache
                    .direct(&file.blocks, &mut fd)
                    .await
                    .with_context(|| format!("failed to create download file '{:?}'", rooted))?;
            }
            EntryKind::Link(link) => {
                let target = Path::new(&link.target);
                let target = if target.is_relative() {
                    target.to_owned()
                } else {
                    self.root.join(target)
                };

                std::os::unix::fs::symlink(target, &rooted)
                    .with_context(|| format!("failed to create symlink '{:?}'", rooted))?;
            }
            _ => {
                debug!("unknown file kind: {:?}", entry.kind);
            }
        };

        Ok(meta::Walk::Continue)
    }
}

pub async fn extract<P: AsRef<Path>>(meta: &Metadata, cache: &Cache, root: P) -> Result<()> {
    let mut visitor = CopyVisitor::new(meta, cache, root.as_ref());

    meta.walk(&mut visitor).await
}

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

struct CopyVisitor<'a> {
    meta: &'a Metadata,
    cache: &'a mut Cache,
    root: &'a Path,
}

#[async_trait::async_trait]
impl<'a> meta::WalkVisitor for CopyVisitor<'a> {
    async fn visit<P: AsRef<Path> + Send + Sync>(
        &mut self,
        path: P,
        entry: &meta::Entry,
    ) -> Result<()> {
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
                    .with_context(|| format!("failed to create directory '{:?}'", rooted))?;

                self.cache
                    .direct(&file.blocks, &mut fd)
                    .await
                    .with_context(|| format!("failed to create download file '{:?}'", rooted))?;
            }
            _ => {}
        };

        Ok(())
        //unimplemented!();
    }
}

pub async fn extract<P: AsRef<Path> + Send + Sync>(
    meta: &Metadata,
    cache: &mut Cache,
    root: P,
) -> Result<()> {
    let mut visitor = CopyVisitor {
        meta,
        cache,
        root: root.as_ref(),
    };

    meta.walk(&mut visitor).await
}

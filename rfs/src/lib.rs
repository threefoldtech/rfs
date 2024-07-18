#[macro_use]
extern crate log;

pub mod cache;
pub mod fungi;
pub mod store;

mod pack;
pub use pack::pack;
mod unpack;
pub use unpack::unpack;
mod clone;
pub use clone::clone;

const PARALLEL_UPLOAD: usize = 10; // number of files we can upload in parallel

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        cache::Cache,
        fungi::meta,
        store::{dir::DirStore, Router},
    };
    use std::path::PathBuf;
    use tokio::{fs, io::AsyncReadExt};

    #[tokio::test]
    async fn pack_unpack() {
        const ROOT: &str = "/tmp/pack-unpack-test";
        let _ = fs::remove_dir_all(ROOT).await;

        let root: PathBuf = ROOT.into();
        let source = root.join("source");
        fs::create_dir_all(&source).await.unwrap();

        for size in [0, 100 * 1024, 1024 * 1024, 10 * 1024 * 1024] {
            let mut urandom = fs::OpenOptions::default()
                .read(true)
                .open("/dev/urandom")
                .await
                .unwrap()
                .take(size);

            let name = format!("file-{}.rnd", size);
            let p = source.join(&name);
            let mut file = fs::OpenOptions::default()
                .create(true)
                .write(true)
                .open(p)
                .await
                .unwrap();

            tokio::io::copy(&mut urandom, &mut file).await.unwrap();
        }

        println!("file generation complete");
        let writer = meta::Writer::new(root.join("meta.fl")).await.unwrap();

        // while we at it we can already create 2 stores and create a router store on top
        // of that.
        let store0 = DirStore::new(root.join("store0")).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut store = Router::new();

        store.add(0x00, 0x7f, store0);
        store.add(0x80, 0xff, store1);

        pack(writer, store, &source, false).await.unwrap();

        println!("packing complete");
        // recreate the stores for reading.
        let store0 = DirStore::new(root.join("store0")).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut store = Router::new();

        store.add(0x00, 0x7f, store0);
        store.add(0x80, 0xff, store1);

        let cache = Cache::new(root.join("cache"), store);

        let reader = meta::Reader::new(root.join("meta.fl")).await.unwrap();
        // validate reader store routing
        let routers = reader.routes().await.unwrap();
        assert_eq!(2, routers.len());
        assert_eq!(routers[0].url, "dir:///tmp/pack-unpack-test/store0");
        assert_eq!(routers[1].url, "dir:///tmp/pack-unpack-test/store1");

        assert_eq!((routers[0].start, routers[0].end), (0x00, 0x7f));
        assert_eq!((routers[1].start, routers[1].end), (0x80, 0xff));

        unpack(&reader, &cache, root.join("destination"), false)
            .await
            .unwrap();

        println!("unpacking complete");
        // compare that source directory is exactly the same as target directory
        let status = std::process::Command::new("diff")
            .arg(root.join("source"))
            .arg(root.join("destination"))
            .status()
            .unwrap();

        assert!(status.success());
    }
}

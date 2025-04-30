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
pub mod config;
mod merge;
pub use merge::merge;
mod docker;
pub use docker::DockerImageToFlist;

const PARALLEL_UPLOAD: usize = 20; // number of files we can upload in parallel
const PARALLEL_DOWNLOAD: usize = 20; // number of files we can download in parallel

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
        let writer = meta::Writer::new(root.join("meta.fl"), true).await.unwrap();

        // while we at it we can already create 2 stores and create a router store on top
        // of that.
        let store0 = DirStore::new(root.join("store0")).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut store = Router::new();

        store.add(0x00, 0x7f, store0);
        store.add(0x80, 0xff, store1);

        pack(writer, store, &source, false, None).await.unwrap();

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

    #[tokio::test]
    async fn test_merge(){
        const ROOT: &str = "/tmp/merge-test";
        let _ = fs::remove_dir_all(ROOT).await;

        println!("declaring directories");

        let root: PathBuf = ROOT.into();
        let source1 = root.join("source1");
        let source2 = root.join("source2");
        let merged_dest = root.join("merged");
        let cache_dir = root.join("cache");

        println!("creating directories");
        
        fs::create_dir_all(&source1).await.unwrap();
        fs::create_dir_all(&source2).await.unwrap();
        fs::create_dir_all(&cache_dir).await.unwrap();

        println!("creating test files");

        create_test_files(&source1, "file1.txt", 1024).await;
        create_test_files(&source1, "file2.txt", 2048).await;

        create_test_files(&source2, "file3.txt", 2048).await;
        create_test_files(&source2, "file4.txt", 512).await;

        println!("test files created");
        println!("packing source1");
        let meta1_path = root.join("meta1.fl");
        let writer1 = meta::Writer::new(&meta1_path, true).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut router1 = Router::new();
        router1.add(0x00, 0xFF, store1);
        
        pack(writer1, router1, &source1, false, None).await.unwrap();
        println!("packing complete for source1");

        println!("packing source2");
        let meta2_path = root.join("meta2.fl");
        let writer2 = meta::Writer::new(&meta2_path, true).await.unwrap();
        let store2 = DirStore::new(root.join("store2")).await.unwrap();
        let mut router2 = Router::new();
        router2.add(0x00, 0xFF, store2);
        pack(writer2, router2, &source2, false, None).await.unwrap();

        println!("packing complete for source2");
        let merged_meta_path = root.join("merged.fl");
        let merged_writer = meta::Writer::new(&merged_meta_path, true).await.unwrap();
        let merged_store = DirStore::new(root.join("merged_store")).await.unwrap();
        let block_store = store::BlockStore::from(merged_store);

        println!("merging");

        merge(
            merged_writer,
            block_store,
            vec![meta1_path.to_string_lossy().to_string(), meta2_path.to_string_lossy().to_string()],
            cache_dir.to_string_lossy().to_string(),
        ).await.unwrap();

        println!("merge complete");
        let merged_reader = meta::Reader::new(&merged_meta_path).await.unwrap();
        let merged_router = store::get_router(&merged_reader).await.unwrap();
        let merged_cache = Cache::new(root.join("merged_cache"), merged_router);
        
        unpack(&merged_reader, &merged_cache, &merged_dest, false)
            .await
            .unwrap();

        assert!(merged_dest.join("file1.txt").exists());
        assert!(merged_dest.join("file2.txt").exists());
        assert!(merged_dest.join("file3.txt").exists());
        assert!(merged_dest.join("file4.txt").exists());


        verify_file_content(merged_dest.join("file1.txt"), 1024).await;
        verify_file_content(merged_dest.join("file2.txt"), 2048).await;
        verify_file_content(merged_dest.join("file3.txt"), 2048).await;
        verify_file_content(merged_dest.join("file4.txt"), 512).await;


    }

    async fn create_test_files<P: AsRef<std::path::Path>>(dir: P, name: &str, size: usize) {
        let mut urandom = fs::OpenOptions::default()
            .read(true)
            .open("/dev/urandom")
            .await
            .unwrap()
            .take(size as u64);

        let p = dir.as_ref().join(name);
        let mut file = fs::OpenOptions::default()
            .create(true)
            .write(true)
            .open(p)
            .await
            .unwrap();

        tokio::io::copy(&mut urandom, &mut file).await.unwrap();
    }

    async fn verify_file_content<P: AsRef<std::path::Path>>(path: P, expected_size: usize) {
        let mut file = fs::OpenOptions::default()
            .read(true)
            .open(path)
            .await
            .unwrap();

        let mut buffer = vec![0; expected_size];
        let size = file.read(&mut buffer).await.unwrap();
        assert_eq!(size, expected_size);
    }
}

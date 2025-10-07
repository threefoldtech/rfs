#[macro_use]
extern crate log;

pub mod cache;
pub mod fungi;
pub mod server;
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
mod upload;
pub use upload::*;
mod download;
pub use download::*;
mod exist;
pub use exist::*;
mod sync;
pub use sync::*;
pub mod flist_inspector;
mod server_api;
pub mod tree_visitor;

const PARALLEL_UPLOAD: usize = 20; // number of files we can upload in parallel

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

    #[test]
    fn pack_unpack() {
        // Run the test in a thread with increased stack size to prevent stack overflow
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024) // 16MB stack size
            .spawn(|| {
                // Create a runtime for async operations
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .thread_stack_size(16 * 1024 * 1024)
                    .enable_time()
                    .enable_io()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    const ROOT: &str = "/tmp/pack-unpack-test";
                    let _ = fs::remove_dir_all(ROOT).await;

                    let root: PathBuf = ROOT.into();
                    let source = root.join("source");
                    fs::create_dir_all(&source).await.unwrap();

                    // Create test files using helper function
                    for size in [0, 100 * 1024, 1024 * 1024, 10 * 1024 * 1024] {
                        let name = format!("file-{}.rnd", size);
                        create_test_files(&source, &name, size).await;
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
                    
                    // Verify unpacked files match expected sizes
                    for size in [0, 100 * 1024, 1024 * 1024, 10 * 1024 * 1024] {
                        let name = format!("file-{}.rnd", size);
                        verify_file_content(root.join("destination").join(&name), size).await;
                    }
                    
                    // compare that source directory is exactly the same as target directory
                    let status = std::process::Command::new("diff")
                        .arg(root.join("source"))
                        .arg(root.join("destination"))
                        .status()
                        .unwrap();

                    assert!(status.success());
                    
                    // Clean up test artifacts
                    let _ = fs::remove_dir_all(ROOT).await;
                    println!("test cleanup complete");
                })
            })
            .unwrap();

        handle.join().unwrap();
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
        let metadata = fs::metadata(path.as_ref()).await.unwrap();
        assert_eq!(
            metadata.len() as usize,
            expected_size,
            "File {:?} has size {} but expected {}",
            path.as_ref(),
            metadata.len(),
            expected_size
        );
    }
}

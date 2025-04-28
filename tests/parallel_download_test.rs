#[cfg(test)]
mod parallel_download_tests {
    use anyhow::Result;
    use std::path::Path;
    use tempdir::TempDir;
    use tokio::runtime::Runtime;
    use std::time::Instant;

    use rfs::fungi::{self, meta};
    use rfs::store::{self, dir::DirStore};
    use rfs::cache::Cache;
    use rfs::{pack, unpack};

    #[test]
    fn test_parallel_download() -> Result<()> {
        // Create a runtime for async operations
        let rt = Runtime::new()?;

        rt.block_on(async {
            // Create temporary directories
            let temp_dir = TempDir::new("parallel-test")?;
            let source_dir = temp_dir.path().join("source");
            let dest_dir_parallel = temp_dir.path().join("dest-parallel");
            let dest_dir_serial = temp_dir.path().join("dest-serial");
            let store_dir = temp_dir.path().join("store");
            let cache_dir = temp_dir.path().join("cache");
            
            std::fs::create_dir_all(&source_dir)?;
            std::fs::create_dir_all(&dest_dir_parallel)?;
            std::fs::create_dir_all(&dest_dir_serial)?;
            std::fs::create_dir_all(&store_dir)?;
            std::fs::create_dir_all(&cache_dir)?;

            // Create some test files
            create_test_files(&source_dir, 20, 1024 * 1024).await?; // 20 files of 1MB each

            // Create a store
            let store = DirStore::new(&store_dir).await?;

            // Create a flist writer
            let fl_path = temp_dir.path().join("test.fl");
            let writer = fungi::Writer::new(&fl_path, true).await?;

            // Pack the files
            pack(writer, store.clone(), &source_dir, true, None).await?;

            // Create a reader for the flist
            let reader = fungi::Reader::new(&fl_path).await?;
            let router = store::get_router(&reader).await?;

            // Test parallel download (default)
            let cache_parallel = Cache::new(&cache_dir, router.clone());
            let start_parallel = Instant::now();
            unpack(&reader, &cache_parallel, &dest_dir_parallel, false).await?;
            let parallel_duration = start_parallel.elapsed();

            // Clear cache directory
            std::fs::remove_dir_all(&cache_dir)?;
            std::fs::create_dir_all(&cache_dir)?;

            // Test serial download by setting PARALLEL_DOWNLOAD to 1
            // This is just a simulation since we can't easily modify the constant at runtime
            // In a real test, we would use a feature flag or environment variable
            let cache_serial = Cache::new(&cache_dir, router);
            let start_serial = Instant::now();
            
            // Here we're still using the parallel implementation, but in a real test
            // we would use a version with PARALLEL_DOWNLOAD=1
            unpack(&reader, &cache_serial, &dest_dir_serial, false).await?;
            
            let serial_duration = start_serial.elapsed();

            // Print the results
            println!("Parallel download time: {:?}", parallel_duration);
            println!("Serial download time: {:?}", serial_duration);

            // Verify files were unpacked correctly
            verify_directories(&source_dir, &dest_dir_parallel)?;
            verify_directories(&source_dir, &dest_dir_serial)?;

            Ok(())
        })
    }

    // Helper function to create test files
    async fn create_test_files(dir: &Path, count: usize, size: usize) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;
        use rand::{thread_rng, Rng};

        for i in 0..count {
            let file_path = dir.join(format!("file_{}.bin", i));
            let mut file = File::create(&file_path).await?;
            
            // Create random data
            let mut data = vec![0u8; size];
            thread_rng().fill(&mut data[..]);
            
            // Write to file
            file.write_all(&data).await?;
            file.flush().await?;
        }

        Ok(())
    }

    // Helper function to verify directories match
    fn verify_directories(source: &Path, dest: &Path) -> Result<()> {
        use std::process::Command;

        let output = Command::new("diff")
            .arg("-r")
            .arg(source)
            .arg(dest)
            .output()?;

        assert!(output.status.success(), "Directories don't match");
        Ok(())
    }
}

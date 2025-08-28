#[cfg(test)]
mod docker_tests {
    use anyhow::Result;
    use std::path::Path;
    use tempdir::TempDir;
    use uuid::Uuid;

    use rfs::fungi;
    use rfs::store::dir::DirStore;
    use rfs::DockerImageToFlist;

    #[test]
    fn test_docker_conversion() -> Result<()> {
        // Skip test if docker is not available
        if !is_docker_available() {
            println!("Docker is not available, skipping test");
            return Ok(());
        }

        // Run the test on a thread with larger stack size to avoid stack overflow
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024) // 16MB stack size
            .spawn(move || -> Result<()> {
                // Create a custom runtime with timers and IO enabled
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_time()
                    .enable_io()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    // Create temporary directories
                    let temp_dir = TempDir::new("docker-test")?;
                    let store_dir = temp_dir.path().join("store");
                    std::fs::create_dir_all(&store_dir)?;

                    // Create a store
                    let store = DirStore::new(&store_dir).await?;

                    // Create a flist writer
                    let fl_path = temp_dir.path().join("alpine-test.fl");
                    let meta = fungi::Writer::new(&fl_path, true).await?;

                    // Create a temporary directory for docker extraction
                    let container_name = Uuid::new_v4().to_string();
                    let docker_tmp_dir = TempDir::new(&container_name)?;

                    // Create DockerImageToFlist instance
                    let mut docker_to_fl = DockerImageToFlist::new(
                        meta,
                        "alpine:latest".to_string(),
                        None, // No credentials for public image
                        docker_tmp_dir,
                    );

                    // Convert docker image to flist
                    docker_to_fl.convert(store, None).await?;

                    // Verify the flist was created
                    assert!(Path::new(&fl_path).exists(), "Flist file was not created");

                    Ok(())
                })
            })
            .unwrap();

        handle.join().unwrap()
    }

    // Helper function to check if docker is available
    fn is_docker_available() -> bool {
        std::process::Command::new("docker")
            .arg("--version")
            .output()
            .is_ok()
    }
}

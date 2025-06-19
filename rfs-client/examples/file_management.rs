use rfs_client::RfsClient;
use rfs_client::types::{ClientConfig, Credentials, UploadOptions, DownloadOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with authentication
    let config = ClientConfig {
        base_url: "http://localhost:8080".to_string(),
        credentials: Some(Credentials {
            username: "user".to_string(),
            password: "password".to_string(),
        }),
        timeout_seconds: 60,
    };
    
    let mut client = RfsClient::new(config);
    
    // Authenticate with the server
    client.authenticate().await?;
    println!("Authentication successful");
    
    // Create a test file to upload
    let test_file_path = "/tmp/test_upload.txt";
    std::fs::write(test_file_path, "This is a test file for RFS client upload")?;
    println!("Created test file at {}", test_file_path);
    
    // Upload the file with options
    println!("Uploading file...");
    let upload_options = UploadOptions {
        chunk_size: Some(1024 * 1024), // 1MB chunks
        verify: true,
    };
    
    let file_hash = client.upload_file(test_file_path, Some(upload_options)).await?;
    println!("File uploaded with hash: {}", file_hash);
    
    // Download the file
    let download_path = "/tmp/test_download.txt";
    println!("Downloading file to {}...", download_path);
    
    let download_options = DownloadOptions {
        verify: true,
    };
    
    client.download_file(&file_hash, download_path, Some(download_options)).await?;
    println!("File downloaded to {}", download_path);
    
    // Verify the downloaded file matches the original
    let original_content = std::fs::read_to_string(test_file_path)?;
    let downloaded_content = std::fs::read_to_string(download_path)?;
    
    if original_content == downloaded_content {
        println!("File contents match! Download successful.");
    } else {
        println!("ERROR: File contents do not match!");
    }
    
    // Clean up test files
    std::fs::remove_file(test_file_path)?;
    std::fs::remove_file(download_path)?;
    println!("Test files cleaned up");
    
    Ok(())
}

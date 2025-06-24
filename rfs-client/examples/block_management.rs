use rfs_client::RfsClient;
use rfs_client::types::{ClientConfig, Credentials};
use openapi::models::{VerifyBlock, VerifyBlocksRequest};

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
    
    // Create a test file to upload for block testing
    let test_file_path = "/tmp/block_test.txt";
    let test_content = "This is a test file for RFS client block management";
    std::fs::write(test_file_path, test_content)?;
    println!("Created test file at {}", test_file_path);
    
    // Upload the file to get blocks
    println!("Uploading file to get blocks...");
    let file_hash = client.upload_file(test_file_path, None).await?;
    println!("File uploaded with hash: {}", file_hash);
    
    // Get blocks by file hash
    println!("Getting blocks for file hash: {}", file_hash);
    let blocks = client.get_blocks_by_hash(&file_hash).await?;
    println!("Found {} blocks for the file", blocks.blocks.len());
    
    // Print block information
    for (i, block_data) in blocks.blocks.iter().enumerate() {
        println!("Block {}: Hash={}, Index={}", i, block_data.hash, block_data.index);
    }
    
    // Verify blocks with complete information
    println!("Verifying blocks...");
    
    // Create a list of VerifyBlock objects with complete information
    let verify_blocks = blocks.blocks.iter().map(|block| {
        VerifyBlock {
            block_hash: block.hash.clone(),
            block_index: block.index,
            file_hash: file_hash.clone(), // Using the actual file hash
        }
    }).collect::<Vec<_>>();

    // Create the request with the complete block information
    for block in verify_blocks.iter() {
        println!("Block: {}", block.block_hash);
        println!("Block index: {}", block.block_index);
        println!("File hash: {}", block.file_hash);
    }
    let request = VerifyBlocksRequest { blocks: verify_blocks };
    
    // Send the verification request
    let verify_result = client.verify_blocks(request).await?;
    println!("Verification result: {} missing blocks", verify_result.missing.len());
    for block in verify_result.missing.iter() {
        println!("Missing block: {}", block);
    }
    
    // List blocks (list_blocks_handler)
    println!("\n1. Listing all blocks with pagination...");
    let blocks_list = client.list_blocks(None).await?;
    println!("Server has {} blocks in total", blocks_list.len());
    if !blocks_list.is_empty() {
        let first_few = blocks_list.iter().take(3)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!("First few blocks: {}", first_few);
    }
    
    // Check if a block exists (check_block_handler)
    if !blocks.blocks.is_empty() {
        let block_to_check = &blocks.blocks[0].hash;
        println!("\n2. Checking if block exists: {}", block_to_check);
        let exists = client.check_block(block_to_check).await?;
        println!("Block exists: {}", exists);
    }
    
    // Get block downloads statistics (get_block_downloads_handler)
    if !blocks.blocks.is_empty() {
        let block_to_check = &blocks.blocks[0].hash;
        println!("\n3. Getting download statistics for block: {}", block_to_check);
        let downloads = client.get_block_downloads(block_to_check).await?;
        println!("Block has been downloaded {} times", downloads.downloads_count);
    }
    
    // Get a specific block content (get_block_handler)
    if !blocks.blocks.is_empty() {
        let block_to_get = &blocks.blocks[0].hash;
        println!("\n4. Getting content for block: {}", block_to_get);
        let block_content = client.get_block(block_to_get).await?;
        println!("Retrieved block with {} bytes", block_content.len());
    }
    
    // Get user blocks (get_user_blocks_handler)
    println!("\n6. Listing user blocks...");
    let user_blocks = client.get_user_blocks(Some(1), Some(10)).await?;
    println!("User has {} blocks (showing page 1 with 10 per page)", user_blocks.total);
    for block in user_blocks.blocks.iter().take(3) {
        println!("  - Block: {}, Size: {}", block.hash, block.size);
    }
    
    // Upload a block (upload_block_handler)
    println!("\n7. Uploading a new test block...");
    let test_block_data = b"This is test block data for direct block upload";
    let new_file_hash = "test_file_hash_for_block_upload";
    let block_index = 0;
    let block_hash = client.upload_block(new_file_hash, block_index, test_block_data.to_vec()).await?;
    println!("Uploaded block with hash: {}", block_hash);
    
    // Clean up
    std::fs::remove_file(test_file_path)?;
    println!("Test file cleaned up");
    
    Ok(())
}

use rfs_client::RfsClient;
use rfs_client::types::{ClientConfig, Credentials, FlistOptions, WaitOptions};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parent_dir = "flists";
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
    
    println!("\n1. CREATE FLIST - Creating an FList from a Docker image");
    let image_name = "alpine:latest";
    println!("Creating FList for image: {}", image_name);
    
    // Use FlistOptions to specify additional parameters
    let options = FlistOptions {
        auth: None,
        username: None,
        password: None,
        email: None,
        server_address: Some("docker.io".to_string()),
        identity_token: None,
        registry_token: None,
    };
    
    // Create the FList and handle potential conflict error
    let job_id = match client.create_flist(&image_name, Some(options)).await {
        Ok(id) => {
            println!("FList creation started with job ID: {}", id);
            Some(id)
        },
        Err(e) => {
            if e.to_string().contains("Conflict") {
                println!("FList already exists");
                None
            } else {
                return Err(e.into());
            }
        }
    };
    
    // 2. Check FList state if we have a job ID
    if let Some(job_id) = &job_id {
        println!("\n2. GET FLIST STATE - Checking FList creation state");
        let state = client.get_flist_state(job_id).await?;
        println!("Current FList state: {:?}", state.flist_state);
        
        // 3. Wait for FList creation with progress reporting
        println!("\n3. WAIT FOR FLIST CREATION - Waiting for FList to be created with progress reporting");
        let wait_options = WaitOptions {
            timeout_seconds: 60,  // Shorter timeout for the example
            poll_interval_ms: 1000,
            progress_callback: Some(Box::new(|state| {
                println!("Progress: FList state is now {:?}", state);
                // No return value needed (returns unit type)
            })),
        };
        
        // Wait for the FList to be created (with a timeout)
        match client.wait_for_flist_creation(job_id, Some(wait_options)).await {
            Ok(final_state) => {
                println!("FList creation completed with state: {:?}", final_state);
            },
            Err(e) => {
                println!("Error waiting for FList creation: {}", e);
                // Continue with the example even if waiting fails
            }
        };
    }
    
    // 4. List all available FLists
    println!("\n4. LIST FLISTS - Listing all available FLists");
    
    // Variable to store the FList path for preview and download
    let mut flist_path_for_preview: Option<String> = None;
    
    match client.list_flists().await {
        Ok(flists) => {
            println!("Found {} FList categories", flists.len());
            
            for (category, files) in &flists {
                println!("Category: {}", category);
                for file in files.iter().take(2) { // Show only first 2 files per category
                    println!("  - {} (size: {} bytes)", file.name, file.size);
                    
                    // Save the first FList path for preview
                    if flist_path_for_preview.is_none() {
                        let path = format!("{}/{}/{}", parent_dir, category, file.name);
                        flist_path_for_preview = Some(path);
                    }
                }
                if files.len() > 2 {
                    println!("  - ... and {} more files", files.len() - 2);
                }
            }
            
            // 5. Preview an FList if we found one
            if let Some(ref flist_path) = flist_path_for_preview {
                println!("\n5. PREVIEW FLIST - Previewing FList: {}", flist_path);
                match client.preview_flist(flist_path).await {
                    Ok(preview) => {
                        println!("FList preview for {}:", flist_path);
                        println!("  - Checksum: {}", preview.checksum);
                        println!("  - Metadata: {}", preview.metadata);
                        
                        // Display content (list of strings)
                        if !preview.content.is_empty() {
                            println!("  - Content entries:");
                            for (i, entry) in preview.content.iter().enumerate().take(5) {
                                println!("    {}. {}", i+1, entry);
                            }
                            if preview.content.len() > 5 {
                                println!("    ... and {} more entries", preview.content.len() - 5);
                            }
                        }
                    },
                    Err(e) => println!("Error previewing FList: {}", e),
                }
            } else {
                println!("No FLists available for preview");
            }
        },
        Err(e) => println!("Error listing FLists: {}", e),
    }
    
    // 6. DOWNLOAD FLIST - Downloading an FList to a local file
    if let Some(ref flist_path) = flist_path_for_preview {
        println!("\n6. DOWNLOAD FLIST - Downloading FList: {}", flist_path);
        
        // Create a temporary output path for the downloaded FList
        let output_path = "/tmp/downloaded_flist.fl";
        
        match client.download_flist(flist_path, output_path).await {
            Ok(_) => {
                println!("FList successfully downloaded to {}", output_path);
                
                // Get file size
                match std::fs::metadata(output_path) {
                    Ok(metadata) => println!("Downloaded file size: {} bytes", metadata.len()),
                    Err(e) => println!("Error getting file metadata: {}", e),
                }
            },
            Err(e) => println!("Error downloading FList: {}", e),
        }
    } else {
        println!("\n6. DOWNLOAD FLIST - No FList available for download");
    }
    
    println!("\nAll FList operations demonstrated:");
    println!("1. create_flist - Create a new FList from a Docker image");
    println!("2. get_flist_state - Check the state of an FList creation job");
    println!("3. wait_for_flist_creation - Wait for an FList to be created with progress reporting");
    println!("4. list_flists - List all available FLists");
    println!("5. preview_flist - Preview the content of an FList");
    println!("6. download_flist - Download an FList to a local file");
    
    Ok(())
}

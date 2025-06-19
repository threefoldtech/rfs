use rfs_client::RfsClient;
use rfs_client::types::{ClientConfig, Credentials, WaitOptions};
use openapi::models::FlistState;

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
    
    // Create an FList from a Docker image
    let image_name = "redis:latest";
    println!("Creating FList for image: {}", image_name);
    
    let job_id = client.create_flist(&image_name, None).await?;
    println!("FList creation started with job ID: {}", job_id);
    
    // Set up options for waiting with progress reporting
    let options = WaitOptions {
        timeout_seconds: 600,  // 10 minutes timeout
        poll_interval_ms: 2000, // Check every 2 seconds
        progress_callback: Some(Box::new(|state| {
            match state {
                FlistState::FlistStateInProgress(info) => {
                    println!("Progress: {:.1}% - {}", info.in_progress.progress, info.in_progress.msg);
                },
                FlistState::FlistStateStarted(_) => {
                    println!("FList creation started...");
                },
                FlistState::FlistStateAccepted(_) => {
                    println!("FList creation request accepted...");
                },
                _ => println!("State: {:?}", state),
            }
        })),
    };
    
    // Wait for the FList to be created
    println!("Waiting for FList creation to complete...");
    
    // Use ? operator to propagate errors properly
    let state = client.wait_for_flist_creation(&job_id, Some(options)).await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    
    println!("FList created successfully!");
    println!("Final state: {:?}", state);
    
    Ok(())
}

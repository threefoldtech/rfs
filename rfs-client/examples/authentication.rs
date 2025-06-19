use rfs_client::RfsClient;
use rfs_client::types::{ClientConfig, Credentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with authentication credentials
    let config = ClientConfig {
        base_url: "http://localhost:8080".to_string(),
        credentials: Some(Credentials {
            username: "user".to_string(),
            password: "password".to_string(),
        }),
        timeout_seconds: 30,
    };
    
    let mut client = RfsClient::new(config);
    println!("Client created with authentication credentials");
    
    // Authenticate with the server
    client.authenticate().await?;
    if client.is_authenticated() {
        println!("Authentication successful");
    } else {
        println!("Authentication failed");
    }

    // Create a client without authentication
    let config_no_auth = ClientConfig {
        base_url: "http://localhost:8080".to_string(),
        credentials: None,
        timeout_seconds: 30,
    };
    
    let client_no_auth = RfsClient::new(config_no_auth);
    println!("Client created without authentication credentials");
    
    // Check health endpoint (doesn't require authentication)
    let health = client_no_auth.health_check().await?;
    println!("Server health: {:?}", health);
    
    Ok(())
}

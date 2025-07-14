mod engine;
mod ibus;

use ibus::register_ibus_engine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("Starting Thaime...");
    
    // Connect to the session bus
    let connection = zbus::Connection::session().await?;
    
    // Register our engine with IBus
    register_ibus_engine(&connection).await?;
    
    println!("Thaime initialized. Press Ctrl+C to exit.");
    
    // Keep the program running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("Received Ctrl+C, shutting down.");
    
    Ok(())
}
mod engine;
mod ibus;

use ibus::register_ibus_engine;

#[tokio::main]
async fn main() -> Result<(), Box<zbus::Error>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    println!("Starting Thaime...");
    
    // Connect to the session bus
    let connection = match zbus::Connection::session().await {
        Ok(conn) => {
            println!("Connected to session bus");
            conn
        }
        Err(e) => {
            eprintln!("Failed to connect to session bus: {}", e);
            return Err(Box::new(e));
        }
    };
    
    // Register our engine with IBus
    match register_ibus_engine(&connection).await {
        Ok(()) => println!("Engine registration successful"),
        Err(e) => {
            eprintln!("Failed to register engine: {}", e);
            return Err(e.into());
        }
    }
    
    println!("Thaime initialized. Press Ctrl+C to exit.");
    
    // Keep the program running until Ctrl+C
    match tokio::signal::ctrl_c().await {
        Ok(()) => println!("Received Ctrl+C, shutting down gracefully."),
        Err(e) => eprintln!("Error waiting for Ctrl+C: {}", e),
    }
    
    Ok(())
}
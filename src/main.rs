mod engine;
mod ibus;
mod factory;

use clap::Parser;
use ibus::register_ibus_component;

#[derive(Parser)]
#[command(name = "thaime")]
#[command(about = "Thai Input Method Engine")]
struct Args {
    /// Run in IBus mode
    #[arg(long)]
    ibus: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    let args = Args::parse();
    
    println!("Starting Thaime...");
    
    // Connect to the session bus
    let connection = match zbus::Connection::session().await {
        Ok(conn) => {
            println!("Connected to session bus");
            conn
        }
        Err(e) => {
            eprintln!("Failed to connect to session bus: {}", e);
            return Err(e.into());
        }
    };
    
    // Register our component with IBus
    match register_ibus_component(&connection, args.ibus).await {
        Ok(()) => println!("Component registration successful"),
        Err(e) => {
            eprintln!("Failed to register component: {}", e);
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
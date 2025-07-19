mod engine;
mod ibus;
mod factory;

use clap::Parser;
use log::info;
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
    
    if args.ibus {
        println!("Thaime running in IBus mode. Waiting for IBus requests...");
        
        // In IBus mode, keep the service running indefinitely
        // We need to keep the connection alive and let it process D-Bus messages
        info!("Starting main service loop...");
        
        // Create a future that keeps the connection active
        let keep_alive = async {
            // Use the connection's executor to handle incoming requests
            // This is similar to Python's GLib.MainLoop().run()
            loop {
                // Process any pending tasks
                tokio::task::yield_now().await;
                
                // Small sleep to prevent busy waiting
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        };
        
        // Use select to handle both the keep-alive loop and Ctrl+C
        tokio::select! {
            _ = keep_alive => {
                // This should never complete
                eprintln!("Keep-alive loop unexpectedly completed");
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Received Ctrl+C, shutting down gracefully.");
            }
        }
    } else {
        println!("Thaime component registration completed.");
        // In registration mode, we can exit after registration
    }
    
    Ok(())
}
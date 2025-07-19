use std::sync::Arc;
use log::info;
use zbus::{Connection, interface, fdo};
use zvariant::ObjectPath;

use crate::engine::ThaiEngine;

// IBus Factory interface
pub struct IBusEngineFactory {
    connection: Arc<Connection>,
    engine_id_counter: std::sync::atomic::AtomicU32,
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl IBusEngineFactory {
    async fn create_engine(&self, engine_name: &str) -> fdo::Result<ObjectPath<'static>> {
        info!("Creating engine: {}", engine_name);
        
        if engine_name == "thaime-rust" {
            let engine_id = self.engine_id_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let engine_path = format!("/org/freedesktop/IBus/Engine/ThaimeRust/{}", engine_id);
            
            info!("Creating engine instance with path: {}", engine_path);
            
            // Create the engine instance
            let thai_engine_core = Arc::new(ThaiEngine::new());
            let ibus_engine = crate::ibus::IBusThaiEngine::new(thai_engine_core);
            
            // Export the engine at the specific path
            let obj_path = ObjectPath::try_from(engine_path.clone()).unwrap();
            self.connection.object_server().at(&obj_path, ibus_engine).await
                .map_err(|e| fdo::Error::Failed(format!("Failed to export engine: {}", e)))?;
            
            // Return a static ObjectPath
            let static_path = ObjectPath::try_from(engine_path).unwrap();
            Ok(static_path)
        } else {
            Err(fdo::Error::Failed(format!("Unknown engine name: {}", engine_name)))
        }
    }
}

impl IBusEngineFactory {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self {
            connection,
            engine_id_counter: std::sync::atomic::AtomicU32::new(0),
        }
    }
}
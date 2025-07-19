use std::sync::Arc;
use log::{info, debug, warn};
use zbus::{Connection, interface, fdo};
use zvariant::{ObjectPath, Value};

use crate::engine::ThaiEngine;
use crate::factory::IBusEngineFactory;

// The IBus engine implementation that connects to dbus
pub struct IBusThaiEngine {
    engine: Arc<ThaiEngine>,
    // We'll need these later for more complex functionality
    preedit_string: String,
    cursor_pos: i32,
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl IBusThaiEngine {
    pub fn process_key_event(&self, keyval: u32, keycode: u32, modifiers: u32) -> fdo::Result<bool> {
        debug!("IBus process_key_event called: keyval={}, keycode={}, modifiers={}", 
                 keyval, keycode, modifiers);
        
        let result = self.engine.process_key_event(keyval, keycode, modifiers);
        Ok(result)
    }
    
    // Minimal required IBus Engine methods
    pub fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> fdo::Result<()> {
        debug!("set_cursor_location called");
        Ok(())
    }
    
    pub fn set_surrounding_text(&self, _text: &str, _cursor_pos: u32, _anchor_pos: u32) -> fdo::Result<()> {
        debug!("set_surrounding_text called");
        Ok(())
    }
    
    pub fn set_content_type(&self, _purpose: u32, _hints: u32) -> fdo::Result<()> {
        debug!("set_content_type called");
        Ok(())
    }
    
    pub fn reset(&self) -> fdo::Result<()> {
        info!("Engine reset called");
        Ok(())
    }
    
    pub fn focus_in(&self) -> fdo::Result<()> {
        info!("Engine focus_in called");
        Ok(())
    }
    
    pub fn focus_out(&self) -> fdo::Result<()> {
        info!("Engine focus_out called");
        Ok(())
    }
    
    pub fn enable(&self) -> fdo::Result<()> {
        info!("Engine enable called");
        Ok(())
    }
    
    pub fn disable(&self) -> fdo::Result<()> {
        info!("Engine disable called");
        Ok(())
    }
    
    pub fn page_up(&self) -> fdo::Result<()> {
        debug!("page_up called");
        Ok(())
    }
    
    pub fn page_down(&self) -> fdo::Result<()> {
        debug!("page_down called");
        Ok(())
    }
    
    pub fn cursor_up(&self) -> fdo::Result<()> {
        debug!("cursor_up called");
        Ok(())
    }
    
    pub fn cursor_down(&self) -> fdo::Result<()> {
        debug!("cursor_down called");
        Ok(())
    }
    
    pub fn property_activate(&self, _prop_name: &str, _prop_state: u32) -> fdo::Result<()> {
        info!("property_activate called");
        Ok(())
    }
}

impl IBusThaiEngine {
    pub fn new(engine: Arc<ThaiEngine>) -> Self {
        Self {
            engine,
            preedit_string: String::new(),
            cursor_pos: 0,
        }
    }
}

// Function to register the IBus component
pub async fn register_ibus_component(connection: &Connection, exec_by_ibus: bool) -> zbus::Result<()> {
    info!("Registering Thaime component with IBus (exec_by_ibus: {})...", exec_by_ibus);
    
    let connection_arc = Arc::new(connection.clone());
    
    if exec_by_ibus {
        info!("Running in IBus mode - registering factory and requesting bus name");
        
        // Register the factory at the standard IBus factory path
        let factory = IBusEngineFactory::new(connection_arc.clone());
        let factory_path = ObjectPath::try_from("/org/freedesktop/IBus/Factory").unwrap();
        connection.object_server().at(factory_path, factory).await?;
        info!("Factory registered at: /org/freedesktop/IBus/Factory");
        
        // Request the bus name that matches our component
        let bus_name = "org.freedesktop.IBus.ThaimaRust";
        match connection.request_name(bus_name).await {
            Ok(_) => info!("Successfully requested bus name: {}", bus_name),
            Err(e) => {
                warn!("Failed to request bus name {}: {}", bus_name, e);
                return Err(e);
            }
        }
        
    } else {
        info!("Running in registration mode - registering component with IBus daemon");
        
        // Create a proxy to the IBus service
        let ibus_proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.IBus",
            "/org/freedesktop/IBus",
            "org.freedesktop.IBus",
        ).await?;
        
        info!("Connected to IBus daemon");
        
        // Create the component description using the expected IBus format
        let component_data = create_component_description();
        
        // Register the component with IBus
        let result: Result<(), zbus::Error> = ibus_proxy.call("RegisterComponent", &(component_data,)).await;
        match result {
            Ok(_) => info!("Successfully registered component with IBus daemon"),
            Err(e) => {
                warn!("Failed to register component with IBus daemon: {}", e);
                // Don't return error here - IBus might not be running yet
                info!("Component registration attempted - IBus daemon may not be running");
            }
        }
    }
    
    info!("IBus component registration completed");
    Ok(())
}

// Create the component description in the format expected by IBus
fn create_component_description() -> Value<'static> {
    use std::collections::HashMap;
    
    // Create component description as a HashMap
    let mut component = HashMap::new();
    
    component.insert("name".to_string(), Value::from("org.freedesktop.IBus.ThaimaRust"));
    component.insert("description".to_string(), Value::from("Thaime Rust Engine"));
    component.insert("version".to_string(), Value::from("0.1.0"));
    component.insert("license".to_string(), Value::from("GPL-3.0-or-later"));
    component.insert("author".to_string(), Value::from("mimocha <chawit.leosrisook@outlook.com>"));
    component.insert("exec".to_string(), Value::from("/usr/local/bin/thaime --ibus"));
    component.insert("textdomain".to_string(), Value::from("thaime"));
    
    // Create engine descriptions
    let mut engine = HashMap::new();
    engine.insert("name".to_string(), Value::from("thaime-rust"));
    engine.insert("longname".to_string(), Value::from("Thai (thaime-rust)"));
    engine.insert("description".to_string(), Value::from("Thai Input Method Engine (Rust)"));
    engine.insert("language".to_string(), Value::from("th"));
    engine.insert("license".to_string(), Value::from("GPL-3.0-or-later"));
    engine.insert("author".to_string(), Value::from("mimocha <chawit.leosrisook@outlook.com>"));
    engine.insert("icon".to_string(), Value::from(""));
    engine.insert("layout".to_string(), Value::from("th"));
    engine.insert("rank".to_string(), Value::from(99u32));
    
    // Add engines array to component
    let engines = vec![Value::from(engine)];
    component.insert("engines".to_string(), Value::from(engines));
    
    Value::from(component)
}
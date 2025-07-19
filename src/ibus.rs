use std::sync::Arc;
use log::{info, debug};
use zbus::{Connection, interface, fdo};
use zvariant::ObjectPath;

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

// IBus Component interface - this is what registers with the IBus daemon
pub struct IBusComponent;

#[interface(name = "org.freedesktop.IBus.Component")]
impl IBusComponent {
    pub fn get_engines(&self) -> fdo::Result<Vec<(String, String, String, String, String, String, String, String, u32)>> {
        info!("get_engines called");
        // Return engine description: (name, longname, description, language, license, author, icon, layout, rank)
        let engines = vec![(
            "thaime-rust".to_string(),
            "Thai (thaime-rust)".to_string(), 
            "Thai Input Method Engine (Rust)".to_string(),
            "th".to_string(),
            "GPL-3.0-or-later".to_string(),
            "mimocha <chawit.leosrisook@outlook.com>".to_string(),
            "".to_string(), // icon
            "th".to_string(), // layout
            99u32, // rank
        )];
        Ok(engines)
    }
}

// Function to register the IBus component
pub async fn register_ibus_component(connection: &Connection, exec_by_ibus: bool) -> zbus::Result<()> {
    info!("Registering Thaime component with IBus...");
    
    let connection_arc = Arc::new(connection.clone());
    
    if exec_by_ibus {
        info!("Running in IBus mode - registering factory");
        
        // Register the factory
        let factory = IBusEngineFactory::new(connection_arc.clone());
        let factory_path = ObjectPath::try_from("/org/freedesktop/IBus/Factory").unwrap();
        connection.object_server().at(factory_path, factory).await?;
        
        // Request the bus name
        connection.request_name("org.freedesktop.IBus.ThaimaRust").await?;
    } else {
        info!("Running in registration mode - registering component");
        
        // Export the component interface
        let component = IBusComponent;
        let component_path = ObjectPath::try_from("/org/freedesktop/IBus/Component").unwrap();
        connection.object_server().at(component_path, component).await?;
        
        // Get IBus daemon proxy and register our component
        let ibus_proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.IBus",
            "/org/freedesktop/IBus",
            "org.freedesktop.IBus",
        ).await?;
        
        // Call RegisterComponent method
        let component_name = "org.freedesktop.IBus.ThaimaRust";
        let component_exec = "/usr/local/bin/thaime --ibus";
        let component_version = "0.1.0";
        let component_description = "Thaime Rust Engine";
        let component_author = "mimocha <chawit.leosrisook@outlook.com>";
        let component_license = "GPL-3.0-or-later";
        let component_textdomain = "thaime";
        
        // This is a simplified registration - in a full implementation we'd use the XML file
        let _result: Result<(), zbus::Error> = ibus_proxy.call(
            "RegisterComponent",
            &(component_name, component_description, component_version, component_license, component_author, component_exec, component_textdomain),
        ).await;
        
        info!("Component registration attempt completed");
    }
    
    info!("Thaime component registration process finished");
    Ok(())
}
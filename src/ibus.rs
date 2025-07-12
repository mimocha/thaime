use std::sync::Arc;
use zbus::{Connection, interface};
use zvariant::ObjectPath;

use crate::engine::ThaiEngine;

// The IBus engine implementation that connects to dbus
pub struct IBusThaiEngine {
    engine: Arc<ThaiEngine>,
    // We'll need these later for more complex functionality
    preedit_string: String,
    cursor_pos: i32,
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl IBusThaiEngine {
    pub fn process_key_event(&self, keyval: u32, keycode: u32, modifiers: u32) -> bool {
        self.engine.process_key_event(keyval, keycode, modifiers)
    }
    
    // Minimal required IBus Engine methods
    pub fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    pub fn set_surrounding_text(&self, _text: &str, _cursor_pos: u32, _anchor_pos: u32) {}
    pub fn set_content_type(&self, _purpose: u32, _hints: u32) {}
    pub fn reset(&self) {}
    pub fn focus_in(&self) {}
    pub fn focus_out(&self) {}
    pub fn enable(&self) {}
    pub fn disable(&self) {}
    pub fn page_up(&self) {}
    pub fn page_down(&self) {}
    pub fn cursor_up(&self) {}
    pub fn cursor_down(&self) {}
    pub fn property_activate(&self, _prop_name: &str, _prop_state: u32) {}
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
pub async fn register_ibus_engine(connection: &Connection) -> zbus::Result<()> {
    let engine = Arc::new(ThaiEngine::new());
    let thai_engine = IBusThaiEngine::new(engine);
    
    // Export the IBus engine interface
    let obj_path = ObjectPath::try_from("/org/freedesktop/IBus/Engine/Thai")
        .unwrap();
    connection.object_server().at(obj_path, thai_engine).await?;
    
    println!("Thai IME engine registered with IBus");
    Ok(())
}
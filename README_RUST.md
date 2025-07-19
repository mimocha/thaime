# Thaime Rust Engine

A Thai Input Method Engine implemented in Rust using IBus.

## Features

- Full IBus integration with proper component registration
- Factory pattern for dynamic engine creation
- Keystroke logging for debugging
- Command-line interface for testing

## Building

```bash
cargo build --release
```

## Installation

Run the setup script as root for system-wide installation:

```bash
sudo ./setup_rust.sh
```

Or as a regular user for user-specific installation:

```bash
./setup_rust.sh
```

## Testing

After installation, restart IBus:

```bash
ibus-daemon -drx
```

Verify the engine is registered:

```bash
ibus list-engine | grep thaime
```

Select the engine:

```bash
ibus engine thaime-rust
```

Or use `ibus-setup` to select it graphically.

## Manual Testing

You can test the engine without full installation:

```bash
# Test basic functionality
./target/release/thaime --help

# Test registration mode (will fail without IBus daemon)
./target/release/thaime

# Test IBus mode (will fail without IBus daemon)  
./target/release/thaime --ibus
```

## Implementation Notes

This implementation follows the IBus protocol closely:

1. **Component Registration**: The engine registers itself as an IBus component
2. **Factory Pattern**: Uses IBusEngineFactory to create engine instances dynamically
3. **D-Bus Integration**: Proper D-Bus service registration and method handling
4. **Event Handling**: Comprehensive keystroke processing and logging

## Architecture

- `main.rs`: Entry point and command-line handling
- `ibus.rs`: IBus integration and component registration
- `factory.rs`: Engine factory implementation
- `engine.rs`: Core Thai engine logic
- `thaime-rust.xml`: IBus component descriptor
- `setup_rust.sh`: Installation script

## Debugging

Logs can be viewed with:

```bash
journalctl -f | grep -i thaime
```

Or run with debug logging:

```bash
RUST_LOG=debug ./target/release/thaime --ibus
```
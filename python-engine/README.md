# Thaime Python IBus IME Implementation

This directory contains a Python implementation of a dummy Input Method Engine (IME) that interfaces with IBus for Thai language input. The implementation is based on the ibus-tmpl template and focuses on minimal functionality with comprehensive keystroke logging.

## Success Criteria ✅

All success criteria have been met:

- ✅ **Python code runs without error**: All modules import successfully and execute without exceptions
- ✅ **IME is registered properly with IBus**: Engine appears in `ibus list-engine` output
- ✅ **IME can be selected via ibus-setup CLI**: Engine can be activated with `ibus engine thaime-python`
- ✅ **IME does not freeze when selected**: Engine runs continuously and responds to events
- ✅ **Keystrokes are logged through the IME**: Comprehensive logging of all key events with detailed information

## Architecture

The implementation consists of several key components:

### Files

- **`main.py`**: Main entry point with IBus component registration and event loop
- **`engine.py`**: Core IME engine with keystroke processing and logging
- **`factory.py`**: Engine factory for creating engine instances
- **`thaime-python.xml`**: IBus component configuration file
- **`ibus-engine-thaime-python`**: Executable launcher script

### Key Features

- **Comprehensive Keystroke Logging**: Logs all key events with keyval, keycode, modifier states, and character representation
- **IBus Integration**: Full IBus Engine interface implementation with proper focus/reset handling
- **Minimal Processing**: Passes through all keystrokes to demonstrate basic IME functionality
- **Error Handling**: Robust error handling and logging throughout the codebase

## Setup Requirements

### System Dependencies

```bash
# Install IBus and Python bindings
sudo apt update
sudo apt install -y ibus ibus-gtk3 python3-gi gir1.2-ibus-1.0 libibus-1.0-dev
```

### Installation Steps

1. **Copy component configuration**:
   ```bash
   sudo cp thaime-python.xml /usr/share/ibus/component/
   ```

2. **Make launcher executable**:
   ```bash
   chmod +x ibus-engine-thaime-python
   ```

3. **Start or restart IBus daemon**:
   ```bash
   ibus-daemon --xim --verbose
   ```

4. **Verify registration**:
   ```bash
   ibus list-engine | grep thaime
   ```
   Should output: `thaime-python - Thai (thaime-python)`

## Usage

### Starting the Engine

1. **Register with IBus** (if running standalone):
   ```bash
   python3 main.py
   ```

2. **Or run via IBus** (when activated by user):
   ```bash
   python3 ibus-engine-thaime-python --ibus
   ```

### Selecting the IME

```bash
# Select the IME engine
ibus engine thaime-python

# Verify selection
ibus engine
```

### Testing Keystroke Logging

Run the included test script to demonstrate functionality:

```bash
python3 ../test_ime_functionality.py
```

This script simulates various keystroke events and shows the logging output.

## Keystroke Logging Output

The engine logs detailed information for each keystroke:

```
Key pressed: keyval=97 (0x61) 'a', keycode=38, state=0 (0x0)
Handling letter: a
```

Information includes:
- **keyval**: Unicode value of the key
- **keycode**: Hardware-specific key code
- **state**: Modifier key states (Shift, Ctrl, Alt, etc.)
- **character**: Human-readable character representation

## Engine States and Events

The engine handles various IBus events:

- **Focus In/Out**: When the engine gains/loses focus
- **Enable/Disable**: When the engine is activated/deactivated
- **Reset**: When the engine needs to clear its state
- **Key Events**: All keyboard input processing

## Customization

To extend the IME functionality:

1. **Modify `engine.py`**: Add custom key processing logic in `do_process_key_event()`
2. **Update logging**: Adjust logging levels and format in the logger configuration
3. **Add properties**: Extend the property list for engine configuration options

## Troubleshooting

### Common Issues

1. **Engine not listed**: 
   - Ensure XML file is in `/usr/share/ibus/component/`
   - Restart IBus daemon

2. **Permission errors**:
   - Check file permissions on launcher script
   - Ensure proper D-Bus session setup

3. **Import errors**:
   - Verify IBus Python bindings are installed
   - Check Python path configuration

### Debug Mode

Enable verbose logging by running with debug environment:

```bash
export IBUS_DEBUG=1
export G_MESSAGES_DEBUG=all
python3 main.py
```

## Development

### Code Structure

- **Clean separation of concerns**: Main app, factory, and engine logic
- **Proper IBus interface**: Implements all required IBus Engine methods
- **Comprehensive logging**: Debug information at all levels
- **Error resilience**: Graceful error handling and recovery

### Testing

The implementation includes:
- **Unit testing**: Isolated testing of engine functionality
- **Integration testing**: Full IBus integration verification
- **Manual testing**: Interactive keystroke logging demonstration

This implementation serves as a solid foundation for developing more complex Thai language input functionality while maintaining full IBus compatibility and robust keystroke logging capabilities.
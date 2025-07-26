# Thaime Python IBus IME Implementation

This directory contains a Python implementation of an Input Method Engine (IME) that interfaces with IBus for Thai language input.
The implementation is based on the Python [ibus-tmpl template](https://github.com/phuang/ibus-tmpl).

## Files

- **`main.py`**: Main entry point with IBus component registration and event loop
- **`engine.py`**: Core IME engine with keystroke processing and logging
- **`factory.py`**: Engine factory for creating engine instances
- **`thaime-python.xml`**: IBus component configuration file
- **`ibus-engine-thaime-python`**: Executable launcher script

## Setup Requirements

### Install System Dependencies

Install IBus, Python IBus binding, and GObject Introspection bindings

**Ubuntu:**

```bash
sudo apt update
sudo apt install -y ibus ibus-gtk3 python3-gi gir1.2-ibus-1.0 libibus-1.0-dev
```

**Fedora:**

```bash
sudo dnf update
sudo dnf install -y ibus ibus-devel ibus-gtk3 gobject-introspection gobject-introspection-devel python3-gobject-base python3-gobject-devel
```

### Installation Steps

1. **Copy component configuration for IBus**:
   ```bash
   sudo cp python-engine/thaime-python.xml /usr/share/ibus/component/
   ```

2. **Make launcher executable**:
   ```bash
   chmod +x ibus-engine-thaime-python
   ```

3. **Start or restart IBus daemon**:
   ```bash
   ibus-daemon -drxv
   ```

4. **Verify registration**:
   ```bash
   ibus list-engine | grep thaime
   ```
   Should output: `thaime - Thaime`

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

### Documentations

Find the IBus documentations from here:
- [github/ibus/wiki](https://github.com/ibus/ibus/wiki)
- [IBus Python Docs](https://lazka.github.io/pgi-docs/#IBus-1.0)
- [IBus C Docs](https://ibus.github.io/docs/ibus-1.5/index.html)

Find relevant GNOME Python API (GLib) documentations from here:
- [Pygobject](https://api.pygobject.gnome.org/)
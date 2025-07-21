# Thaime Python Engine

A Python-based IBus Input Method Engine (IME) with three input modes for different typing needs.

## Features

### Three Input Modes

1. **Latin Mode** (`thaime-python-latin`)
   - Standard QWERTY keyboard passthrough
   - For normal English/Latin text input
   - Layout: US

2. **Thai-Kedmanee Mode** (`thaime-python-kedmanee`)
   - QWERTY to Thai character conversion
   - Uses standard Thai Kedmanee layout mapping
   - Layout: Thai
   - Full support for Thai characters, numerals, and punctuation

3. **Thaime Mode** (`thaime-python-thaime`)
   - Experimental Latin-to-Thai conversion engine
   - Currently implements keystroke logging for development
   - Layout: US
   - Foundation for future intelligent conversion features

### IBus Integration

- Full IBus compatibility
- Proper component registration with three selectable engines
- Support for engine switching via standard desktop input method settings
- Comprehensive logging and debugging capabilities

## Installation

### Prerequisites

- Ubuntu/Debian: `sudo apt install ibus ibus-gtk3 python3-gi gir1.2-ibus-1.0 libibus-1.0-dev`
- Python 3.x with GObject Introspection bindings

### Setup

1. Navigate to the python-engine directory:
   ```bash
   cd python-engine
   ```

2. Run the setup script:
   ```bash
   ./setup.sh
   ```

3. Restart IBus daemon:
   ```bash
   ibus-daemon -drx
   ```

4. Verify engine registration:
   ```bash
   ibus list-engine | grep thaime
   ```

## Usage

### Selecting Input Modes

Use your desktop environment's input method settings to select from:
- **Latin (thaime-python)** - Standard QWERTY input
- **Thai Kedmanee (thaime-python)** - Thai text input with Kedmanee layout
- **Thaime (thaime-python)** - Experimental conversion engine

### Thai-Kedmanee Layout

The Thai-Kedmanee mode provides complete Thai character input:

```
QWERTY: a s d f g h j k l
Thai:   ฟ ห ก ด เ ้ ่ า ส
```

Example conversions:
- `hello` → `้ำสสน`
- `asdf` → `ฟหกด`
- `123` → `ๅ/_`

## Testing

### Quick Test
```bash
python3 test_three_modes.py
```

### Comprehensive Integration Test
```bash
python3 test_integration.py
```

### Original Keystroke Logging Test
```bash
python3 test_ime_functionality.py
```

## Development

### Architecture

- `engine.py` - Core engine implementations (BaseEngine, LatinEngine, ThaiKedmaneeEngine, ThaiimeEngine)
- `factory.py` - IBus engine factory for creating engine instances
- `main.py` - IBus component registration and main application
- `thai_keymap.py` - Thai Kedmanee keyboard layout mapping
- `thaime-python.xml` - IBus component configuration

### Adding New Features

1. Extend the appropriate engine class in `engine.py`
2. Update the keymap in `thai_keymap.py` if needed
3. Add tests to verify functionality
4. Update documentation

### Debugging

All engines provide detailed logging. To see debug output:
```bash
# Run with debug logging
PYTHONPATH=. python3 main.py --ibus
```

## Future Plans

The Thaime mode is designed to evolve into an intelligent Latin-to-Thai conversion engine that can:
- Analyze Latin text patterns
- Apply phonetic conversion rules
- Provide context-aware Thai text suggestions
- Learn from user corrections and preferences

The current keystroke logging implementation provides the foundation for developing these advanced features.

## Technical Details

For detailed technical information about the implementation, see [THREE_MODES_README.md](THREE_MODES_README.md).

## License

GPL-3.0-or-later
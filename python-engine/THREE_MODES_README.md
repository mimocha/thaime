# Thaime Python IME - Three Modes Implementation

This document describes the three-mode implementation of the Thaime Python Input Method Engine (IME) for IBus.

## Overview

The Thaime Python IME now supports three distinct input modes:

1. **Latin Mode** - Standard QWERTY keyboard passthrough
2. **Thai-Kedmanee Mode** - QWERTY to Thai character conversion using the Kedmanee layout
3. **Thaime Mode** - Dummy implementation for future Latin-to-Thai conversion with keystroke logging

## Engine Architecture

### Base Engine Class (`BaseEngine`)

All three modes inherit from `BaseEngine`, which provides:
- Common IBus event handling
- Logging infrastructure
- Property management
- Base key processing framework

### Engine Implementations

#### 1. Latin Engine (`LatinEngine`)
- **Purpose**: Standard QWERTY keyboard input
- **Behavior**: Passes all keystrokes through to the system without modification
- **Use case**: When users want normal Latin text input

#### 2. Thai-Kedmanee Engine (`ThaiKedmaneeEngine`)
- **Purpose**: Thai text input using the standard Kedmanee layout
- **Behavior**: Converts QWERTY keystrokes to Thai characters using the Kedmanee mapping
- **Use case**: Traditional Thai text input for users familiar with Kedmanee layout

#### 3. Thaime Engine (`ThaiimeEngine`)
- **Purpose**: Future Latin-to-Thai conversion engine (currently dummy implementation)
- **Behavior**: Logs all keystrokes for analysis, passes through to system
- **Use case**: Development/testing of the core Latin-to-Thai conversion algorithm

## Thai Kedmanee Layout

The implementation includes a comprehensive Thai Kedmanee keyboard layout mapping (`thai_keymap.py`):

- Complete character mappings for unshifted and shifted keys
- Support for Thai numerals, punctuation, and special characters
- Text conversion utilities (`qwerty_to_thai`, `thai_to_qwerty`)

### Example Mappings:
- `a` → `ฟ` (THAI CHARACTER FO FAN)
- `s` → `ห` (THAI CHARACTER HO HIP)
- `d` → `ก` (THAI CHARACTER KO KAI)
- `1` → `ๅ` (THAI CHARACTER ANGKHANKHU)

## IBus Integration

### Component Registration

The IME registers as a single IBus component (`org.freedesktop.IBus.ThaimePython`) with three engines:

```xml
<engines>
    <engine>
        <name>thaime-python-latin</name>
        <longname>Latin (thaime-python)</longname>
        <language>en</language>
        <layout>us</layout>
    </engine>
    <engine>
        <name>thaime-python-kedmanee</name>
        <longname>Thai Kedmanee (thaime-python)</longname>
        <language>th</language>
        <layout>th</layout>
    </engine>
    <engine>
        <name>thaime-python-thaime</name>
        <longname>Thaime (thaime-python)</longname>
        <language>th</language>
        <layout>us</layout>
    </engine>
</engines>
```

### Engine Factory

The `EngineFactory` class handles engine instantiation based on the requested engine name, creating the appropriate engine type for each mode.

## Installation and Usage

### Installation

1. Run the setup script:
   ```bash
   cd python-engine
   ./setup.sh
   ```

2. Restart IBus:
   ```bash
   ibus-daemon -drx
   ```

3. Verify registration:
   ```bash
   ibus list-engine | grep thaime
   ```

### Usage

Users can select any of the three engines through their desktop environment's input method settings:

- **Latin (thaime-python)** - For standard English typing
- **Thai Kedmanee (thaime-python)** - For Thai typing with Kedmanee layout
- **Thaime (thaime-python)** - For testing the experimental conversion engine

## Testing

The implementation includes comprehensive tests:

### Unit Tests
- `test_three_modes.py` - Basic functionality testing for all three modes
- `test_integration.py` - Complete integration testing including IBus component registration

### Running Tests
```bash
cd python-engine

# Test basic functionality
python3 test_three_modes.py

# Run comprehensive integration tests
python3 test_integration.py

# Test original keystroke logging
python3 test_ime_functionality.py
```

## Technical Details

### Key Processing Flow

1. **Key Event Reception**: All key events go through `do_process_key_event()`
2. **Base Processing**: Common logging and validation in `BaseEngine`
3. **Mode-Specific Processing**: Each engine implements `_process_key_specific()`
4. **Character Output**: Thai-Kedmanee mode commits converted characters via IBus

### Character Conversion (Thai-Kedmanee Mode)

```python
def _process_key_specific(self, keyval, keycode, state):
    if 32 <= keyval <= 126:  # Printable character
        char = chr(keyval)
        thai_char = KEDMANEE_KEYMAP.get(char)
        if thai_char:
            text = IBus.Text.new_from_string(thai_char)
            self.commit_text(text)
            return True  # Consume the key event
    return False  # Let system handle
```

## Future Development

The Thaime mode is designed as a foundation for implementing the core Latin-to-Thai conversion algorithm. The current dummy implementation provides:

- Complete keystroke logging
- IBus integration framework
- Testing infrastructure

Future enhancements will replace the passthrough behavior with intelligent Latin-to-Thai conversion logic.

## Backwards Compatibility

The implementation maintains backwards compatibility with the original `thaime-python` engine name, which maps to the Thaime mode for existing configurations.
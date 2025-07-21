#!/usr/bin/env python3
"""
Test script to demonstrate the three modes of the Thaime Python IME:
1. Latin mode (QWERTY passthrough)
2. Thai-Kedmanee mode (QWERTY to Thai conversion)
3. Thaime mode (keystroke logging for future Latin-to-Thai)
"""

import sys
import os
import logging

# Add the python-engine directory to Python path
sys.path.insert(0, os.path.dirname(__file__))

import gi
gi.require_version('IBus', '1.0')
from gi.repository import IBus
from gi.repository import GLib

import engine

def test_engine_modes():
    """Test all three engine modes"""
    
    # Setup logging
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )
    
    print("Testing Thaime Python IME - Three Modes")
    print("=" * 60)
    
    # Create a mock bus connection (simplified for testing)
    class MockBus:
        def get_connection(self):
            return None
    
    mock_bus = MockBus()
    
    # Test cases
    test_cases = [
        # (keyval, keycode, state, description)
        (ord('a'), 38, 0, "Letter 'a'"),
        (ord('s'), 39, 0, "Letter 's'"),
        (ord('d'), 40, 0, "Letter 'd'"),
        (ord('h'), 43, 0, "Letter 'h'"),
        (ord('A'), 38, IBus.ModifierType.SHIFT_MASK, "Letter 'A' (with Shift)"),
        (ord('1'), 10, 0, "Number '1'"),
        (IBus.KEY_space, 65, 0, "Space key"),
    ]
    
    # Test each engine mode
    engines = [
        ("Latin Mode", engine.LatinEngine),
        ("Thai-Kedmanee Mode", engine.ThaiKedmaneeEngine),
        ("Thaime Mode", engine.ThaiimeEngine),
    ]
    
    for engine_name, engine_class in engines:
        print(f"\n{engine_name}")
        print("-" * len(engine_name))
        
        # Create engine instance
        test_engine = engine_class(mock_bus, f"/test/{engine_name.lower().replace(' ', '_')}/path")
        
        # Test key events
        for keyval, keycode, state, description in test_cases:
            print(f"\nTesting: {description}")
            try:
                # Simulate key press event
                result = test_engine.do_process_key_event(keyval, keycode, state)
                print(f"  Result: {'Handled by IME' if result else 'Passed through to system'}")
            except Exception as e:
                print(f"  Error: {e}")
    
    print("\n" + "=" * 60)
    print("Three-mode test completed!")
    print("\nSummary:")
    print("- Latin Mode: Passes all keys through to system (standard QWERTY)")
    print("- Thai-Kedmanee Mode: Converts QWERTY keys to Thai characters")
    print("- Thaime Mode: Logs keystrokes for future Latin-to-Thai conversion")

def test_thai_keymap():
    """Test Thai keymap conversion specifically"""
    print("\n" + "=" * 60)
    print("Thai Kedmanee Keymap Test")
    print("-" * 30)
    
    import thai_keymap
    
    test_words = ["hello", "world", "asdf", "qwerty", "123"]
    
    for word in test_words:
        thai_word = thai_keymap.qwerty_to_thai(word)
        print(f"'{word}' -> '{thai_word}'")

if __name__ == "__main__":
    test_engine_modes()
    test_thai_keymap()
#!/usr/bin/env python3
"""
Complete integration test for the Thaime Python IME three-mode implementation.
This test verifies that:
1. All three engines can be instantiated
2. Each engine behaves according to its specification
3. The factory can create all three engine types
4. IBus component registration works
"""

import sys
import os
import logging

# Add the python-engine directory to Python path
sys.path.insert(0, os.path.dirname(__file__))

import gi
gi.require_version('IBus', '1.0')
from gi.repository import IBus, GLib

import main
import factory
import engine

def test_component_registration():
    """Test that the IBus component is properly configured"""
    print("=" * 60)
    print("Testing IBus Component Registration")
    print("=" * 60)
    
    # Create the IMApp
    app = main.IMApp(False)  # Not executed by ibus
    component = app._IMApp__component
    
    print(f"Component name: {component.get_name()}")
    print(f"Component description: {component.get_description()}")
    print(f"Component version: {component.get_version()}")
    
    engines = component.get_engines()
    print(f"\nNumber of engines registered: {len(engines)}")
    
    expected_engines = [
        ("thaime-python-latin", "Latin (thaime-python)", "en", "us"),
        ("thaime-python-kedmanee", "Thai Kedmanee (thaime-python)", "th", "th"),
        ("thaime-python-thaime", "Thaime (thaime-python)", "th", "us"),
    ]
    
    print("\nEngine details:")
    for i, engine_desc in enumerate(engines):
        name = engine_desc.get_name()
        longname = engine_desc.get_longname()
        language = engine_desc.get_language()
        layout = engine_desc.get_layout()
        description = engine_desc.get_description()
        
        print(f"  {i+1}. {name}")
        print(f"     Long name: {longname}")
        print(f"     Language: {language}")
        print(f"     Layout: {layout}")
        print(f"     Description: {description}")
        
        # Verify expected values
        expected = expected_engines[i]
        assert name == expected[0], f"Expected name {expected[0]}, got {name}"
        assert longname == expected[1], f"Expected longname {expected[1]}, got {longname}"
        assert language == expected[2], f"Expected language {expected[2]}, got {language}"
        assert layout == expected[3], f"Expected layout {expected[3]}, got {layout}"
        
    print("\n✓ All engines registered correctly!")
    return True

def test_factory_creation():
    """Test that the factory can create all three engine types"""
    print("\n" + "=" * 60)
    print("Testing Engine Factory")
    print("=" * 60)
    
    class MockBus:
        def get_connection(self):
            return None
    
    mock_bus = MockBus()
    test_factory = factory.EngineFactory(mock_bus)
    
    engine_types = [
        ("thaime-python-latin", engine.LatinEngine, "Latin"),
        ("thaime-python-kedmanee", engine.ThaiKedmaneeEngine, "Thai-Kedmanee"),
        ("thaime-python-thaime", engine.ThaiimeEngine, "Thaime"),
        ("thaime-python", engine.ThaiimeEngine, "Thaime"),  # Legacy compatibility
    ]
    
    for engine_name, expected_class, expected_mode in engine_types:
        print(f"\nTesting factory creation of: {engine_name}")
        
        created_engine = test_factory.do_create_engine(engine_name)
        
        if created_engine is None:
            print(f"  ✗ Factory returned None for {engine_name}")
            return False
        
        if not isinstance(created_engine, expected_class):
            print(f"  ✗ Wrong engine type: expected {expected_class}, got {type(created_engine)}")
            return False
        
        if hasattr(created_engine, 'engine_name') and created_engine.engine_name != expected_mode:
            print(f"  ✗ Wrong engine mode: expected {expected_mode}, got {created_engine.engine_name}")
            return False
        
        print(f"  ✓ Successfully created {engine_name} -> {type(created_engine).__name__}")
    
    # Test unknown engine
    unknown_engine = test_factory.do_create_engine("unknown-engine")
    if unknown_engine is not None:
        print(f"  ✗ Factory should return None for unknown engine, got {type(unknown_engine)}")
        return False
    
    print("  ✓ Factory correctly rejected unknown engine")
    print("\n✓ Factory creation test passed!")
    return True

def test_engine_behavior():
    """Test the specific behavior of each engine type"""
    print("\n" + "=" * 60)
    print("Testing Engine Behavior")
    print("=" * 60)
    
    class MockBus:
        def get_connection(self):
            return None
    
    mock_bus = MockBus()
    
    # Test cases
    test_cases = [
        (ord('a'), 38, 0, "Letter 'a'"),
        (ord('s'), 39, 0, "Letter 's'"),  
        (ord('1'), 10, 0, "Number '1'"),
        (IBus.KEY_space, 65, 0, "Space key"),
    ]
    
    engines = [
        ("Latin", engine.LatinEngine(mock_bus, "/test/latin")),
        ("Thai-Kedmanee", engine.ThaiKedmaneeEngine(mock_bus, "/test/kedmanee")),
        ("Thaime", engine.ThaiimeEngine(mock_bus, "/test/thaime")),
    ]
    
    for engine_name, test_engine in engines:
        print(f"\n{engine_name} Engine:")
        print("-" * (len(engine_name) + 8))
        
        for keyval, keycode, state, description in test_cases:
            result = test_engine.do_process_key_event(keyval, keycode, state)
            expected_result = False if engine_name in ["Latin", "Thaime"] else True
            
            if result != expected_result:
                print(f"  ✗ {description}: expected {'handled' if expected_result else 'passthrough'}, got {'handled' if result else 'passthrough'}")
                return False
            else:
                print(f"  ✓ {description}: {'handled' if result else 'passthrough'}")
    
    print("\n✓ All engine behavior tests passed!")
    return True

def test_thai_keymap_integration():
    """Test Thai keymap integration"""
    print("\n" + "=" * 60)
    print("Testing Thai Keymap Integration")
    print("=" * 60)
    
    import thai_keymap
    
    # Test basic mapping
    test_cases = [
        ('a', 'ฟ'),
        ('s', 'ห'),
        ('d', 'ก'),
        ('f', 'ด'),
        ('1', 'ๅ'),
        (' ', ' '),
    ]
    
    for qwerty_char, expected_thai in test_cases:
        thai_char = thai_keymap.KEDMANEE_KEYMAP.get(qwerty_char, qwerty_char)
        if thai_char != expected_thai:
            print(f"  ✗ '{qwerty_char}' -> '{thai_char}' (expected '{expected_thai}')")
            return False
        else:
            print(f"  ✓ '{qwerty_char}' -> '{thai_char}'")
    
    # Test text conversion
    test_text = "hello"
    converted = thai_keymap.qwerty_to_thai(test_text)
    expected = "้ำสสน"
    
    if converted != expected:
        print(f"  ✗ Text conversion: '{test_text}' -> '{converted}' (expected '{expected}')")
        return False
    else:
        print(f"  ✓ Text conversion: '{test_text}' -> '{converted}'")
    
    print("\n✓ Thai keymap integration test passed!")
    return True

def run_all_tests():
    """Run all tests"""
    # Setup logging to be less verbose for tests
    logging.basicConfig(
        level=logging.WARNING,  # Reduce noise for tests
        format='%(name)s - %(levelname)s - %(message)s'
    )
    
    print("Thaime Python IME - Complete Integration Test")
    print("=" * 60)
    
    all_tests_passed = True
    
    try:
        # Run all test functions
        test_functions = [
            test_component_registration,
            test_factory_creation,
            test_engine_behavior,
            test_thai_keymap_integration,
        ]
        
        for test_func in test_functions:
            if not test_func():
                all_tests_passed = False
                break
        
        if all_tests_passed:
            print("\n" + "=" * 60)
            print("🎉 ALL TESTS PASSED! 🎉")
            print("=" * 60)
            print()
            print("Summary:")
            print("✓ IBus component registration works correctly")
            print("✓ Factory can create all three engine types")
            print("✓ Latin mode passes through all keys")
            print("✓ Thai-Kedmanee mode converts QWERTY to Thai")
            print("✓ Thaime mode logs keystrokes (dummy implementation)")
            print("✓ Thai keymap integration works correctly")
            print()
            print("The three-mode implementation is ready for IBus deployment!")
            
        else:
            print("\n" + "=" * 60)
            print("❌ SOME TESTS FAILED")
            print("=" * 60)
            return 1
            
    except Exception as e:
        print(f"\n❌ Test failed with exception: {e}")
        import traceback
        traceback.print_exc()
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(run_all_tests())
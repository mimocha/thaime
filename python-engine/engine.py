import logging

import gi
gi.require_version('IBus', '1.0')

from gi.repository import GLib, IBus
from thai_keymap import KEDMANEE_KEYMAP


class BaseEngine(IBus.Engine):
    """Base engine class with common functionality"""
    
    def __init__(self, bus, object_path, engine_name):
        super(BaseEngine, self).__init__(
            connection=bus.get_connection(),
            object_path=object_path
        )
        self.__bus = bus
        self.engine_name = engine_name
        self.logger = logging.getLogger(f'thaime-python.{engine_name}')
        self.logger.info(f"Engine created: {engine_name} with path: {object_path}")
        
        # Initialize engine state
        self.__preedit_string = ""
        self.__lookup_table = IBus.LookupTable()
        self.__is_invalidate = False
        
        # Create property list for the engine
        self.__prop_list = IBus.PropList()
        self.__prop_list.append(IBus.Property(
            key="mode",
            prop_type=IBus.PropType.NORMAL,
            label=IBus.Text.new_from_string(f"Mode: {engine_name}"),
            icon="input-keyboard",
            tooltip=IBus.Text.new_from_string(f"Thaime {engine_name} Mode"),
            sensitive=True,
            visible=True,
            state=IBus.PropState.UNCHECKED,
            sub_props=None
        ))

    def do_process_key_event(self, keyval, keycode, state):
        """Process key events - to be overridden by specific engines"""
        # Ignore key release events
        is_press = ((state & IBus.ModifierType.RELEASE_MASK) == 0)
        if not is_press:
            return False

        # Log the keystroke
        key_char = chr(keyval) if 32 <= keyval <= 126 else f"<{keyval}>"
        self.logger.info(f"Key pressed: keyval={keyval} (0x{keyval:x}) '{key_char}', keycode={keycode}, state={state} (0x{state:x})")
        
        return self._process_key_specific(keyval, keycode, state)
    
    def _process_key_specific(self, keyval, keycode, state):
        """Override this method in specific engine implementations"""
        return False

    def do_focus_in(self):
        """Called when the engine gains focus"""
        self.logger.info(f"{self.engine_name} engine focused in")
        self.register_properties(self.__prop_list)

    def do_focus_out(self):
        """Called when the engine loses focus"""
        self.logger.info(f"{self.engine_name} engine focused out")

    def do_reset(self):
        """Called when the engine needs to be reset"""
        self.logger.info(f"{self.engine_name} engine reset")
        self.__preedit_string = ""
        self.__invalidate()

    def do_enable(self):
        """Called when the engine is enabled"""
        self.logger.info(f"{self.engine_name} engine enabled")

    def do_disable(self):
        """Called when the engine is disabled"""
        self.logger.info(f"{self.engine_name} engine disabled")

    def do_set_cursor_location(self, x, y, w, h):
        """Called when the cursor location changes"""
        self.logger.debug(f"Cursor location: x={x}, y={y}, w={w}, h={h}")

    def do_set_surrounding_text(self, text, cursor_pos, anchor_pos):
        """Called when surrounding text changes"""
        self.logger.debug(f"Surrounding text: '{text}', cursor={cursor_pos}, anchor={anchor_pos}")

    def do_property_activate(self, prop_name, state):
        """Called when a property is activated"""
        self.logger.info(f"Property activated: {prop_name}, state={state}")

    def __invalidate(self):
        """Mark engine for update"""
        if self.__is_invalidate:
            return
        self.__is_invalidate = True
        GLib.idle_add(self.__update, priority=GLib.PRIORITY_LOW)

    def __update(self):
        """Update the engine state"""
        self.logger.debug("Updating engine state")
        self.__is_invalidate = False


class LatinEngine(BaseEngine):
    """Latin input mode - standard QWERTY passthrough"""
    
    def __init__(self, bus, object_path):
        super(LatinEngine, self).__init__(bus, object_path, "Latin")
    
    def _process_key_specific(self, keyval, keycode, state):
        """Latin mode: just pass through all keys"""
        self.logger.debug("Latin mode: passing through key")
        return False  # Let the system handle it normally


class ThaiKedmaneeEngine(BaseEngine):
    """Thai Kedmanee input mode - converts QWERTY to Thai characters"""
    
    def __init__(self, bus, object_path):
        super(ThaiKedmaneeEngine, self).__init__(bus, object_path, "Thai-Kedmanee")
    
    def _process_key_specific(self, keyval, keycode, state):
        """Thai Kedmanee mode: convert QWERTY keys to Thai characters"""
        # Skip if modifier keys are pressed (except Shift for uppercase)
        if state & (IBus.ModifierType.CONTROL_MASK | IBus.ModifierType.MOD1_MASK):
            return False
        
        # Convert key to character if it's printable
        if 32 <= keyval <= 126:
            char = chr(keyval)
            thai_char = KEDMANEE_KEYMAP.get(char)
            
            if thai_char:
                self.logger.info(f"Converting '{char}' to '{thai_char}'")
                # Commit the Thai character
                text = IBus.Text.new_from_string(thai_char)
                self.commit_text(text)
                return True  # Consume the key event
            
        return False  # Let the system handle it normally


class ThaiimeEngine(BaseEngine):
    """Thaime mode - Latin-to-Thai conversion engine (dummy implementation with keystroke logging)"""
    
    def __init__(self, bus, object_path):
        super(ThaiimeEngine, self).__init__(bus, object_path, "Thaime")
    
    def _process_key_specific(self, keyval, keycode, state):
        """Thaime mode: for now, just keystroke logging as before"""
        # For demonstration, we'll handle a few specific keys
        if keyval in range(ord('a'), ord('z') + 1):  # lowercase letters
            if state & (IBus.ModifierType.CONTROL_MASK | IBus.ModifierType.MOD1_MASK) == 0:
                self.logger.info(f"Handling letter: {chr(keyval)}")
                # For demo purposes, just pass through - a real IME would do transformation
                return False  # Let the system handle it normally
        elif keyval in range(ord('A'), ord('Z') + 1):  # uppercase letters
            if state & (IBus.ModifierType.CONTROL_MASK | IBus.ModifierType.MOD1_MASK) == 0:
                self.logger.info(f"Handling uppercase letter: {chr(keyval)}")
                return False  # Let the system handle it normally
        elif keyval in range(ord('0'), ord('9') + 1):  # numbers
            self.logger.info(f"Handling number: {chr(keyval)}")
            return False  # Let the system handle it normally
        elif keyval == IBus.KEY_space:
            self.logger.info("Handling space key")
            return False  # Let the system handle it normally
        elif keyval == IBus.KEY_Return:
            self.logger.info("Handling Enter key")
            return False  # Let the system handle it normally
        elif keyval == IBus.KEY_BackSpace:
            self.logger.info("Handling Backspace key")
            return False  # Let the system handle it normally
        elif keyval == IBus.KEY_Tab:
            self.logger.info("Handling Tab key")
            return False  # Let the system handle it normally
        elif keyval == IBus.KEY_Escape:
            self.logger.info("Handling Escape key")
            return False  # Let the system handle it normally
        else:
            self.logger.info(f"Unhandled key: {keyval}")
            return False  # Let the system handle it normally

        return False  # Default: don't consume the key


# For backwards compatibility, keep the original Engine class as an alias
Engine = ThaiimeEngine
import logging

import gi
gi.require_version('IBus', '1.0')

from gi.repository import GLib, IBus


class Engine(IBus.Engine):
    def __init__(self, bus, object_path):
        super(Engine, self).__init__(
            connection=bus.get_connection(),
            object_path=object_path
        )
        self.__bus = bus
        self.logger = logging.getLogger('thaime-python.engine')
        self.logger.info(f"Engine created with path: {object_path}")
        
        # Initialize engine state
        self.__preedit_string = ""
        self.__lookup_table = IBus.LookupTable()
        self.__is_invalidate = False
        
        # Create property list for the engine
        self.__prop_list = IBus.PropList()
        self.__prop_list.append(IBus.Property(
            key="test",
            prop_type=IBus.PropType.NORMAL,
            label=IBus.Text.new_from_string("Test"),
            icon="input-keyboard",
            tooltip=IBus.Text.new_from_string("Thaime Test Property"),
            sensitive=True,
            visible=True,
            state=IBus.PropState.UNCHECKED,
            sub_props=None
        ))

    def do_process_key_event(self, keyval, keycode, state):
        """Process key events and log keystrokes"""
        # Ignore key release events
        is_press = ((state & IBus.ModifierType.RELEASE_MASK) == 0)
        if not is_press:
            return False

        # Log the keystroke
        key_char = chr(keyval) if 32 <= keyval <= 126 else f"<{keyval}>"
        self.logger.info(f"Key pressed: keyval={keyval} (0x{keyval:x}) '{key_char}', keycode={keycode}, state={state} (0x{state:x})")
        
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

    def do_focus_in(self):
        """Called when the engine gains focus"""
        self.logger.info("Engine focused in")
        self.register_properties(self.__prop_list)

    def do_focus_out(self):
        """Called when the engine loses focus"""
        self.logger.info("Engine focused out")

    def do_reset(self):
        """Called when the engine needs to be reset"""
        self.logger.info("Engine reset")
        self.__preedit_string = ""
        self.__invalidate()

    def do_enable(self):
        """Called when the engine is enabled"""
        self.logger.info("Engine enabled")

    def do_disable(self):
        """Called when the engine is disabled"""
        self.logger.info("Engine disabled")

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
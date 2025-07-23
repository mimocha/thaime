import logging

import gi
gi.require_version('IBus', '1.0')

from gi.repository import GLib, IBus
from thai_keymap import KEDMANEE_KEYMAP


class Engine(IBus.Engine):

    ## ====================================================================== ##
    ## INIT
    ## ====================================================================== ##

    def __init__(self, bus, object_path):
        super(Engine, self).__init__(
            connection=bus.get_connection(),
            object_path=object_path
        )
        self.__bus = bus

        self.logger = logging.getLogger('thaime.engine')
        self.logger.info(f"Engine created with path: {object_path}")
        
        # Initialize engine state
        self.__preedit_string = ""
        self.__lookup_table = IBus.LookupTable()
        self.__is_invalidate = False

        # Define input modes
        self.MODE_LATIN = 0
        self.MODE_KEDMANEE = 1
        self.MODE_PHONETIC = 2
        self.current_mode = self.MODE_LATIN

        self.setup_properties()
        return

    def setup_properties(self):
        # Standard Latin ANSI-QWERTY keymap
        self.mode_latin = IBus.Property(
            key="mode.latin",
            prop_type=IBus.PropType.RADIO,
            label=IBus.Text.new_from_string("Latin"),
            tooltip=IBus.Text.new_from_string("Latin Keyboard Layout"),
            sensitive=True,
            visible=True,
            state=IBus.PropState.CHECKED, # Default Selected
            symbol=IBus.Text.new_from_string('L')
        )

        # Standard Thai Kedmanee keymap
        self.mode_kedmanee = IBus.Property(
            key="mode.kedmanee",
            prop_type=IBus.PropType.RADIO,
            label=IBus.Text.new_from_string("Kedmanee"),
            tooltip=IBus.Text.new_from_string("Kedmanee Keyboard Layout"),
            sensitive=True,
            visible=True,
            state=IBus.PropState.UNCHECKED,
            symbol=IBus.Text.new_from_string('K')
        )

        # Core Latin-to-Thai text conversion mode
        self.mode_phonetic = IBus.Property(
            key="mode.phonetic",
            prop_type=IBus.PropType.RADIO,
            label=IBus.Text.new_from_string("Phonetic"),
            tooltip=IBus.Text.new_from_string("Thaime Phonetic Conversion Mode"),
            sensitive=True,
            visible=True,
            state=IBus.PropState.UNCHECKED,
            icon="input-keyboard",
            symbol=IBus.Text.new_from_string('P')
        )

        # Create property list for the engine
        # Use this to toggle engine mode
        self.props_list = IBus.PropList()
        self.props_list.append(self.mode_latin)
        self.props_list.append(self.mode_kedmanee)
        self.props_list.append(self.mode_phonetic)

        # Register properties with IBus
        self.register_properties(self.props_list)

    ## ====================================================================== ##
    ## HANDLING STATES
    ## ====================================================================== ##

    def do_property_activate(self, prop_name, state):
        """Called when user clicks on a property"""
        if prop_name == 'mode.latin':
            self.set_mode(self.MODE_LATIN)
        elif prop_name == 'mode.kedmanee':
            self.set_mode(self.MODE_KEDMANEE)
        elif prop_name == 'mode.phonetic':
            self.set_mode(self.MODE_PHONETIC)

    def set_mode(self, new_mode):
        """Switch to a new input mode and update UI"""
        if self.current_mode == new_mode:
            return
        
        self.logger.debug(f"Switching input mode from {self.current_mode} to {new_mode}")
        self.current_mode = new_mode
        
        # Update radio button states
        self.mode_latin.set_state(
            IBus.PropState.CHECKED if new_mode == self.MODE_LATIN else IBus.PropState.UNCHECKED
        )
        self.mode_kedmanee.set_state(
            IBus.PropState.CHECKED if new_mode == self.MODE_KEDMANEE else IBus.PropState.UNCHECKED
        )
        self.mode_phonetic.set_state(
            IBus.PropState.CHECKED if new_mode == self.MODE_PHONETIC else IBus.PropState.UNCHECKED
        )
        
        # Notify IBus of property changes
        self.update_property(self.mode_latin)
        self.update_property(self.mode_kedmanee)
        self.update_property(self.mode_phonetic)
        
        # Reset any current composition state when switching modes
        self.do_reset()

    ## ====================================================================== ##
    ## HANDLING INPUT
    ## ====================================================================== ##

    def do_process_key_event(self, keyval, keycode, state):
        """Process key events based on current mode"""
        # Ignore key release events
        is_press = ((state & IBus.ModifierType.RELEASE_MASK) == 0)
        if not is_press:
            return False
        
        # Log the keystroke
        key_char = chr(keyval) if 32 <= keyval <= 126 else f"<{keyval}>"
        self.logger.info(f"Key pressed: keyval={keyval} (0x{keyval:x}) '{key_char}', keycode={keycode}, state={state} (0x{state:x})")
        
        # TODO: Handle mode switching shortcuts (optional)
        if state & IBus.ModifierType.CONTROL_MASK:
            if keyval == IBus.KEY_1:
                self.set_mode(self.MODE_LATIN)
                return True
            elif keyval == IBus.KEY_2:
                self.set_mode(self.MODE_KEDMANEE)
                return True
            elif keyval == IBus.KEY_3:
                self.set_mode(self.MODE_PHONETIC)
                return True
        
        # Route to mode-specific processing
        if self.current_mode == self.MODE_LATIN:
            return self.process_latin_input(keyval, keycode, state)
        elif self.current_mode == self.MODE_KEDMANEE:
            return self.process_kedmanee_input(keyval, keycode, state)
        elif self.current_mode == self.MODE_PHONETIC:
            return self.process_phonetic_input(keyval, keycode, state)
        
        # This should never happen
        self.logger.error(f"do_process_key_event: Unexpected IME mode - {self.current_mode}")
        raise RuntimeError(f"do_process_key_event: Unexpected IME mode - {self.current_mode}")
    
    def process_latin_input(self, keyval, keycode, state):
        """Latin ANSI-QWERTY keyboard layout mode"""
        # Do nothing, let system handle key
        return False
    
    def process_kedmanee_input(self, keyval, keycode, state):
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

    def process_phonetic_input(self, keyval, keycode, state):
        """Handle phonetic input mode"""
        # TODO: Implement this
        return False

    ## ====================================================================== ##
    ## BEHAVIORS
    ## ====================================================================== ##

    def do_focus_in(self):
        """Called when the engine gains focus"""
        self.logger.debug("Engine focused in")
        self.register_properties(self.props_list)

    def do_focus_out(self):
        """Called when the engine loses focus"""
        self.logger.debug("Engine focused out")

    def do_reset(self):
        """Called when the engine needs to be reset"""
        self.logger.debug("Engine reset")
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
import logging
import json
import os

import gi
gi.require_version('IBus', '1.0')

from gi.repository import GLib, IBus
from thai_keymap import KEDMANEE_KEYMAP


class Engine(IBus.Engine):
    """Input Method Engine core class"""

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
        self.__lookup_table = IBus.LookupTable.new(5, 0, True, True)
        self.__is_invalidate = False
        self.__trie_data = self.load_trie_data()

        # Define input modes
        self.MODE_LATIN = 0
        self.MODE_KEDMANEE = 1
        self.MODE_PHONETIC = 2
        self.current_mode = self.MODE_LATIN

        self.init_properties()

    def init_properties(self):
        """Initialize IME properties menu"""
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
        # Skip if modifier keys are pressed
        if state & (IBus.ModifierType.CONTROL_MASK | IBus.ModifierType.MOD1_MASK):
            return False

        key_char = chr(keyval)

        if keyval == IBus.KEY_BackSpace:
            if self.__preedit_string:
                self.__preedit_string = self.__preedit_string[:-1]
                self.update_preedit_and_lookup()
                return True
            return False

        if keyval == IBus.KEY_Escape:
            if self.__preedit_string:
                self.do_reset()
                return True
            return False
        
        if keyval in (IBus.KEY_Return, IBus.KEY_KP_Enter, IBus.KEY_space):
            if self.__preedit_string:
                if self.__lookup_table.get_number_of_candidates() > 0:
                    candidate = self.__lookup_table.get_candidate(self.__lookup_table.get_cursor_pos())
                    if candidate:
                        self.commit_candidate(candidate)
                else:
                    self.commit_text(IBus.Text.new_from_string(self.__preedit_string))
                
                self.do_reset()
                
                if keyval == IBus.KEY_space:
                    self.commit_text(IBus.Text.new_from_string(" "))
                
                return True
            
            if keyval == IBus.KEY_space:
                return False
            
            return False

        if keyval == IBus.KEY_Up:
            if self.__lookup_table.get_number_of_candidates() > 0:
                self.__lookup_table.cursor_up()
                self.update_lookup_table(self.__lookup_table, True)
                return True
            return False

        if keyval == IBus.KEY_Down:
            if self.__lookup_table.get_number_of_candidates() > 0:
                self.__lookup_table.cursor_down()
                self.update_lookup_table(self.__lookup_table, True)
                return True
            return False

        if IBus.KEY_1 <= keyval <= IBus.KEY_5:
            if self.__preedit_string:
                candidates = self.lookup_candidates(self.__preedit_string)
                index = keyval - IBus.KEY_1
                if index < len(candidates):
                    self.commit_candidate(candidates[index])
                    self.do_reset()
                    return True

        if 'a' <= key_char.lower() <= 'z':
            self.__preedit_string += key_char.lower()
            self.update_preedit_and_lookup()
            return True

        return False

    def load_trie_data(self):
        script_dir = os.path.dirname(os.path.abspath(__file__))
        trie_path = os.path.join(script_dir, 'trie.json')
        try:
            with open(trie_path, 'r', encoding='utf-8') as f:
                return json.load(f)
        except FileNotFoundError:
            self.logger.error(f"Trie data file not found at {trie_path}")
            return {}
        except json.JSONDecodeError:
            self.logger.error(f"Error decoding Trie data from {trie_path}")
            return {}

    def lookup_candidates(self, prefix):
        if not prefix:
            return []
        
        candidates = self.__trie_data.get(prefix, [])
        # Sort by frequency (descending)
        sorted_candidates = sorted(candidates, key=lambda item: item[1], reverse=True)
        # Return only the Thai word
        return [IBus.Text.new_from_string(c[0]) for c in sorted_candidates]

    def update_preedit_and_lookup(self):
        if not self.__preedit_string:
            self.hide_preedit_and_lookup()
            return

        # Update pre-edit text
        preedit_text = IBus.Text.new_from_string(self.__preedit_string)
        self.update_preedit_text(preedit_text, len(self.__preedit_string), True)
        
        # Update lookup table
        candidates = self.lookup_candidates(self.__preedit_string)
        self.__lookup_table.clear()
        if candidates:
            self.__lookup_table.set_orientation(IBus.Orientation.VERTICAL)
            for candidate in candidates[:5]:
                self.__lookup_table.append_candidate(candidate)
            
            if self.__lookup_table.get_number_of_candidates() > 0:
                self.update_lookup_table(self.__lookup_table, True)
            else:
                self.hide_lookup_table()
        else:
            self.hide_lookup_table()

    def hide_preedit_and_lookup(self):
        self.hide_preedit_text()
        self.hide_lookup_table()

    def commit_candidate(self, candidate):
        self.commit_text(candidate)

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
        self.hide_preedit_and_lookup()

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
        if self.__is_invalidate:
            return
        self.__is_invalidate = True
        GLib.idle_add(self.__update, priority=GLib.PRIORITY_LOW)

    def __update(self):
        """Update the engine state"""
        self.logger.debug("Updating engine state")
        self.__is_invalidate = False
// SPDX-License-Identifier: MPL-2.0

//! THAIME Engine - Core library for the Thai Input Method Engine.
//!
//! This crate provides:
//! - A Rust API for use by other workspace crates (e.g., thaime_cli)
//! - A C ABI for use by framework frontends (IBus, Fcitx5, etc.)

pub mod config;
pub mod context;
pub mod keymap;
pub mod ngram;
pub mod ranking;
pub mod trie;
pub mod validate;

use context::InputContext;
use ngram::NgramData;
use ranking::{Candidate, LatticeEdge};
use trie::Dictionary;

/// Input mode determines how keystrokes are processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum InputMode {
    /// Latin romanization → Thai candidates via dictionary/lattice/Viterbi.
    #[default]
    Romanization = 0,
    /// Standard Thai keyboard layout (TIS 820-2538). Direct 1:1 key→Thai mapping.
    Kedmanee = 1,
    /// Pass-through Latin. Characters output as-is.
    Latin = 2,
}

/// Result of processing a key through the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyResult {
    /// Key was accepted into the composition buffer (Romanization mode).
    /// Caller should query `preedit()` and `candidates()` for updated state.
    Consumed,
    /// Key immediately produced an output character (Kedmanee/Latin mode).
    Committed(char),
    /// Key was not handled by the engine.
    Rejected,
}

/// The top-level engine handle.
///
/// Wraps an [`InputContext`] and provides the public Rust API.
/// Frontends using the C ABI receive an opaque pointer to this struct.
#[derive(Default)]
pub struct ThaiMeEngine {
    context: Option<InputContext>,
    mode: InputMode,
}

impl ThaiMeEngine {
    /// Create a new engine with the embedded dictionary.
    ///
    /// If the `embed-ngram` feature is also enabled, loads the embedded
    /// n-gram data for context-dependent ranking.
    ///
    /// Panics if the embedded data is corrupt.
    #[cfg(feature = "embed-dict")]
    pub fn new() -> Self {
        let dict = Dictionary::from_embedded();
        #[cfg(feature = "embed-ngram")]
        let context = {
            let ngram = NgramData::from_embedded();
            InputContext::with_ngram(dict, ngram)
        };
        #[cfg(not(feature = "embed-ngram"))]
        let context = InputContext::new(dict);
        Self {
            context: Some(context),
            mode: InputMode::default(),
        }
    }

    /// Create a new engine from pre-built dictionary bytes.
    ///
    /// Useful for testing or when the dictionary is loaded at runtime.
    pub fn from_dict_bytes(trie_bytes: Vec<u8>, metadata_bytes: &[u8]) -> Self {
        let dict = Dictionary::from_bytes(trie_bytes, metadata_bytes);
        Self {
            context: Some(InputContext::new(dict)),
            mode: InputMode::default(),
        }
    }

    /// Create a new engine from pre-built dictionary bytes and n-gram data.
    pub fn from_dict_bytes_with_ngram(
        trie_bytes: Vec<u8>,
        metadata_bytes: &[u8],
        ngram: NgramData,
    ) -> Self {
        let dict = Dictionary::from_bytes(trie_bytes, metadata_bytes);
        Self {
            context: Some(InputContext::with_ngram(dict, ngram)),
            mode: InputMode::default(),
        }
    }

    /// Create a new engine from dictionary bytes and a raw n-gram binary blob.
    ///
    /// Parses the n-gram binary at construction time. Returns an error
    /// string if the ngram binary is malformed.
    pub fn from_dict_bytes_with_ngram_binary(
        trie_bytes: Vec<u8>,
        metadata_bytes: &[u8],
        ngram_bytes: &[u8],
    ) -> Result<Self, String> {
        let dict = Dictionary::from_bytes(trie_bytes, metadata_bytes);
        let ngram =
            NgramData::from_bytes(ngram_bytes).map_err(|e| format!("ngram parse error: {e}"))?;
        Ok(Self {
            context: Some(InputContext::with_ngram(dict, ngram)),
            mode: InputMode::default(),
        })
    }

    /// Hot-load n-gram data after engine construction.
    ///
    /// Replaces any previously loaded n-gram data and re-ranks candidates
    /// if the buffer is non-empty.
    pub fn load_ngram(&mut self, ngram: NgramData) {
        if let Some(ctx) = &mut self.context {
            ctx.load_ngram(ngram);
        }
    }

    /// Append a Latin character to the input buffer.
    ///
    /// Returns `true` if the key was consumed by the engine.
    pub fn push_key(&mut self, ch: char) -> bool {
        match &mut self.context {
            Some(ctx) => ctx.push_key(ch),
            None => false,
        }
    }

    /// Remove the last character from the input buffer (backspace).
    ///
    /// Returns `true` if there was a character to remove.
    pub fn pop_key(&mut self) -> bool {
        match &mut self.context {
            Some(ctx) => ctx.pop_key(),
            None => false,
        }
    }

    /// Get the current ranked candidate list.
    pub fn candidates(&self) -> &[Candidate] {
        match &self.context {
            Some(ctx) => ctx.candidates(),
            None => &[],
        }
    }

    /// Get the lattice edges from the most recent ranking.
    pub fn lattice_edges(&self) -> &[LatticeEdge] {
        match &self.context {
            Some(ctx) => ctx.lattice_edges(),
            None => &[],
        }
    }

    /// Commit the candidate at the given index.
    ///
    /// Returns the Thai text if successful, or `None` if the index is
    /// out of bounds. Clears the input buffer on success.
    pub fn commit(&mut self, index: usize) -> Option<String> {
        match &mut self.context {
            Some(ctx) => ctx.commit(index),
            None => None,
        }
    }

    /// Reset the engine state, clearing the input buffer and candidates.
    pub fn reset(&mut self) {
        if let Some(ctx) = &mut self.context {
            ctx.reset();
        }
    }

    /// Clear the committed context history (e.g., on focus change).
    ///
    /// Does not affect the current input buffer.
    pub fn clear_context(&mut self) {
        if let Some(ctx) = &mut self.context {
            ctx.clear_context();
        }
    }

    /// Get the committed context history.
    pub fn committed_context(&self) -> &[String] {
        match &self.context {
            Some(ctx) => ctx.committed_context(),
            None => &[],
        }
    }

    /// Get the current Latin input buffer (for preedit display).
    pub fn preedit(&self) -> &str {
        match &self.context {
            Some(ctx) => ctx.preedit(),
            None => "",
        }
    }

    /// Get the current input mode.
    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// Set the input mode. Resets any active composition when the mode changes.
    pub fn set_mode(&mut self, mode: InputMode) {
        if self.mode != mode {
            self.reset();
            self.mode = mode;
        }
    }

    /// Process a key with mode-aware behavior.
    ///
    /// - **Romanization**: delegates to [`push_key`](Self::push_key), returns `Consumed` or `Rejected`.
    /// - **Kedmanee**: maps the key via the Kedmanee layout, returns `Committed(thai_char)` or `Rejected`.
    /// - **Latin**: passes printable ASCII through, returns `Committed(ch)` or `Rejected`.
    pub fn process_key(&mut self, ch: char) -> KeyResult {
        match self.mode {
            InputMode::Romanization => {
                if self.push_key(ch) {
                    KeyResult::Consumed
                } else {
                    KeyResult::Rejected
                }
            }
            InputMode::Kedmanee => match keymap::kedmanee_map(ch) {
                Some(thai_ch) => KeyResult::Committed(thai_ch),
                None => KeyResult::Rejected,
            },
            InputMode::Latin => {
                if ch.is_ascii_graphic() || ch == ' ' {
                    KeyResult::Committed(ch)
                } else {
                    KeyResult::Rejected
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C ABI exports
// ---------------------------------------------------------------------------

/// Create a new engine instance with the embedded dictionary.
/// Returns a pointer that must be freed with `thaime_engine_free()`.
#[cfg(feature = "embed-dict")]
#[no_mangle]
pub extern "C" fn thaime_engine_new() -> *mut ThaiMeEngine {
    let engine = Box::new(ThaiMeEngine::new());
    Box::into_raw(engine)
}

/// Free an engine instance.
///
/// # Safety
/// `engine` must be a pointer returned by `thaime_engine_new()` and must
/// not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn thaime_engine_free(engine: *mut ThaiMeEngine) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
    }
}

/// Process a key press. Returns true if the engine consumed the key.
///
/// Currently accepts ASCII alphabetic characters (a-z, A-Z) as input
/// to the romanization buffer.
///
/// # Safety
/// `engine` must be a valid pointer from `thaime_engine_new()`.
#[no_mangle]
pub unsafe extern "C" fn thaime_process_key(
    engine: *mut ThaiMeEngine,
    keyval: u32,
    _keycode: u32,
    _modifiers: u32,
) -> bool {
    let engine = &mut *engine;
    // Interpret keyval as a Unicode code point
    if let Some(ch) = char::from_u32(keyval) {
        engine.push_key(ch)
    } else {
        false
    }
}

/// Reset the engine state, clearing any in-progress input.
///
/// # Safety
/// `engine` must be a valid pointer from `thaime_engine_new()`.
#[no_mangle]
pub unsafe extern "C" fn thaime_reset(engine: *mut ThaiMeEngine) {
    if !engine.is_null() {
        let engine = &mut *engine;
        engine.reset();
    }
}

/// Clear the committed context history.
///
/// Call this when the text field loses focus, the cursor jumps, or the
/// application switches. Does not affect the current input buffer.
///
/// # Safety
/// `engine` must be a valid pointer from `thaime_engine_new()`.
#[no_mangle]
pub unsafe extern "C" fn thaime_clear_context(engine: *mut ThaiMeEngine) {
    if !engine.is_null() {
        let engine = &mut *engine;
        engine.clear_context();
    }
}

/// Get the current input mode.
///
/// Returns: 0 = Romanization, 1 = Kedmanee, 2 = Latin.
///
/// # Safety
/// `engine` must be a valid pointer from `thaime_engine_new()`.
#[no_mangle]
pub unsafe extern "C" fn thaime_get_mode(engine: *const ThaiMeEngine) -> u8 {
    if engine.is_null() {
        return 0;
    }
    (*engine).mode() as u8
}

/// Set the input mode. Resets any active composition when the mode changes.
///
/// `mode`: 0 = Romanization, 1 = Kedmanee, 2 = Latin. Invalid values are ignored.
///
/// # Safety
/// `engine` must be a valid pointer from `thaime_engine_new()`.
#[no_mangle]
pub unsafe extern "C" fn thaime_set_mode(engine: *mut ThaiMeEngine, mode: u8) {
    if engine.is_null() {
        return;
    }
    let engine = &mut *engine;
    if let Some(m) = match mode {
        0 => Some(InputMode::Romanization),
        1 => Some(InputMode::Kedmanee),
        2 => Some(InputMode::Latin),
        _ => None,
    } {
        engine.set_mode(m);
    }
}

/// Process a key press with mode-aware behavior.
///
/// Returns:
/// - `0` — key rejected (not handled)
/// - `1` — key consumed into composition buffer (Romanization mode)
/// - `2+` — committed character as a Unicode code point (Kedmanee/Latin mode)
///
/// # Safety
/// `engine` must be a valid pointer from `thaime_engine_new()`.
#[no_mangle]
pub unsafe extern "C" fn thaime_process_key_ex(
    engine: *mut ThaiMeEngine,
    keyval: u32,
    _keycode: u32,
    _modifiers: u32,
) -> u32 {
    if engine.is_null() {
        return 0;
    }
    let engine = &mut *engine;
    if let Some(ch) = char::from_u32(keyval) {
        match engine.process_key(ch) {
            KeyResult::Rejected => 0,
            KeyResult::Consumed => 1,
            KeyResult::Committed(c) => c as u32,
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::tests::build_test_dict;

    fn make_engine() -> ThaiMeEngine {
        let dict = build_test_dict();
        ThaiMeEngine {
            context: Some(InputContext::new(dict)),
            mode: InputMode::default(),
        }
    }

    #[test]
    fn test_push_and_candidates() {
        let mut engine = make_engine();
        assert!(engine.push_key('m'));
        assert!(engine.push_key('a'));
        assert!(engine.push_key('i'));

        assert_eq!(engine.preedit(), "mai");
        let candidates = engine.candidates();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].thai, "ไม่");
        assert_eq!(candidates[0].words[0].thai, "ไม่");
    }

    #[test]
    fn test_commit_and_reset() {
        let mut engine = make_engine();
        for ch in "mai".chars() {
            engine.push_key(ch);
        }

        let result = engine.commit(0);
        assert_eq!(result, Some("ไม่".to_string()));
        assert!(engine.preedit().is_empty());
        assert!(engine.candidates().is_empty());
    }

    #[test]
    fn test_pop_key() {
        let mut engine = make_engine();
        for ch in "mai".chars() {
            engine.push_key(ch);
        }
        engine.pop_key();
        assert_eq!(engine.preedit(), "ma");
    }

    #[test]
    fn test_reset() {
        let mut engine = make_engine();
        for ch in "mai".chars() {
            engine.push_key(ch);
        }
        engine.reset();
        assert!(engine.preedit().is_empty());
        assert!(engine.candidates().is_empty());
    }

    #[test]
    fn test_default_engine_has_no_context() {
        let engine = ThaiMeEngine::default();
        assert!(engine.preedit().is_empty());
        assert!(engine.candidates().is_empty());
    }

    #[test]
    fn test_c_abi_lifecycle() {
        // Test that C ABI functions work with a manually constructed engine
        let engine = Box::into_raw(Box::new(make_engine()));
        assert!(!engine.is_null());
        unsafe {
            assert!(thaime_process_key(engine, b'm' as u32, 0, 0));
            assert!(thaime_process_key(engine, b'a' as u32, 0, 0));
            assert!(thaime_process_key(engine, b'i' as u32, 0, 0));
            thaime_clear_context(engine);
            thaime_reset(engine);
            thaime_engine_free(engine);
        }
    }

    // ── Mode switching tests ────────────────────────────────────────

    #[test]
    fn test_default_mode_is_romanization() {
        let engine = make_engine();
        assert_eq!(engine.mode(), InputMode::Romanization);
    }

    #[test]
    fn test_set_mode() {
        let mut engine = make_engine();
        engine.set_mode(InputMode::Kedmanee);
        assert_eq!(engine.mode(), InputMode::Kedmanee);
        engine.set_mode(InputMode::Latin);
        assert_eq!(engine.mode(), InputMode::Latin);
        engine.set_mode(InputMode::Romanization);
        assert_eq!(engine.mode(), InputMode::Romanization);
    }

    #[test]
    fn test_set_mode_resets_buffer() {
        let mut engine = make_engine();
        engine.push_key('m');
        engine.push_key('a');
        assert_eq!(engine.preedit(), "ma");

        engine.set_mode(InputMode::Kedmanee);
        assert!(engine.preedit().is_empty());
        assert!(engine.candidates().is_empty());
    }

    #[test]
    fn test_set_mode_same_mode_no_reset() {
        let mut engine = make_engine();
        engine.push_key('m');
        engine.push_key('a');

        // Setting the same mode should NOT reset
        engine.set_mode(InputMode::Romanization);
        assert_eq!(engine.preedit(), "ma");
    }

    #[test]
    fn test_process_key_romanization() {
        let mut engine = make_engine();
        assert_eq!(engine.process_key('m'), KeyResult::Consumed);
        assert_eq!(engine.preedit(), "m");
        assert_eq!(engine.process_key('1'), KeyResult::Rejected);
    }

    #[test]
    fn test_process_key_kedmanee() {
        let mut engine = make_engine();
        engine.set_mode(InputMode::Kedmanee);

        assert_eq!(engine.process_key('a'), KeyResult::Committed('ฟ'));
        assert_eq!(engine.process_key('A'), KeyResult::Committed('ฤ'));
        assert_eq!(engine.process_key('1'), KeyResult::Committed('ๅ'));
        // Preedit should remain empty — Kedmanee produces immediate output
        assert!(engine.preedit().is_empty());
    }

    #[test]
    fn test_process_key_latin() {
        let mut engine = make_engine();
        engine.set_mode(InputMode::Latin);

        assert_eq!(engine.process_key('a'), KeyResult::Committed('a'));
        assert_eq!(engine.process_key('Z'), KeyResult::Committed('Z'));
        assert_eq!(engine.process_key('5'), KeyResult::Committed('5'));
        assert_eq!(engine.process_key(' '), KeyResult::Committed(' '));
        assert_eq!(engine.process_key('!'), KeyResult::Committed('!'));
        // Non-printable rejected
        assert_eq!(engine.process_key('\t'), KeyResult::Rejected);
        assert_eq!(engine.process_key('\n'), KeyResult::Rejected);
    }

    #[test]
    fn test_c_abi_mode_functions() {
        let engine = Box::into_raw(Box::new(make_engine()));
        unsafe {
            assert_eq!(thaime_get_mode(engine), 0); // Romanization

            thaime_set_mode(engine, 1); // Kedmanee
            assert_eq!(thaime_get_mode(engine), 1);

            // Process key in Kedmanee mode
            let result = thaime_process_key_ex(engine, b'a' as u32, 0, 0);
            assert_eq!(result, 'ฟ' as u32);

            thaime_set_mode(engine, 2); // Latin
            assert_eq!(thaime_get_mode(engine), 2);
            let result = thaime_process_key_ex(engine, b'a' as u32, 0, 0);
            assert_eq!(result, 'a' as u32);

            // Invalid mode ignored
            thaime_set_mode(engine, 99);
            assert_eq!(thaime_get_mode(engine), 2); // Still Latin

            thaime_engine_free(engine);
        }
    }
}

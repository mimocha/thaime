// SPDX-License-Identifier: MPL-2.0

//! THAIME Engine - Core library for the Thai Input Method Engine.
//!
//! This crate provides:
//! - A Rust API for use by other workspace crates (e.g., thaime_cli)
//! - A C ABI for use by framework frontends (IBus, Fcitx5, etc.)

pub mod context;
pub mod keymap;
pub mod ranking;
pub mod trie;
pub mod validate;

use context::InputContext;
use ranking::{Candidate, LatticeEdge};
use trie::Dictionary;

/// The top-level engine handle.
///
/// Wraps an [`InputContext`] and provides the public Rust API.
/// Frontends using the C ABI receive an opaque pointer to this struct.
#[derive(Default)]
pub struct ThaiMeEngine {
    context: Option<InputContext>,
}

impl ThaiMeEngine {
    /// Create a new engine with the embedded dictionary.
    ///
    /// Panics if the embedded dictionary data is corrupt.
    #[cfg(feature = "embed-dict")]
    pub fn new() -> Self {
        let dict = Dictionary::from_embedded();
        Self {
            context: Some(InputContext::new(dict)),
        }
    }

    /// Create a new engine from pre-built dictionary bytes.
    ///
    /// Useful for testing or when the dictionary is loaded at runtime.
    pub fn from_dict_bytes(trie_bytes: Vec<u8>, metadata_bytes: &[u8]) -> Self {
        let dict = Dictionary::from_bytes(trie_bytes, metadata_bytes);
        Self {
            context: Some(InputContext::new(dict)),
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

    /// Get the current Latin input buffer (for preedit display).
    pub fn preedit(&self) -> &str {
        match &self.context {
            Some(ctx) => ctx.preedit(),
            None => "",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::tests::build_test_dict;

    fn make_engine() -> ThaiMeEngine {
        let dict = build_test_dict();
        ThaiMeEngine {
            context: Some(InputContext::new(dict)),
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
            thaime_reset(engine);
            thaime_engine_free(engine);
        }
    }
}

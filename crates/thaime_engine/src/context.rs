// SPDX-License-Identifier: MPL-2.0

//! Input context state machine.
//!
//! Manages the state for a single input session: accumulating keystrokes,
//! querying the trie for prefix matches, and maintaining the candidate list.
//!
//! On each keystroke (`push_key` / `pop_key`), the full ranking pipeline
//! is re-run on the current buffer contents. This is simple and correct
//! for the MVP; incremental updates can be added later if profiling shows
//! a need.

use crate::ranking::{self, Candidate, DEFAULT_K};
use crate::trie::Dictionary;

/// Maximum input buffer length (bytes). Safety valve against huge lattices.
const MAX_BUFFER_LEN: usize = 50;

/// Stateful input session context.
///
/// Created once per input session (e.g., per text field focus). Holds a
/// reference to the shared dictionary and maintains the Latin input buffer
/// and current candidate list.
pub struct InputContext {
    buffer: String,
    dictionary: Dictionary,
    candidates: Vec<Candidate>,
}

impl InputContext {
    /// Create a new input context with the given dictionary.
    pub fn new(dictionary: Dictionary) -> Self {
        Self {
            buffer: String::new(),
            dictionary,
            candidates: Vec::new(),
        }
    }

    /// Append a character to the input buffer and refresh candidates.
    ///
    /// Non-ASCII and non-alphabetic characters are ignored.
    /// Returns `true` if the character was accepted.
    pub fn push_key(&mut self, ch: char) -> bool {
        if !ch.is_ascii_alphabetic() {
            return false;
        }
        if self.buffer.len() >= MAX_BUFFER_LEN {
            return false;
        }
        self.buffer.push(ch.to_ascii_lowercase());
        self.refresh_candidates();
        true
    }

    /// Remove the last character from the buffer and refresh candidates.
    ///
    /// Returns `true` if there was a character to remove.
    pub fn pop_key(&mut self) -> bool {
        if self.buffer.pop().is_none() {
            return false;
        }
        self.refresh_candidates();
        true
    }

    /// Get the current ranked candidate list.
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// Commit the candidate at the given index.
    ///
    /// Returns the Thai text of the committed candidate, or `None` if the
    /// index is out of bounds. Clears the input buffer and candidate list.
    pub fn commit(&mut self, index: usize) -> Option<String> {
        let thai = self.candidates.get(index).map(|c| c.thai.clone());
        if thai.is_some() {
            self.buffer.clear();
            self.candidates.clear();
        }
        thai
    }

    /// Clear the input buffer and candidates without committing.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.candidates.clear();
    }

    /// Get the current Latin input buffer (for preedit display).
    pub fn preedit(&self) -> &str {
        &self.buffer
    }

    /// Re-run the ranking pipeline on the current buffer.
    fn refresh_candidates(&mut self) {
        if self.buffer.is_empty() {
            self.candidates.clear();
        } else {
            self.candidates = ranking::rank_candidates(&self.buffer, &self.dictionary, DEFAULT_K);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::tests::build_test_dict;

    fn make_context() -> InputContext {
        InputContext::new(build_test_dict())
    }

    #[test]
    fn test_push_key_builds_buffer() {
        let mut ctx = make_context();
        assert!(ctx.push_key('m'));
        assert_eq!(ctx.preedit(), "m");
        assert!(ctx.push_key('a'));
        assert_eq!(ctx.preedit(), "ma");
        assert!(ctx.push_key('i'));
        assert_eq!(ctx.preedit(), "mai");
    }

    #[test]
    fn test_push_key_lowercases() {
        let mut ctx = make_context();
        ctx.push_key('M');
        ctx.push_key('A');
        ctx.push_key('I');
        assert_eq!(ctx.preedit(), "mai");
    }

    #[test]
    fn test_push_key_rejects_non_alpha() {
        let mut ctx = make_context();
        assert!(!ctx.push_key('1'));
        assert!(!ctx.push_key(' '));
        assert!(!ctx.push_key('.'));
        assert!(ctx.preedit().is_empty());
    }

    #[test]
    fn test_candidates_update_on_push() {
        let mut ctx = make_context();
        ctx.push_key('m');
        ctx.push_key('a');
        ctx.push_key('i');

        let candidates = ctx.candidates();
        assert_eq!(candidates.len(), 3); // ไม่, ไหม, ใหม่
        assert_eq!(candidates[0].thai, "ไม่");
    }

    #[test]
    fn test_pop_key() {
        let mut ctx = make_context();
        ctx.push_key('m');
        ctx.push_key('a');
        ctx.push_key('i');

        assert!(ctx.pop_key());
        assert_eq!(ctx.preedit(), "ma");

        // "ma" matches word 5 (มา) as a complete tiling
        let candidates = ctx.candidates();
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].thai, "มา");
    }

    #[test]
    fn test_pop_key_empty_buffer() {
        let mut ctx = make_context();
        assert!(!ctx.pop_key());
    }

    #[test]
    fn test_commit() {
        let mut ctx = make_context();
        ctx.push_key('m');
        ctx.push_key('a');
        ctx.push_key('i');

        let result = ctx.commit(0);
        assert_eq!(result, Some("ไม่".to_string()));
        assert!(ctx.preedit().is_empty());
        assert!(ctx.candidates().is_empty());
    }

    #[test]
    fn test_commit_out_of_bounds() {
        let mut ctx = make_context();
        ctx.push_key('m');
        ctx.push_key('a');
        ctx.push_key('i');

        assert!(ctx.commit(99).is_none());
        // Buffer should NOT be cleared on failed commit
        assert_eq!(ctx.preedit(), "mai");
    }

    #[test]
    fn test_reset() {
        let mut ctx = make_context();
        ctx.push_key('m');
        ctx.push_key('a');
        ctx.push_key('i');

        ctx.reset();
        assert!(ctx.preedit().is_empty());
        assert!(ctx.candidates().is_empty());
    }

    #[test]
    fn test_max_buffer_length() {
        let mut ctx = make_context();
        for _ in 0..MAX_BUFFER_LEN {
            assert!(ctx.push_key('a'));
        }
        // Should reject the next character
        assert!(!ctx.push_key('a'));
        assert_eq!(ctx.preedit().len(), MAX_BUFFER_LEN);
    }

    #[test]
    fn test_multi_word_input() {
        let mut ctx = make_context();
        for ch in "mainai".chars() {
            ctx.push_key(ch);
        }

        let candidates = ctx.candidates();
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].thai, "ไม่ใน");
        assert_eq!(candidates[0].words, vec!["ไม่", "ใน"]);
    }
}

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

use std::collections::HashMap;

use crate::config::{MAX_BUFFER_LEN, MAX_CONTEXT_DEPTH};
use crate::ngram::NgramData;
use crate::ranking::{self, Candidate, LatticeEdge, RankingParams};
use crate::trie::Dictionary;

/// A single-word candidate matching at position 0 of the input buffer.
///
/// Used by the hybrid candidate UX to present first-word alternatives
/// alongside the full-sentence Viterbi result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FirstWordCandidate {
    /// Thai text for this word.
    pub thai: String,
    /// Unigram frequency score (higher = more common).
    pub frequency: f64,
    /// Number of bytes consumed from the Latin input buffer.
    pub end_pos: usize,
}

/// Stateful input session context.
///
/// Created once per input session (e.g., per text field focus). Holds a
/// reference to the shared dictionary and maintains the Latin input buffer
/// and current candidate list.
pub struct InputContext {
    buffer: String,
    dictionary: Dictionary,
    ngram: Option<NgramData>,
    candidates: Vec<Candidate>,
    lattice_edges: Vec<LatticeEdge>,
    committed_context: Vec<String>,
}

impl InputContext {
    /// Create a new input context with the given dictionary.
    pub fn new(dictionary: Dictionary) -> Self {
        Self {
            buffer: String::new(),
            dictionary,
            ngram: None,
            candidates: Vec::new(),
            lattice_edges: Vec::new(),
            committed_context: Vec::new(),
        }
    }

    /// Create a new input context with the given dictionary and n-gram data.
    pub fn with_ngram(dictionary: Dictionary, ngram: NgramData) -> Self {
        Self {
            buffer: String::new(),
            dictionary,
            ngram: Some(ngram),
            candidates: Vec::new(),
            lattice_edges: Vec::new(),
            committed_context: Vec::new(),
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

    /// Get the lattice edges from the most recent ranking.
    pub fn lattice_edges(&self) -> &[LatticeEdge] {
        &self.lattice_edges
    }

    /// Commit the candidate at the given index.
    ///
    /// Returns the Thai text of the committed candidate, or `None` if the
    /// index is out of bounds. Clears the input buffer and candidate list.
    /// Pushes the committed word(s) onto the context history.
    pub fn commit(&mut self, index: usize) -> Option<String> {
        let candidate = self.candidates.get(index).cloned();
        if let Some(ref c) = candidate {
            // Push each word individually for word-level bigram context
            for word in &c.words {
                self.committed_context.push(word.thai.clone());
            }
            // Trim context to max depth
            let len = self.committed_context.len();
            if len > MAX_CONTEXT_DEPTH {
                self.committed_context.drain(..len - MAX_CONTEXT_DEPTH);
            }
            self.buffer.clear();
            self.candidates.clear();
            self.lattice_edges.clear();
        }
        candidate.map(|c| c.thai)
    }

    /// Clear the input buffer and candidates without committing.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.candidates.clear();
        self.lattice_edges.clear();
        self.committed_context.clear();
    }

    /// Clear the committed context history (e.g., on focus change).
    ///
    /// Does not affect the current input buffer or candidates.
    pub fn clear_context(&mut self) {
        self.committed_context.clear();
        // Re-rank with cleared context if there's active input
        if !self.buffer.is_empty() {
            self.refresh_candidates();
        }
    }

    /// Get the committed context history.
    pub fn committed_context(&self) -> &[String] {
        &self.committed_context
    }

    /// Hot-load n-gram data after construction.
    ///
    /// Replaces any previously loaded n-gram data and re-ranks candidates
    /// if the buffer is non-empty.
    pub fn load_ngram(&mut self, ngram: NgramData) {
        self.ngram = Some(ngram);
        if !self.buffer.is_empty() {
            self.refresh_candidates();
        }
    }

    /// Get first-word candidates: distinct Thai words matching at position 0
    /// of the current input buffer, deduplicated by Thai text (best frequency
    /// kept), sorted by frequency descending.
    pub fn first_word_candidates(&self) -> Vec<FirstWordCandidate> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        // Collect lattice edges starting at position 0, deduplicate by Thai text
        let mut best: HashMap<String, FirstWordCandidate> = HashMap::new();
        for edge in &self.lattice_edges {
            if edge.start != 0 {
                continue;
            }
            best.entry(edge.thai.clone())
                .and_modify(|existing| {
                    if edge.frequency > existing.frequency {
                        existing.frequency = edge.frequency;
                        existing.end_pos = edge.end;
                    }
                })
                .or_insert(FirstWordCandidate {
                    thai: edge.thai.clone(),
                    frequency: edge.frequency,
                    end_pos: edge.end,
                });
        }

        let mut result: Vec<FirstWordCandidate> = best.into_values().collect();
        result.sort_by(|a, b| {
            b.frequency
                .partial_cmp(&a.frequency)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    /// Commit a partial word from the front of the buffer.
    ///
    /// Consumes `consume_bytes` from the front of the Latin input buffer,
    /// pushes `thai_word` onto the committed context, and re-ranks the
    /// remaining buffer.
    ///
    /// Returns `true` on success, `false` if `consume_bytes` is 0 or
    /// exceeds the buffer length.
    pub fn commit_partial(&mut self, thai_word: &str, consume_bytes: usize) -> bool {
        if consume_bytes == 0 || consume_bytes > self.buffer.len() {
            return false;
        }

        // Push the Thai word onto committed context
        self.committed_context.push(thai_word.to_string());
        let len = self.committed_context.len();
        if len > MAX_CONTEXT_DEPTH {
            self.committed_context.drain(..len - MAX_CONTEXT_DEPTH);
        }

        // Trim the buffer
        self.buffer = self.buffer[consume_bytes..].to_string();

        // Re-rank the remainder
        self.refresh_candidates();
        true
    }

    /// Get the current Latin input buffer (for preedit display).
    pub fn preedit(&self) -> &str {
        &self.buffer
    }

    /// Re-run the ranking pipeline on the current buffer.
    fn refresh_candidates(&mut self) {
        if self.buffer.is_empty() {
            self.candidates.clear();
            self.lattice_edges.clear();
        } else {
            let result = ranking::rank_candidates(
                &self.buffer,
                &self.dictionary,
                self.ngram.as_ref(),
                &self.committed_context,
                &RankingParams::default(),
            );
            self.candidates = result.candidates;
            self.lattice_edges = result.lattice_edges;
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
        assert_eq!(candidates[0].word_count(), 2);
        assert_eq!(candidates[0].words[0].thai, "ไม่");
        assert_eq!(candidates[0].words[1].thai, "ใน");
    }

    // ── First-word candidates ───────────────────────────────────────

    #[test]
    fn test_first_word_candidates_empty_buffer() {
        let ctx = make_context();
        assert!(ctx.first_word_candidates().is_empty());
    }

    #[test]
    fn test_first_word_candidates_single_word() {
        let mut ctx = make_context();
        for ch in "mai".chars() {
            ctx.push_key(ch);
        }

        let fw = ctx.first_word_candidates();
        // "mai" matches: "ma" → มา, "mai" → ไม่/ไหม/ใหม่ = 4 first-word candidates
        assert_eq!(fw.len(), 4);
        // Should be sorted by frequency descending — ไม่ (0.013) is highest
        assert_eq!(fw[0].thai, "ไม่");
        assert_eq!(fw[0].end_pos, 3);
        // มา has end_pos 2 (consumes "ma")
        let ma = fw.iter().find(|c| c.thai == "มา").unwrap();
        assert_eq!(ma.end_pos, 2);
    }

    #[test]
    fn test_first_word_candidates_multi_word() {
        let mut ctx = make_context();
        for ch in "mainai".chars() {
            ctx.push_key(ch);
        }

        let fw = ctx.first_word_candidates();
        // Should include matches at position 0: "ma" → มา, "mai" → ไม่/ไหม/ใหม่
        assert!(fw.len() >= 2);
        // All should start at position 0
        for c in &fw {
            assert!(c.end_pos > 0);
            assert!(c.end_pos <= 6); // <= length of "mainai"
        }
    }

    // ── Partial commit ──────────────────────────────────────────────

    #[test]
    fn test_commit_partial_trims_buffer() {
        let mut ctx = make_context();
        for ch in "mainai".chars() {
            ctx.push_key(ch);
        }

        // Commit first word "mai" (3 bytes) as ไม่
        assert!(ctx.commit_partial("ไม่", 3));
        assert_eq!(ctx.preedit(), "nai");
        // Context should contain the committed word
        assert_eq!(ctx.committed_context(), &["ไม่"]);
        // Candidates should be refreshed for "nai"
        assert!(!ctx.candidates().is_empty());
    }

    #[test]
    fn test_commit_partial_entire_buffer() {
        let mut ctx = make_context();
        for ch in "mai".chars() {
            ctx.push_key(ch);
        }

        // Commit entire buffer
        assert!(ctx.commit_partial("ไม่", 3));
        assert!(ctx.preedit().is_empty());
        assert!(ctx.candidates().is_empty());
        assert_eq!(ctx.committed_context(), &["ไม่"]);
    }

    #[test]
    fn test_commit_partial_zero_bytes_rejected() {
        let mut ctx = make_context();
        for ch in "mai".chars() {
            ctx.push_key(ch);
        }

        assert!(!ctx.commit_partial("ไม่", 0));
        // Buffer unchanged
        assert_eq!(ctx.preedit(), "mai");
    }

    #[test]
    fn test_commit_partial_exceeds_buffer_rejected() {
        let mut ctx = make_context();
        for ch in "mai".chars() {
            ctx.push_key(ch);
        }

        assert!(!ctx.commit_partial("ไม่", 100));
        assert_eq!(ctx.preedit(), "mai");
    }

    #[test]
    fn test_commit_partial_context_depth() {
        let mut ctx = make_context();

        // Commit three words to test context depth trimming
        for ch in "mainai".chars() {
            ctx.push_key(ch);
        }
        ctx.commit_partial("ไม่", 3);

        for ch in "mainai".chars() {
            ctx.push_key(ch);
        }
        ctx.commit_partial("ใน", 3);

        // MAX_CONTEXT_DEPTH is 2, so only last 2 should remain
        let context = ctx.committed_context();
        assert!(context.len() <= MAX_CONTEXT_DEPTH);
    }
}

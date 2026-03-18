// SPDX-License-Identifier: MPL-2.0

//! N-gram data storage and scoring.
//!
//! Loads pre-scored log₁₀ probabilities from a v1 binary file and
//! performs Stupid Backoff scoring with trigram→bigram→unigram fallback.
//! Binary search on sorted u16 ID arrays for fast lookup.
//!
//! ## Stupid Backoff (trigram)
//!
//! ```text
//! score(w | w_prev2, w_prev1):
//!   if trigram exists:
//!       10^(log₁₀(P(w | w_prev2, w_prev1)))
//!   elif bigram exists:
//!       alpha * 10^(log₁₀(P(w | w_prev1)))
//!   else:
//!       alpha^2 * 10^(log₁₀(P(w)))
//! ```
//!
//! When `w_prev2` is `None`, degrades to bigram. When both are `None`
//! (beginning of sentence), falls back to unigram probability directly.

use std::collections::HashMap;

use crate::config::FLOOR_PROB;

const HEADER_SIZE: usize = 32;
const MAGIC: &[u8; 4] = b"TNLM";

/// Errors that can occur when parsing an n-gram binary file.
#[derive(Debug)]
pub enum NgramError {
    /// Data is smaller than the 32-byte header.
    TooSmall,
    /// Magic bytes are not "TNLM".
    BadMagic,
    /// Format version is not supported by this engine.
    UnsupportedVersion(u16),
    /// String table contains invalid UTF-8.
    InvalidUtf8,
    /// Data ends before all sections are read.
    Truncated,
}

impl std::fmt::Display for NgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall => write!(f, "data too small for header"),
            Self::BadMagic => write!(f, "invalid magic bytes (expected TNLM)"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported format version: {v}"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in string table"),
            Self::Truncated => write!(f, "data truncated"),
        }
    }
}

impl std::error::Error for NgramError {}

/// A bigram entry: (w1, w2) → log₁₀(P(w2|w1)).
#[derive(Debug, Clone)]
struct BigramEntry {
    w1: u16,
    w2: u16,
    score: f32,
}

/// A trigram entry: (w1, w2, w3) → log₁₀(P(w3|w1,w2)).
#[derive(Debug, Clone)]
struct TrigramEntry {
    w1: u16,
    w2: u16,
    w3: u16,
    score: f32,
}

/// N-gram data with pre-scored log₁₀ probabilities.
///
/// Supports binary loading from v1 format and in-memory construction
/// from raw counts (for testing and TSV import).
#[derive(Debug)]
pub struct NgramData {
    word_to_id: HashMap<String, u16>,
    unigram_scores: Vec<f32>,
    bigrams: Vec<BigramEntry>,
    trigrams: Vec<TrigramEntry>,
    alpha: f32,
    min_count: u8,
}

impl NgramData {
    /// Parse n-gram data from a v1 binary blob.
    ///
    /// The binary format is documented in `.docs/ngram-handover-v1.md`.
    pub fn from_bytes(data: &[u8]) -> Result<Self, NgramError> {
        if data.len() < HEADER_SIZE {
            return Err(NgramError::TooSmall);
        }

        // --- Header ---
        if &data[0..4] != MAGIC {
            return Err(NgramError::BadMagic);
        }
        let format_version = u16::from_le_bytes([data[4], data[5]]);
        if format_version != 1 {
            return Err(NgramError::UnsupportedVersion(format_version));
        }
        // flags at [6..8] — reserved, ignored
        let vocab_size = u16::from_le_bytes([data[8], data[9]]) as usize;
        // smoothing at [10] — informational, ignored by scorer
        let min_count = data[11];
        let n_unigrams = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let n_bigrams = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let n_trigrams = u32::from_le_bytes([data[20], data[21], data[22], data[23]]) as usize;
        let alpha = f32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        // build_info at [28..32] — informational, ignored

        let mut pos = HEADER_SIZE;

        // --- String table ---
        let mut word_to_id = HashMap::with_capacity(vocab_size);
        for id in 0..vocab_size {
            if pos >= data.len() {
                return Err(NgramError::Truncated);
            }
            let len = data[pos] as usize;
            pos += 1;
            if pos + len > data.len() {
                return Err(NgramError::Truncated);
            }
            let word = std::str::from_utf8(&data[pos..pos + len])
                .map_err(|_| NgramError::InvalidUtf8)?
                .to_string();
            pos += len;
            word_to_id.insert(word, id as u16);
        }

        // --- Unigrams (f32 × n_unigrams) ---
        let unigram_bytes = n_unigrams * 4;
        if pos + unigram_bytes > data.len() {
            return Err(NgramError::Truncated);
        }
        let mut unigram_scores = Vec::with_capacity(n_unigrams);
        for i in 0..n_unigrams {
            let off = pos + i * 4;
            let score =
                f32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
            unigram_scores.push(score);
        }
        pos += unigram_bytes;

        // --- Bigrams (8 bytes each: u16 w1, u16 w2, f32 score) ---
        let bigram_bytes = n_bigrams * 8;
        if pos + bigram_bytes > data.len() {
            return Err(NgramError::Truncated);
        }
        let mut bigrams = Vec::with_capacity(n_bigrams);
        for i in 0..n_bigrams {
            let off = pos + i * 8;
            let w1 = u16::from_le_bytes([data[off], data[off + 1]]);
            let w2 = u16::from_le_bytes([data[off + 2], data[off + 3]]);
            let score =
                f32::from_le_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]);
            bigrams.push(BigramEntry { w1, w2, score });
        }
        pos += bigram_bytes;

        // --- Trigrams (12 bytes each: u16 w1, u16 w2, u16 w3, u16 _pad, f32 score) ---
        let trigram_bytes = n_trigrams * 12;
        if pos + trigram_bytes > data.len() {
            return Err(NgramError::Truncated);
        }
        let mut trigrams = Vec::with_capacity(n_trigrams);
        for i in 0..n_trigrams {
            let off = pos + i * 12;
            let w1 = u16::from_le_bytes([data[off], data[off + 1]]);
            let w2 = u16::from_le_bytes([data[off + 2], data[off + 3]]);
            let w3 = u16::from_le_bytes([data[off + 4], data[off + 5]]);
            // _pad at off+6..off+8, skip
            let score =
                f32::from_le_bytes([data[off + 8], data[off + 9], data[off + 10], data[off + 11]]);
            trigrams.push(TrigramEntry { w1, w2, w3, score });
        }

        Ok(Self {
            word_to_id,
            unigram_scores,
            bigrams,
            trigrams,
            alpha,
            min_count,
        })
    }

    /// Load n-gram data from the embedded binary (set by build script).
    #[cfg(feature = "embed-ngram")]
    pub fn from_embedded() -> Self {
        let data = include_bytes!(env!("THAIME_NGRAM_PATH"));
        Self::from_bytes(data).expect("embedded ngram data is corrupt")
    }

    /// Create from in-memory count data.
    ///
    /// Converts raw counts to log₁₀ probabilities and builds sorted arrays.
    /// Used for testing and TSV import paths.
    pub fn from_raw(
        unigram_counts: HashMap<String, u64>,
        bigram_counts: HashMap<(String, String), u64>,
        trigram_counts: HashMap<(String, String, String), u64>,
    ) -> Self {
        // Collect all words and assign IDs (sorted for determinism)
        let mut words: Vec<String> = unigram_counts.keys().cloned().collect();
        // Also include words from bigrams/trigrams not in unigrams
        for (w1, w2) in bigram_counts.keys() {
            if !unigram_counts.contains_key(w1) {
                words.push(w1.clone());
            }
            if !unigram_counts.contains_key(w2) {
                words.push(w2.clone());
            }
        }
        for (w1, w2, w3) in trigram_counts.keys() {
            for w in [w1, w2, w3] {
                if !unigram_counts.contains_key(w) && !words.contains(w) {
                    words.push(w.clone());
                }
            }
        }
        words.sort();
        words.dedup();

        let word_to_id: HashMap<String, u16> = words
            .iter()
            .enumerate()
            .map(|(i, w)| (w.clone(), i as u16))
            .collect();

        // Compute log₁₀ unigram probabilities
        let total: u64 = unigram_counts.values().sum();
        let total_f64 = total.max(1) as f64;
        let mut unigram_scores = vec![FLOOR_PROB.log10() as f32; words.len()];
        for (word, &count) in &unigram_counts {
            if let Some(&id) = word_to_id.get(word) {
                let prob = count as f64 / total_f64;
                unigram_scores[id as usize] = prob.log10() as f32;
            }
        }

        // Compute log₁₀ bigram conditional probabilities
        let mut bigrams: Vec<BigramEntry> = bigram_counts
            .iter()
            .filter_map(|((w1, w2), &bg_count)| {
                let id1 = *word_to_id.get(w1)?;
                let id2 = *word_to_id.get(w2)?;
                let uni_count = unigram_counts.get(w1).copied().unwrap_or(1);
                let score = (bg_count as f64 / uni_count.max(1) as f64).log10() as f32;
                Some(BigramEntry {
                    w1: id1,
                    w2: id2,
                    score,
                })
            })
            .collect();
        bigrams.sort_by_key(|e| (e.w1, e.w2));

        // Compute log₁₀ trigram conditional probabilities
        let mut trigrams: Vec<TrigramEntry> = trigram_counts
            .iter()
            .filter_map(|((w1, w2, w3), &tri_count)| {
                let id1 = *word_to_id.get(w1)?;
                let id2 = *word_to_id.get(w2)?;
                let id3 = *word_to_id.get(w3)?;
                let bg_count = bigram_counts
                    .get(&(w1.clone(), w2.clone()))
                    .copied()
                    .unwrap_or(1);
                let score = (tri_count as f64 / bg_count.max(1) as f64).log10() as f32;
                Some(TrigramEntry {
                    w1: id1,
                    w2: id2,
                    w3: id3,
                    score,
                })
            })
            .collect();
        trigrams.sort_by_key(|e| (e.w1, e.w2, e.w3));

        Self {
            word_to_id,
            unigram_scores,
            bigrams,
            trigrams,
            alpha: crate::config::DEFAULT_ALPHA as f32,
            min_count: 0,
        }
    }

    // --- Scoring methods (same signatures, new internals) ---

    /// Unigram probability: `10^(log₁₀(P(w)))`.
    ///
    /// Returns [`FLOOR_PROB`] for unknown words.
    pub fn unigram_prob(&self, w: &str) -> f64 {
        match self.word_to_id.get(w) {
            Some(&id) => 10f64.powf(self.unigram_scores[id as usize] as f64),
            None => FLOOR_PROB,
        }
    }

    /// Compute Stupid Backoff score for word `w` given previous word `w_prev`.
    ///
    /// Returns a probability-like score (higher = more likely).
    ///
    /// - If `w_prev` is `Some` and the bigram exists:
    ///   `10^(log₁₀(P(w|w_prev)))`
    /// - Otherwise: `alpha * P_unigram(w)`
    /// - If `w_prev` is `None` (BOS): `P_unigram(w)` (no alpha penalty)
    pub fn bigram_score(&self, w_prev: Option<&str>, w: &str, alpha: f64) -> f64 {
        match w_prev {
            Some(prev) => {
                if let (Some(&id_prev), Some(&id_w)) =
                    (self.word_to_id.get(prev), self.word_to_id.get(w))
                {
                    if let Some(score) = self.lookup_bigram(id_prev, id_w) {
                        return 10f64.powf(score as f64);
                    }
                }
                alpha * self.unigram_prob(w)
            }
            None => self.unigram_prob(w),
        }
    }

    /// Compute Stupid Backoff score with trigram→bigram→unigram fallback.
    ///
    /// - `w_prev2`: two words back (`None` if unavailable)
    /// - `w_prev1`: one word back (`None` at BOS)
    /// - `w`: current word
    /// - `alpha`: backoff penalty factor
    pub fn trigram_score(
        &self,
        w_prev2: Option<&str>,
        w_prev1: Option<&str>,
        w: &str,
        alpha: f64,
    ) -> f64 {
        // Try trigram if both previous words are available
        if let (Some(p2), Some(p1)) = (w_prev2, w_prev1) {
            if let (Some(&id2), Some(&id1), Some(&id_w)) = (
                self.word_to_id.get(p2),
                self.word_to_id.get(p1),
                self.word_to_id.get(w),
            ) {
                if let Some(score) = self.lookup_trigram(id2, id1, id_w) {
                    return 10f64.powf(score as f64);
                }
            }
        }
        // Fall back: apply alpha penalty only when we actually had trigram
        // context to back off from (w_prev2 was Some). At BOS (w_prev2=None)
        // there's no trigram level to penalize, so delegate directly.
        if w_prev2.is_some() {
            alpha * self.bigram_score(w_prev1, w, alpha)
        } else {
            self.bigram_score(w_prev1, w, alpha)
        }
    }

    /// Number of unigram entries (= vocabulary size).
    pub fn unigram_count(&self) -> usize {
        self.unigram_scores.len()
    }

    /// Number of bigram entries.
    pub fn bigram_count(&self) -> usize {
        self.bigrams.len()
    }

    /// Number of trigram entries.
    pub fn trigram_count(&self) -> usize {
        self.trigrams.len()
    }

    /// Backoff weight from the binary header.
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Minimum count threshold used during binary generation.
    pub fn min_count(&self) -> u8 {
        self.min_count
    }

    /// Vocabulary size (number of unique words).
    pub fn vocab_size(&self) -> usize {
        self.word_to_id.len()
    }

    // --- Private helpers ---

    /// Binary search for a bigram entry.
    fn lookup_bigram(&self, w1: u16, w2: u16) -> Option<f32> {
        self.bigrams
            .binary_search_by_key(&(w1, w2), |e| (e.w1, e.w2))
            .ok()
            .map(|idx| self.bigrams[idx].score)
    }

    /// Binary search for a trigram entry.
    fn lookup_trigram(&self, w1: u16, w2: u16, w3: u16) -> Option<f32> {
        self.trigrams
            .binary_search_by_key(&(w1, w2, w3), |e| (e.w1, e.w2, e.w3))
            .ok()
            .map(|idx| self.trigrams[idx].score)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Tolerance for f32 round-trip through log₁₀ → 10^x
    const TOL: f64 = 1e-6;

    fn make_test_ngram() -> NgramData {
        let mut unigrams = HashMap::new();
        unigrams.insert("ไม่".to_string(), 1000);
        unigrams.insert("ใน".to_string(), 800);
        unigrams.insert("ได้".to_string(), 500);
        unigrams.insert("มา".to_string(), 300);
        unigrams.insert("การ".to_string(), 600);

        let mut bigrams = HashMap::new();
        bigrams.insert(("ไม่".to_string(), "ได้".to_string()), 200);
        bigrams.insert(("ใน".to_string(), "การ".to_string()), 150);
        bigrams.insert(("ไม่".to_string(), "มา".to_string()), 80);

        let mut trigrams = HashMap::new();
        trigrams.insert(("ไม่".to_string(), "ได้".to_string(), "มา".to_string()), 50);

        NgramData::from_raw(unigrams, bigrams, trigrams)
    }

    #[test]
    fn test_unigram_prob() {
        let ngram = make_test_ngram();
        let total = 1000 + 800 + 500 + 300 + 600; // 3200
        let prob = ngram.unigram_prob("ไม่");
        assert!((prob - 1000.0 / total as f64).abs() < TOL);
    }

    #[test]
    fn test_unigram_prob_unseen() {
        let ngram = make_test_ngram();
        let prob = ngram.unigram_prob("UNKNOWN");
        assert!((prob - FLOOR_PROB).abs() < TOL);
    }

    #[test]
    fn test_bigram_score_exists() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        // bigram(ไม่, ได้) = 200, unigram(ไม่) = 1000
        let score = ngram.bigram_score(Some("ไม่"), "ได้", alpha);
        assert!((score - 200.0 / 1000.0).abs() < TOL);
    }

    #[test]
    fn test_bigram_score_fallback_to_unigram() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // No bigram (ใน, มา), so fallback: alpha * P(มา)
        let score = ngram.bigram_score(Some("ใน"), "มา", alpha);
        assert!((score - alpha * 300.0 / total).abs() < TOL);
    }

    #[test]
    fn test_bigram_score_bos() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // BOS (None) → P_unigram(ไม่), no alpha penalty
        let score = ngram.bigram_score(None, "ไม่", alpha);
        assert!((score - 1000.0 / total).abs() < TOL);
    }

    #[test]
    fn test_bigram_score_unseen_prev() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // w_prev not in unigrams at all → fallback: alpha * P(ใน)
        let score = ngram.bigram_score(Some("UNKNOWN"), "ใน", alpha);
        assert!((score - alpha * 800.0 / total).abs() < TOL);
    }

    #[test]
    fn test_counts() {
        let ngram = make_test_ngram();
        assert_eq!(ngram.unigram_count(), 5);
        assert_eq!(ngram.bigram_count(), 3);
        assert_eq!(ngram.trigram_count(), 1);
    }

    // --- Trigram scoring tests ---

    #[test]
    fn test_trigram_score_exists() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        // trigram(ไม่, ได้, มา) = 50, bigram(ไม่, ได้) = 200
        let score = ngram.trigram_score(Some("ไม่"), Some("ได้"), "มา", alpha);
        assert!((score - 50.0 / 200.0).abs() < TOL);
    }

    #[test]
    fn test_trigram_score_fallback_to_bigram() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // No trigram (ใน, การ, มา), fallback: alpha * bigram_score(การ, มา)
        // No bigram (การ, มา), so further fallback: alpha * alpha * P(มา)
        let expected = alpha * alpha * (300.0 / total);
        let score = ngram.trigram_score(Some("ใน"), Some("การ"), "มา", alpha);
        assert!((score - expected).abs() < TOL);
    }

    #[test]
    fn test_trigram_score_fallback_when_bigram_exists() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        // No trigram (ใน, ไม่, ได้), but bigram(ไม่, ได้) = 200, unigram(ไม่) = 1000
        // Fallback: alpha * bigram_score(Some("ไม่"), "ได้", alpha)
        //         = alpha * (200 / 1000)
        let expected = alpha * (200.0 / 1000.0);
        let score = ngram.trigram_score(Some("ใน"), Some("ไม่"), "ได้", alpha);
        assert!((score - expected).abs() < TOL);
    }

    #[test]
    fn test_trigram_score_partial_context() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        // Only w_prev1 provided (w_prev2 = None) → no trigram context to back
        // off from, so delegate directly to bigram_score (no extra alpha).
        // bigram(ได้, มา)? No. So bigram_score falls back: alpha * P(มา)
        let total = 3200.0;
        let expected = alpha * (300.0 / total);
        let score = ngram.trigram_score(None, Some("ได้"), "มา", alpha);
        assert!((score - expected).abs() < TOL);
    }

    #[test]
    fn test_trigram_score_bos() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // Both None → no trigram context, delegate directly to bigram_score.
        // bigram_score(None, "ไม่", alpha) = P_unigram(ไม่) (BOS, no alpha)
        let expected = 1000.0 / total;
        let score = ngram.trigram_score(None, None, "ไม่", alpha);
        assert!((score - expected).abs() < TOL);
    }

    // --- Binary parser tests ---

    #[test]
    fn test_from_bytes_bad_magic() {
        let data = vec![0u8; 64];
        assert!(matches!(
            NgramData::from_bytes(&data),
            Err(NgramError::BadMagic)
        ));
    }

    #[test]
    fn test_from_bytes_too_small() {
        let data = vec![0u8; 16];
        assert!(matches!(
            NgramData::from_bytes(&data),
            Err(NgramError::TooSmall)
        ));
    }

    #[test]
    fn test_from_bytes_unsupported_version() {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(b"TNLM");
        data[4] = 99; // version 99
        assert!(matches!(
            NgramData::from_bytes(&data),
            Err(NgramError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn test_from_bytes_truncated() {
        let mut data = vec![0u8; HEADER_SIZE];
        data[0..4].copy_from_slice(b"TNLM");
        data[4] = 1; // version 1
                     // vocab_size = 1, but no string table data
        data[8] = 1;
        // n_unigrams = 1
        data[12] = 1;
        assert!(matches!(
            NgramData::from_bytes(&data),
            Err(NgramError::Truncated)
        ));
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        // Build a minimal binary by hand: 1 word "ทดสอบ", 0 bigrams, 0 trigrams
        let word = "ทดสอบ";
        let word_bytes = word.as_bytes();
        let vocab_size: u16 = 1;
        let n_unigrams: u32 = 1;
        let n_bigrams: u32 = 0;
        let n_trigrams: u32 = 0;
        let alpha: f32 = 0.4;
        let unigram_score: f32 = -1.5; // log₁₀(P)

        let mut data = Vec::new();
        // Header
        data.extend_from_slice(b"TNLM");
        data.extend_from_slice(&1u16.to_le_bytes()); // version
        data.extend_from_slice(&0u16.to_le_bytes()); // flags
        data.extend_from_slice(&vocab_size.to_le_bytes());
        data.push(0); // smoothing
        data.push(10); // min_count
        data.extend_from_slice(&n_unigrams.to_le_bytes());
        data.extend_from_slice(&n_bigrams.to_le_bytes());
        data.extend_from_slice(&n_trigrams.to_le_bytes());
        data.extend_from_slice(&alpha.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes()); // build_info
        assert_eq!(data.len(), HEADER_SIZE);

        // String table
        data.push(word_bytes.len() as u8);
        data.extend_from_slice(word_bytes);

        // Unigrams
        data.extend_from_slice(&unigram_score.to_le_bytes());

        let ngram = NgramData::from_bytes(&data).unwrap();
        assert_eq!(ngram.vocab_size(), 1);
        assert_eq!(ngram.unigram_count(), 1);
        assert_eq!(ngram.bigram_count(), 0);
        assert_eq!(ngram.trigram_count(), 0);
        assert!((ngram.alpha() - 0.4).abs() < 1e-6);
        assert_eq!(ngram.min_count(), 10);

        // Verify score: 10^(-1.5) ≈ 0.031623
        let prob = ngram.unigram_prob(word);
        assert!((prob - 10f64.powf(-1.5)).abs() < TOL);
    }

    /// Parse actual test binary from data/input/ (if available).
    #[test]
    fn test_parse_real_binary() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/input/thaime_ngram_v1_mc20.bin");
        if !path.exists() {
            eprintln!("Skipping test_parse_real_binary: file not found");
            return;
        }
        let data = std::fs::read(&path).unwrap();
        let ngram = NgramData::from_bytes(&data).unwrap();

        // Verify header fields from known mc20 binary
        assert_eq!(ngram.min_count(), 20);
        assert!((ngram.alpha() - 0.4).abs() < 1e-6);
        assert!(ngram.vocab_size() > 1000);
        assert!(ngram.bigram_count() > 10_000);
        assert!(ngram.trigram_count() > 10_000);

        // Common Thai word should have reasonable probability
        let prob = ngram.unigram_prob("ที่");
        assert!(prob > 0.001, "ที่ should be a very common word");

        // Unknown word should get floor
        let prob_unk = ngram.unigram_prob("XYZZY");
        assert!((prob_unk - FLOOR_PROB).abs() < TOL);
    }

    #[test]
    fn test_binary_search_edge_cases() {
        let mut unigrams = HashMap::new();
        unigrams.insert("a".to_string(), 100);
        unigrams.insert("b".to_string(), 80);
        unigrams.insert("c".to_string(), 60);

        let mut bigrams = HashMap::new();
        bigrams.insert(("a".to_string(), "b".to_string()), 50);
        bigrams.insert(("a".to_string(), "c".to_string()), 30);
        bigrams.insert(("b".to_string(), "c".to_string()), 20);

        let ngram = NgramData::from_raw(unigrams, bigrams, HashMap::new());

        // First entry
        let score = ngram.bigram_score(Some("a"), "b", 0.4);
        assert!((score - 50.0 / 100.0).abs() < TOL);

        // Last entry
        let score = ngram.bigram_score(Some("b"), "c", 0.4);
        assert!((score - 20.0 / 80.0).abs() < TOL);

        // Missing entry — falls back
        let total = 240.0;
        let score = ngram.bigram_score(Some("c"), "a", 0.4);
        assert!((score - 0.4 * 100.0 / total).abs() < TOL);
    }
}

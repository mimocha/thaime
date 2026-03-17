// SPDX-License-Identifier: MPL-2.0

//! N-gram data storage and scoring (dev mode).
//!
//! Loads raw TSV count files at startup and computes Stupid Backoff
//! scores at query time. Supports trigram→bigram→unigram fallback.
//!
//! ## Stupid Backoff (trigram)
//!
//! ```text
//! score(w | w_prev2, w_prev1):
//!   if trigram_count(w_prev2, w_prev1, w) > 0:
//!       trigram_count(w_prev2, w_prev1, w) / bigram_count(w_prev2, w_prev1)
//!   elif bigram_count(w_prev1, w) > 0:
//!       alpha * bigram_count(w_prev1, w) / unigram_count(w_prev1)
//!   else:
//!       alpha^2 * P_unigram(w)
//! ```
//!
//! When `w_prev2` is `None`, degrades to bigram. When both are `None`
//! (beginning of sentence), falls back to unigram probability directly.

use std::collections::HashMap;
use std::io::{self, BufRead};
use std::path::Path;

/// N-gram count data for Stupid Backoff scoring.
#[derive(Debug)]
pub struct NgramData {
    unigram_counts: HashMap<String, u64>,
    bigram_counts: HashMap<(String, String), u64>,
    trigram_counts: HashMap<(String, String, String), u64>,
    total_unigram_count: u64,
}

use crate::config::FLOOR_PROB;

impl NgramData {
    /// Load n-gram data from raw TSV count files.
    ///
    /// - `unigram_path`: Tab-separated `thai\tcount` (one per line)
    /// - `bigram_path`: Tab-separated `w1\tw2\tcount` (one per line)
    /// - `trigram_path`: Optional tab-separated `w1\tw2\tw3\tcount` (one per line)
    /// - `trigram_min_count`: Minimum count threshold for trigram entries
    ///
    /// Optionally filters entries to only include words present in `vocab`.
    /// If `vocab` is `None`, all entries are loaded.
    pub fn from_tsv_files(
        unigram_path: &Path,
        bigram_path: &Path,
        trigram_path: Option<&Path>,
        trigram_min_count: u64,
        vocab: Option<&std::collections::HashSet<String>>,
    ) -> io::Result<Self> {
        let unigram_counts = Self::load_unigrams(unigram_path, vocab)?;
        let total_unigram_count = unigram_counts.values().sum();
        let bigram_counts = Self::load_bigrams(bigram_path, vocab)?;
        let trigram_counts = match trigram_path {
            Some(path) => Self::load_trigrams(path, trigram_min_count, vocab)?,
            None => HashMap::new(),
        };

        Ok(Self {
            unigram_counts,
            bigram_counts,
            trigram_counts,
            total_unigram_count,
        })
    }

    /// Create from in-memory data (for testing).
    #[cfg(test)]
    pub fn from_raw(
        unigram_counts: HashMap<String, u64>,
        bigram_counts: HashMap<(String, String), u64>,
        trigram_counts: HashMap<(String, String, String), u64>,
    ) -> Self {
        let total_unigram_count = unigram_counts.values().sum();
        Self {
            unigram_counts,
            bigram_counts,
            trigram_counts,
            total_unigram_count,
        }
    }

    /// Compute Stupid Backoff score for word `w` given previous word `w_prev`.
    ///
    /// Returns a probability-like score (higher = more likely).
    ///
    /// - If `w_prev` is `Some` and the bigram exists:
    ///   `bigram_count(w_prev, w) / unigram_count(w_prev)`
    /// - Otherwise: `alpha * P_unigram(w)`
    /// - If `w_prev` is `None` (BOS): `P_unigram(w)` (no alpha penalty)
    pub fn bigram_score(&self, w_prev: Option<&str>, w: &str, alpha: f64) -> f64 {
        match w_prev {
            Some(prev) => {
                let bigram_key = (prev.to_string(), w.to_string());
                if let Some(&bg_count) = self.bigram_counts.get(&bigram_key) {
                    let uni_count = self.unigram_counts.get(prev).copied().unwrap_or(1);
                    bg_count as f64 / uni_count as f64
                } else {
                    alpha * self.unigram_prob(w)
                }
            }
            None => {
                // BOS: just use unigram probability (no alpha penalty)
                self.unigram_prob(w)
            }
        }
    }

    /// Unigram probability: `count(w) / total_count`.
    pub fn unigram_prob(&self, w: &str) -> f64 {
        if self.total_unigram_count == 0 {
            return FLOOR_PROB;
        }
        let count = self.unigram_counts.get(w).copied().unwrap_or(0);
        if count == 0 {
            FLOOR_PROB
        } else {
            count as f64 / self.total_unigram_count as f64
        }
    }

    /// Number of unigram entries loaded.
    pub fn unigram_count(&self) -> usize {
        self.unigram_counts.len()
    }

    /// Number of bigram entries loaded.
    pub fn bigram_count(&self) -> usize {
        self.bigram_counts.len()
    }

    /// Number of trigram entries loaded.
    pub fn trigram_count(&self) -> usize {
        self.trigram_counts.len()
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
            let tri_key = (p2.to_string(), p1.to_string(), w.to_string());
            if let Some(&tri_count) = self.trigram_counts.get(&tri_key) {
                let bg_key = (p2.to_string(), p1.to_string());
                let bg_count = self.bigram_counts.get(&bg_key).copied().unwrap_or(1);
                return tri_count as f64 / bg_count as f64;
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

    fn load_unigrams(
        path: &Path,
        vocab: Option<&std::collections::HashSet<String>>,
    ) -> io::Result<HashMap<String, u64>> {
        let file = std::fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut counts = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split('\t');
            let word = match parts.next() {
                Some(w) => w.to_string(),
                None => continue,
            };
            let count: u64 = match parts.next().and_then(|s| s.parse().ok()) {
                Some(c) => c,
                None => continue,
            };
            if let Some(v) = vocab {
                if !v.contains(&word) {
                    continue;
                }
            }
            counts.insert(word, count);
        }

        Ok(counts)
    }

    fn load_bigrams(
        path: &Path,
        vocab: Option<&std::collections::HashSet<String>>,
    ) -> io::Result<HashMap<(String, String), u64>> {
        let file = std::fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut counts = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split('\t');
            let w1 = match parts.next() {
                Some(w) => w.to_string(),
                None => continue,
            };
            let w2 = match parts.next() {
                Some(w) => w.to_string(),
                None => continue,
            };
            let count: u64 = match parts.next().and_then(|s| s.parse().ok()) {
                Some(c) => c,
                None => continue,
            };
            if let Some(v) = vocab {
                if !v.contains(&w1) || !v.contains(&w2) {
                    continue;
                }
            }
            counts.insert((w1, w2), count);
        }

        Ok(counts)
    }

    fn load_trigrams(
        path: &Path,
        min_count: u64,
        vocab: Option<&std::collections::HashSet<String>>,
    ) -> io::Result<HashMap<(String, String, String), u64>> {
        let file = std::fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut counts = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split('\t');
            let w1 = match parts.next() {
                Some(w) => w.to_string(),
                None => continue,
            };
            let w2 = match parts.next() {
                Some(w) => w.to_string(),
                None => continue,
            };
            let w3 = match parts.next() {
                Some(w) => w.to_string(),
                None => continue,
            };
            let count: u64 = match parts.next().and_then(|s| s.parse().ok()) {
                Some(c) => c,
                None => continue,
            };
            if count < min_count {
                continue;
            }
            if let Some(v) = vocab {
                if !v.contains(&w1) || !v.contains(&w2) || !v.contains(&w3) {
                    continue;
                }
            }
            counts.insert((w1, w2, w3), count);
        }

        Ok(counts)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!((prob - 1000.0 / total as f64).abs() < 1e-10);
    }

    #[test]
    fn test_unigram_prob_unseen() {
        let ngram = make_test_ngram();
        let prob = ngram.unigram_prob("UNKNOWN");
        assert!((prob - FLOOR_PROB).abs() < 1e-15);
    }

    #[test]
    fn test_bigram_score_exists() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        // bigram(ไม่, ได้) = 200, unigram(ไม่) = 1000
        let score = ngram.bigram_score(Some("ไม่"), "ได้", alpha);
        assert!((score - 200.0 / 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_bigram_score_fallback_to_unigram() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // No bigram (ใน, มา), so fallback: alpha * P(มา)
        let score = ngram.bigram_score(Some("ใน"), "มา", alpha);
        assert!((score - alpha * 300.0 / total).abs() < 1e-10);
    }

    #[test]
    fn test_bigram_score_bos() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // BOS (None) → P_unigram(ไม่), no alpha penalty
        let score = ngram.bigram_score(None, "ไม่", alpha);
        assert!((score - 1000.0 / total).abs() < 1e-10);
    }

    #[test]
    fn test_bigram_score_unseen_prev() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 3200.0;
        // w_prev not in unigrams at all → fallback: alpha * P(ใน)
        let score = ngram.bigram_score(Some("UNKNOWN"), "ใน", alpha);
        assert!((score - alpha * 800.0 / total).abs() < 1e-10);
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
        assert!((score - 50.0 / 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_trigram_score_fallback_to_bigram() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        // No trigram (ใน, การ, มา), but bigram exists: (การ, ...) — no,
        // fallback to alpha * bigram_score(Some("การ"), "มา", alpha).
        // No bigram (การ, มา), so further fallback: alpha * alpha * P(มา)
        let total = 3200.0;
        let expected = alpha * alpha * (300.0 / total);
        let score = ngram.trigram_score(Some("ใน"), Some("การ"), "มา", alpha);
        assert!((score - expected).abs() < 1e-10);
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
        assert!((score - expected).abs() < 1e-10);
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
        assert!((score - expected).abs() < 1e-10);
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
        assert!((score - expected).abs() < 1e-10);
    }
}

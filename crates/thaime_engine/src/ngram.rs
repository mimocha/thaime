// SPDX-License-Identifier: MPL-2.0

//! N-gram data storage and scoring (dev mode).
//!
//! Loads raw TSV count files at startup and computes Stupid Backoff
//! scores at query time. This supports bigram context-dependent
//! candidate ranking.
//!
//! ## Stupid Backoff
//!
//! ```text
//! score(w | w_prev):
//!   if bigram_count(w_prev, w) > 0:
//!       bigram_count(w_prev, w) / unigram_count(w_prev)
//!   else:
//!       alpha * P_unigram(w)
//! ```
//!
//! When `w_prev` is `None` (beginning of sentence), falls back to
//! unigram probability directly.

use std::collections::HashMap;
use std::io::{self, BufRead};
use std::path::Path;

/// N-gram count data for Stupid Backoff scoring.
#[derive(Debug)]
pub struct NgramData {
    unigram_counts: HashMap<String, u64>,
    bigram_counts: HashMap<(String, String), u64>,
    total_unigram_count: u64,
}

/// Floor probability for unseen words to avoid log(0).
const FLOOR_PROB: f64 = 1e-10;

impl NgramData {
    /// Load n-gram data from raw TSV count files.
    ///
    /// - `unigram_path`: Tab-separated `thai\tcount` (one per line)
    /// - `bigram_path`: Tab-separated `w1\tw2\tcount` (one per line)
    ///
    /// Optionally filters entries to only include words present in `vocab`.
    /// If `vocab` is `None`, all entries are loaded.
    pub fn from_tsv_files(
        unigram_path: &Path,
        bigram_path: &Path,
        vocab: Option<&std::collections::HashSet<String>>,
    ) -> io::Result<Self> {
        let unigram_counts = Self::load_unigrams(unigram_path, vocab)?;
        let total_unigram_count = unigram_counts.values().sum();
        let bigram_counts = Self::load_bigrams(bigram_path, vocab)?;

        Ok(Self {
            unigram_counts,
            bigram_counts,
            total_unigram_count,
        })
    }

    /// Create from in-memory data (for testing).
    #[cfg(test)]
    pub fn from_raw(
        unigram_counts: HashMap<String, u64>,
        bigram_counts: HashMap<(String, String), u64>,
    ) -> Self {
        let total_unigram_count = unigram_counts.values().sum();
        Self {
            unigram_counts,
            bigram_counts,
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

        let mut bigrams = HashMap::new();
        bigrams.insert(("ไม่".to_string(), "ได้".to_string()), 200);
        bigrams.insert(("ใน".to_string(), "การ".to_string()), 150);

        NgramData::from_raw(unigrams, bigrams)
    }

    #[test]
    fn test_unigram_prob() {
        let ngram = make_test_ngram();
        let total = 1000 + 800 + 500 + 300; // 2600
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
        let total = 2600.0;
        // No bigram (ไม่, มา), so fallback: alpha * P(มา)
        let score = ngram.bigram_score(Some("ไม่"), "มา", alpha);
        assert!((score - alpha * 300.0 / total).abs() < 1e-10);
    }

    #[test]
    fn test_bigram_score_bos() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 2600.0;
        // BOS (None) → P_unigram(ไม่), no alpha penalty
        let score = ngram.bigram_score(None, "ไม่", alpha);
        assert!((score - 1000.0 / total).abs() < 1e-10);
    }

    #[test]
    fn test_bigram_score_unseen_prev() {
        let ngram = make_test_ngram();
        let alpha = 0.4;
        let total = 2600.0;
        // w_prev not in unigrams at all → fallback: alpha * P(ใน)
        let score = ngram.bigram_score(Some("UNKNOWN"), "ใน", alpha);
        assert!((score - alpha * 800.0 / total).abs() < 1e-10);
    }

    #[test]
    fn test_counts() {
        let ngram = make_test_ngram();
        assert_eq!(ngram.unigram_count(), 4);
        assert_eq!(ngram.bigram_count(), 2);
    }
}

// SPDX-License-Identifier: MPL-2.0

//! Candidate ranking via Viterbi k-best on a word lattice.
//!
//! Given a Latin input string and a dictionary, this module:
//! 1. Builds a word lattice (DAG) by running `common_prefix_search` at every
//!    position in the input
//! 2. Scores paths through the lattice using Viterbi DP with k-best tracking
//! 3. Returns deduplicated, ranked Thai candidates
//!
//! Scoring formula (MVP, unigram):
//!   edge_cost = -ln(max(freq, MIN_FREQ)) + LAMBDA
//!   path_cost = sum of edge costs
//!
//! Lower cost = better candidate.

use std::collections::HashSet;

use crate::trie::Dictionary;

/// Segmentation penalty added per word. Higher values favor fewer, longer words.
/// Tuned on mock data — revisit once real-data testing is possible via the CLI.
pub const LAMBDA: f64 = 1.0;

/// Floor for word frequency to avoid -ln(0).
const MIN_FREQ: f64 = 1e-4;

/// Default number of candidates to track per lattice position.
pub const DEFAULT_K: usize = 10;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A ranked candidate: one possible interpretation of the full Latin input.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Concatenated Thai text (all words joined).
    pub thai: String,
    /// Individual Thai words that make up this candidate.
    pub words: Vec<String>,
    /// Total path cost (lower = better).
    pub score: f64,
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// An edge in the word lattice.
struct LatticeEdge {
    start: usize,
    thai: String,
    cost: f64,
}

/// A partial path through the lattice (used during Viterbi forward pass).
#[derive(Clone)]
struct PartialPath {
    cost: f64,
    words: Vec<String>,
}

// ---------------------------------------------------------------------------
// Core algorithm
// ---------------------------------------------------------------------------

/// Build a word lattice from the input and score all complete paths.
///
/// Returns up to `k` candidates, deduplicated by Thai text and sorted by
/// ascending cost (best first). Returns an empty list if no complete tiling
/// of the input exists.
pub fn rank_candidates(input: &str, dictionary: &Dictionary, k: usize) -> Vec<Candidate> {
    let n = input.len();
    if n == 0 {
        return Vec::new();
    }

    // --- Build lattice edges, grouped by end position ---
    //
    // edges_by_end[pos] contains all edges that END at byte position `pos`.
    // An edge from `start` to `end` means input[start..end] matched a
    // romanization key in the trie.

    let mut edges_by_end: Vec<Vec<LatticeEdge>> = (0..=n).map(|_| Vec::new()).collect();

    for start in 0..n {
        for prefix_match in dictionary.prefix_search(&input[start..]) {
            let end = start + prefix_match.prefix_len;
            for entry in &prefix_match.entries {
                let cost = -(entry.frequency.max(MIN_FREQ).ln()) + LAMBDA;
                edges_by_end[end].push(LatticeEdge {
                    start,
                    thai: entry.thai.clone(),
                    cost,
                });
            }
        }
    }

    // --- Viterbi forward pass with k-best tracking ---
    //
    // best[pos] holds the k-best partial paths ending at byte position `pos`.
    // A partial path at position 0 is the empty seed (cost=0, no words).

    let mut best: Vec<Vec<PartialPath>> = (0..=n).map(|_| Vec::new()).collect();
    best[0].push(PartialPath {
        cost: 0.0,
        words: Vec::new(),
    });

    for end in 1..=n {
        let mut candidates: Vec<PartialPath> = Vec::new();

        for edge in &edges_by_end[end] {
            for path in &best[edge.start] {
                let mut new_words = path.words.clone();
                new_words.push(edge.thai.clone());
                candidates.push(PartialPath {
                    cost: path.cost + edge.cost,
                    words: new_words,
                });
            }
        }

        // Sort by cost (ascending) and keep only the k-best
        candidates.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);
        best[end] = candidates;
    }

    // --- Collect complete paths and deduplicate ---

    let mut results: Vec<Candidate> = best[n]
        .iter()
        .map(|path| {
            let thai = path.words.concat();
            Candidate {
                thai,
                words: path.words.clone(),
                score: path.cost,
            }
        })
        .collect();

    // Deduplicate by Thai text, keeping the lowest-cost instance.
    // Results are already sorted by cost, so first occurrence wins.
    let mut seen = HashSet::new();
    results.retain(|c| seen.insert(c.thai.clone()));

    results
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::Dictionary;

    /// Build the same test dictionary as in trie::tests.
    ///
    /// word_id 0: ไม่   freq=0.013  romanizations: ["mai", "maai"]
    /// word_id 1: ใน   freq=0.012  romanizations: ["nai"]
    /// word_id 2: ไหม  freq=0.005  romanizations: ["mai"]
    /// word_id 3: สวัสดี freq=0.003  romanizations: ["sawatdee"]
    /// word_id 4: ใหม่  freq=0.004  romanizations: ["mai"]
    /// word_id 5: มา   freq=0.008  romanizations: ["ma", "maa"]
    fn build_test_dict() -> Dictionary {
        use crate::trie::tests::build_test_dict;
        build_test_dict()
    }

    #[test]
    fn test_empty_input() {
        let dict = build_test_dict();
        let results = rank_candidates("", &dict, DEFAULT_K);
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_match() {
        let dict = build_test_dict();
        let results = rank_candidates("xyz", &dict, DEFAULT_K);
        assert!(results.is_empty());
    }

    #[test]
    fn test_single_word_candidates() {
        let dict = build_test_dict();
        let results = rank_candidates("mai", &dict, DEFAULT_K);

        // "mai" maps to 3 words: ไม่ (0.013), ไหม (0.005), ใหม่ (0.004)
        // All are single-word complete paths covering the full input.
        assert_eq!(results.len(), 3);

        // Best candidate should be ไม่ (highest frequency = lowest cost)
        assert_eq!(results[0].thai, "ไม่");
        assert_eq!(results[0].words, vec!["ไม่"]);

        // Second should be ไหม
        assert_eq!(results[1].thai, "ไหม");

        // Third should be ใหม่
        assert_eq!(results[2].thai, "ใหม่");

        // Scores should be strictly increasing (lower = better)
        assert!(results[0].score < results[1].score);
        assert!(results[1].score < results[2].score);
    }

    #[test]
    fn test_multi_word_path() {
        let dict = build_test_dict();
        let results = rank_candidates("mainai", &dict, DEFAULT_K);

        // Possible complete paths (tiling all 6 bytes):
        //   "mai"(3) + "nai"(3) → ไม่ใน, ไหมใน, ใหม่ใน
        //   "ma"(2) + ???       → no word matches "inai" from position 2
        assert_eq!(results.len(), 3);

        // Best: ไม่ใน (ไม่ has highest freq among "mai" words)
        assert_eq!(results[0].thai, "ไม่ใน");
        assert_eq!(results[0].words, vec!["ไม่", "ใน"]);
    }

    #[test]
    fn test_single_long_word() {
        let dict = build_test_dict();
        let results = rank_candidates("sawatdee", &dict, DEFAULT_K);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].thai, "สวัสดี");
        assert_eq!(results[0].words, vec!["สวัสดี"]);
    }

    #[test]
    fn test_partial_coverage_returns_empty() {
        let dict = build_test_dict();
        // "maix" — "mai" matches positions 0..3, but "x" at position 3 has no match
        let results = rank_candidates("maix", &dict, DEFAULT_K);
        assert!(results.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let dict = build_test_dict();
        // "maai" maps only to ไม่ (word 0), so there's one path.
        // "mai" also maps to ไม่. If both variants lead to the same Thai output
        // in a multi-word context, dedup should keep only the best.
        let results = rank_candidates("maai", &dict, DEFAULT_K);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].thai, "ไม่");
    }

    #[test]
    fn test_k_limits_results() {
        let dict = build_test_dict();
        // With k=1, only the best candidate should be returned
        let results = rank_candidates("mai", &dict, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].thai, "ไม่");
    }

    #[test]
    fn test_score_includes_lambda_penalty() {
        let dict = build_test_dict();

        // Single word "nai" → ใน, freq=0.012
        // Expected cost: -ln(0.012) + 0.5
        let results = rank_candidates("nai", &dict, DEFAULT_K);
        assert_eq!(results.len(), 1);

        let expected_cost = -(0.012_f64.ln()) + LAMBDA;
        assert!((results[0].score - expected_cost).abs() < 1e-10);
    }

    #[test]
    fn test_multi_word_score_is_sum() {
        let dict = build_test_dict();

        // "mainai" → ไม่ใน = cost(ไม่) + cost(ใน)
        let single_mai = rank_candidates("mai", &dict, DEFAULT_K);
        let single_nai = rank_candidates("nai", &dict, DEFAULT_K);
        let combined = rank_candidates("mainai", &dict, DEFAULT_K);

        let mai_cost = single_mai[0].score; // ไม่
        let nai_cost = single_nai[0].score; // ใน
        let combined_cost = combined[0].score; // ไม่ใน

        assert!((combined_cost - (mai_cost + nai_cost)).abs() < 1e-10);
    }
}

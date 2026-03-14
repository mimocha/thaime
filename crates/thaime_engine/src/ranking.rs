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

use std::collections::{HashMap, HashSet};

use crate::ngram::NgramData;
use crate::trie::Dictionary;

/// Default segmentation penalty per word. Higher = fewer, longer words preferred.
pub const DEFAULT_LAMBDA: f64 = 1.0;

/// Default floor for word frequency to avoid -ln(0).
pub const DEFAULT_MIN_FREQ: f64 = 1e-4;

/// Default number of candidates to track per lattice position.
pub const DEFAULT_K: usize = 10;

/// Default bigram weight multiplier.
pub const DEFAULT_BIGRAM_WEIGHT: f64 = 2.0;

/// Default Stupid Backoff penalty factor.
pub const DEFAULT_ALPHA: f64 = 0.4;

/// Parameters controlling the ranking algorithm.
///
/// All fields have sensible defaults via `Default`. The TUI uses this to
/// allow runtime tuning; `InputContext` uses the defaults.
#[derive(Debug, Clone)]
pub struct RankingParams {
    /// Segmentation penalty added per word.
    pub lambda: f64,
    /// Floor for word frequency to avoid -ln(0).
    pub min_freq: f64,
    /// Number of best candidates to track per lattice position.
    pub k: usize,
    /// Bigram scoring weight multiplier.
    pub bigram_weight: f64,
    /// Stupid Backoff penalty factor.
    pub alpha: f64,
}

impl Default for RankingParams {
    fn default() -> Self {
        Self {
            lambda: DEFAULT_LAMBDA,
            min_freq: DEFAULT_MIN_FREQ,
            k: DEFAULT_K,
            bigram_weight: DEFAULT_BIGRAM_WEIGHT,
            alpha: DEFAULT_ALPHA,
        }
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single word within a candidate path.
#[derive(Debug, Clone)]
pub struct CandidateWord {
    /// Thai text for this word.
    pub thai: String,
    /// Raw word frequency from the dictionary.
    pub frequency: f64,
    /// Cost contribution: -ln(max(freq, MIN_FREQ)) + LAMBDA.
    pub cost: f64,
}

/// A ranked candidate: one possible interpretation of the full Latin input.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Concatenated Thai text (all words joined).
    pub thai: String,
    /// Individual words that make up this candidate, with per-word scoring.
    pub words: Vec<CandidateWord>,
    /// Total path cost (lower = better).
    pub score: f64,
    /// Sum of -ln(freq) components (without lambda penalties).
    pub freq_cost: f64,
    /// Total segmentation penalty: word_count * LAMBDA.
    pub seg_penalty: f64,
    /// Total bigram scoring contribution.
    pub bigram_cost: f64,
}

impl Candidate {
    /// Number of words in this candidate path.
    pub fn word_count(&self) -> usize {
        self.words.len()
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// An edge in the word lattice: a single trie match spanning [start, end) in the input.
#[derive(Debug, Clone)]
pub struct LatticeEdge {
    /// Start byte position in the input string.
    pub start: usize,
    /// End byte position in the input string.
    pub end: usize,
    /// Thai text for this word.
    pub thai: String,
    /// Word ID from the dictionary.
    pub word_id: u32,
    /// Raw word frequency from the dictionary.
    pub frequency: f64,
    /// Edge cost: -ln(max(freq, MIN_FREQ)) + LAMBDA.
    pub cost: f64,
}

/// A partial path through the lattice (used during Viterbi forward pass).
#[derive(Clone)]
struct PartialPath {
    cost: f64,
    words: Vec<CandidateWord>,
    /// Thai text of the last word in the path, for bigram state tracking.
    last_thai: Option<String>,
}

// ---------------------------------------------------------------------------
// Core algorithm
// ---------------------------------------------------------------------------

/// Result of ranking: candidates plus the lattice used to produce them.
pub struct RankingResult {
    /// Ranked candidates (deduplicated, best first).
    pub candidates: Vec<Candidate>,
    /// All lattice edges discovered during trie lookup.
    pub lattice_edges: Vec<LatticeEdge>,
}

/// Build a word lattice from the input and score all complete paths.
///
/// Returns up to `k` candidates, deduplicated by Thai text and sorted by
/// ascending cost (best first). Returns an empty list if no complete tiling
/// of the input exists. Also returns the full lattice for inspection.
///
/// When `ngram` is `Some`, bigram scoring is applied using Stupid Backoff.
/// The `context` slice provides previously committed Thai words; the last
/// entry is used as the bigram context for the first word in each path.
/// When `ngram` is `None`, the function behaves identically to unigram-only.
pub fn rank_candidates(
    input: &str,
    dictionary: &Dictionary,
    ngram: Option<&NgramData>,
    context: &[String],
    params: &RankingParams,
) -> RankingResult {
    let n = input.len();
    if n == 0 {
        return RankingResult {
            candidates: Vec::new(),
            lattice_edges: Vec::new(),
        };
    }

    // --- Build lattice edges, grouped by end position ---
    //
    // edges_by_end[pos] contains all edges that END at byte position `pos`.
    // An edge from `start` to `end` means input[start..end] matched a
    // romanization key in the trie.

    let mut all_edges: Vec<LatticeEdge> = Vec::new();
    let mut edges_by_end: Vec<Vec<usize>> = (0..=n).map(|_| Vec::new()).collect();

    for start in 0..n {
        for prefix_match in dictionary.prefix_search(&input[start..]) {
            let end = start + prefix_match.prefix_len;
            for entry in &prefix_match.entries {
                let cost = -(entry.frequency.max(params.min_freq).ln()) + params.lambda;
                let idx = all_edges.len();
                all_edges.push(LatticeEdge {
                    start,
                    end,
                    thai: entry.thai.clone(),
                    word_id: entry.word_id,
                    frequency: entry.frequency,
                    cost,
                });
                edges_by_end[end].push(idx);
            }
        }
    }

    // --- Viterbi forward pass with k-best tracking ---
    //
    // With bigram scoring, state is keyed by (position, prev_thai) so that
    // different previous words at the same position maintain separate paths.
    //
    // best[pos] maps prev_thai → k-best partial paths ending at that position.
    // At position 0, the seed uses the committed context as prev_thai.

    let context_prev: Option<String> = context.last().cloned();
    let beam_limit = params.k * 4; // global beam per position

    let mut best: Vec<HashMap<Option<String>, Vec<PartialPath>>> =
        (0..=n).map(|_| HashMap::new()).collect();
    best[0]
        .entry(context_prev.clone())
        .or_default()
        .push(PartialPath {
            cost: 0.0,
            words: Vec::new(),
            last_thai: context_prev,
        });

    for end in 1..=n {
        // Collect new candidate paths into a temporary map keyed by new prev_thai
        let mut new_states: HashMap<Option<String>, Vec<PartialPath>> = HashMap::new();

        for &edge_idx in &edges_by_end[end] {
            let edge = &all_edges[edge_idx];
            for paths in best[edge.start].values() {
                for path in paths {
                    let unigram_cost = edge.cost;

                    let bigram_bonus = if let Some(ngram_data) = ngram {
                        let score = ngram_data.bigram_score(
                            path.last_thai.as_deref(),
                            &edge.thai,
                            params.alpha,
                        );
                        // Convert probability to cost: -ln(score), clamp to avoid infinity
                        -(score.max(1e-20).ln())
                    } else {
                        0.0
                    };

                    let edge_cost = unigram_cost + params.bigram_weight * bigram_bonus;

                    let mut new_words = path.words.clone();
                    new_words.push(CandidateWord {
                        thai: edge.thai.clone(),
                        frequency: edge.frequency,
                        cost: edge.cost, // store unigram cost per word
                    });

                    let new_prev = Some(edge.thai.clone());
                    new_states
                        .entry(new_prev.clone())
                        .or_default()
                        .push(PartialPath {
                            cost: path.cost + edge_cost,
                            words: new_words,
                            last_thai: new_prev,
                        });
                }
            }
        }

        // Prune: keep top-k per prev_thai state
        for paths in new_states.values_mut() {
            paths.sort_by(|a, b| {
                a.cost
                    .partial_cmp(&b.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            paths.truncate(params.k);
        }

        // Global beam pruning across all prev_thai at this position
        let total: usize = new_states.values().map(|v| v.len()).sum();
        if total > beam_limit {
            // Collect all paths, sort, keep top beam_limit, rebuild map
            let mut all_paths: Vec<PartialPath> = new_states.into_values().flatten().collect();
            all_paths.sort_by(|a, b| {
                a.cost
                    .partial_cmp(&b.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            all_paths.truncate(beam_limit);
            new_states = HashMap::new();
            for path in all_paths {
                new_states
                    .entry(path.last_thai.clone())
                    .or_default()
                    .push(path);
            }
        }

        best[end] = new_states;
    }

    // --- Collect complete paths and deduplicate ---

    let mut all_final_paths: Vec<&PartialPath> = best[n].values().flatten().collect();
    all_final_paths.sort_by(|a, b| {
        a.cost
            .partial_cmp(&b.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut results: Vec<Candidate> = all_final_paths
        .iter()
        .map(|path| {
            let thai: String = path.words.iter().map(|w| w.thai.as_str()).collect();
            let freq_cost: f64 = path
                .words
                .iter()
                .map(|w| -(w.frequency.max(params.min_freq).ln()))
                .sum();
            let seg_penalty = path.words.len() as f64 * params.lambda;
            let bigram_cost = path.cost - freq_cost - seg_penalty;
            Candidate {
                thai,
                words: path.words.clone(),
                score: path.cost,
                freq_cost,
                seg_penalty,
                bigram_cost,
            }
        })
        .collect();

    // Deduplicate by Thai text, keeping the lowest-cost instance.
    // Results are already sorted by cost, so first occurrence wins.
    let mut seen = HashSet::new();
    results.retain(|c| seen.insert(c.thai.clone()));
    results.truncate(params.k);

    RankingResult {
        candidates: results,
        lattice_edges: all_edges,
    }
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

    /// Helper to extract just the candidates from a ranking result.
    fn rank(input: &str, dict: &Dictionary, k: usize) -> Vec<Candidate> {
        let params = RankingParams {
            k,
            ..Default::default()
        };
        rank_candidates(input, dict, None, &[], &params).candidates
    }

    #[test]
    fn test_empty_input() {
        let dict = build_test_dict();
        let results = rank("", &dict, DEFAULT_K);
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_match() {
        let dict = build_test_dict();
        let results = rank("xyz", &dict, DEFAULT_K);
        assert!(results.is_empty());
    }

    #[test]
    fn test_single_word_candidates() {
        let dict = build_test_dict();
        let results = rank("mai", &dict, DEFAULT_K);

        // "mai" maps to 3 words: ไม่ (0.013), ไหม (0.005), ใหม่ (0.004)
        // All are single-word complete paths covering the full input.
        assert_eq!(results.len(), 3);

        // Best candidate should be ไม่ (highest frequency = lowest cost)
        assert_eq!(results[0].thai, "ไม่");
        assert_eq!(results[0].words.len(), 1);
        assert_eq!(results[0].words[0].thai, "ไม่");

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
        let results = rank("mainai", &dict, DEFAULT_K);

        // Possible complete paths (tiling all 6 bytes):
        //   "mai"(3) + "nai"(3) → ไม่ใน, ไหมใน, ใหม่ใน
        //   "ma"(2) + ???       → no word matches "inai" from position 2
        assert_eq!(results.len(), 3);

        // Best: ไม่ใน (ไม่ has highest freq among "mai" words)
        assert_eq!(results[0].thai, "ไม่ใน");
        assert_eq!(results[0].word_count(), 2);
        assert_eq!(results[0].words[0].thai, "ไม่");
        assert_eq!(results[0].words[1].thai, "ใน");
    }

    #[test]
    fn test_single_long_word() {
        let dict = build_test_dict();
        let results = rank("sawatdee", &dict, DEFAULT_K);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].thai, "สวัสดี");
        assert_eq!(results[0].word_count(), 1);
        assert_eq!(results[0].words[0].thai, "สวัสดี");
    }

    #[test]
    fn test_partial_coverage_returns_empty() {
        let dict = build_test_dict();
        // "maix" — "mai" matches positions 0..3, but "x" at position 3 has no match
        let results = rank("maix", &dict, DEFAULT_K);
        assert!(results.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let dict = build_test_dict();
        // "maai" maps only to ไม่ (word 0), so there's one path.
        // "mai" also maps to ไม่. If both variants lead to the same Thai output
        // in a multi-word context, dedup should keep only the best.
        let results = rank("maai", &dict, DEFAULT_K);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].thai, "ไม่");
    }

    #[test]
    fn test_k_limits_results() {
        let dict = build_test_dict();
        // With k=1, only the best candidate should be returned
        let results = rank("mai", &dict, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].thai, "ไม่");
    }

    #[test]
    fn test_score_includes_lambda_penalty() {
        let dict = build_test_dict();

        // Single word "nai" → ใน, freq=0.012
        // Expected cost: -ln(0.012) + 0.5
        let results = rank("nai", &dict, DEFAULT_K);
        assert_eq!(results.len(), 1);

        let expected_cost = -(0.012_f64.ln()) + DEFAULT_LAMBDA;
        assert!((results[0].score - expected_cost).abs() < 1e-10);
    }

    #[test]
    fn test_multi_word_score_is_sum() {
        let dict = build_test_dict();

        // "mainai" → ไม่ใน = cost(ไม่) + cost(ใน)
        let single_mai = rank("mai", &dict, DEFAULT_K);
        let single_nai = rank("nai", &dict, DEFAULT_K);
        let combined = rank("mainai", &dict, DEFAULT_K);

        let mai_cost = single_mai[0].score; // ไม่
        let nai_cost = single_nai[0].score; // ใน
        let combined_cost = combined[0].score; // ไม่ใน

        assert!((combined_cost - (mai_cost + nai_cost)).abs() < 1e-10);
    }

    #[test]
    fn test_score_decomposition() {
        let dict = build_test_dict();
        let results = rank("nai", &dict, DEFAULT_K);
        assert_eq!(results.len(), 1);

        let c = &results[0];
        // freq_cost = -ln(0.012), seg_penalty = 1 * LAMBDA
        let expected_freq = -(0.012_f64.ln());
        assert!((c.freq_cost - expected_freq).abs() < 1e-10);
        assert!((c.seg_penalty - DEFAULT_LAMBDA).abs() < 1e-10);
        assert!((c.score - (c.freq_cost + c.seg_penalty)).abs() < 1e-10);
    }

    #[test]
    fn test_lattice_edges_returned() {
        let dict = build_test_dict();
        let result = rank_candidates("mai", &dict, None, &[], &RankingParams::default());

        // "mai" should produce edges for "ma" (pos 0..2) and "mai" (pos 0..3)
        assert!(!result.lattice_edges.is_empty());

        // Verify edge structure
        let ma_edges: Vec<_> = result
            .lattice_edges
            .iter()
            .filter(|e| e.start == 0 && e.end == 2)
            .collect();
        assert_eq!(ma_edges.len(), 1); // มา
        assert_eq!(ma_edges[0].thai, "มา");

        let mai_edges: Vec<_> = result
            .lattice_edges
            .iter()
            .filter(|e| e.start == 0 && e.end == 3)
            .collect();
        assert_eq!(mai_edges.len(), 3); // ไม่, ไหม, ใหม่
    }

    #[test]
    fn test_candidate_word_details() {
        let dict = build_test_dict();
        let results = rank("mainai", &dict, DEFAULT_K);

        let best = &results[0]; // ไม่ใน
        assert_eq!(best.word_count(), 2);

        // First word: ไม่, freq=0.013
        assert_eq!(best.words[0].thai, "ไม่");
        assert!((best.words[0].frequency - 0.013).abs() < 1e-10);

        // Second word: ใน, freq=0.012
        assert_eq!(best.words[1].thai, "ใน");
        assert!((best.words[1].frequency - 0.012).abs() < 1e-10);
    }
}

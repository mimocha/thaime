// SPDX-License-Identifier: MPL-2.0

//! Tunable parameters and constants for the THAIME engine.
//!
//! All compile-time defaults live here so they are easy to find and adjust
//! in one place. Runtime overrides (e.g. via [`crate::ranking::RankingParams`])
//! still reference these as their `Default` values.

// ---------------------------------------------------------------------------
// Ranking parameters (used by ranking.rs via RankingParams)
// ---------------------------------------------------------------------------

/// Default segmentation penalty per word. Higher = fewer, longer words preferred.
pub const DEFAULT_LAMBDA: f64 = 1.0;

/// Default floor for word frequency to avoid -ln(0).
pub const DEFAULT_MIN_FREQ: f64 = 5e-6;

/// Default number of candidates to track per lattice position.
pub const DEFAULT_K: usize = 10;

/// Default n-gram weight multiplier.
pub const DEFAULT_NGRAM_WEIGHT: f64 = 2.0;

/// Default Stupid Backoff penalty factor.
pub const DEFAULT_ALPHA: f64 = 0.4;

/// Beam multiplier for global pruning. At each lattice position, at most
/// `k * BEAM_MULTIPLIER` partial paths are kept before per-state pruning.
pub const BEAM_MULTIPLIER: usize = 4;

// ---------------------------------------------------------------------------
// Input context parameters (used by context.rs)
// ---------------------------------------------------------------------------

/// Maximum input buffer length (bytes). Safety valve against huge lattices.
pub const MAX_BUFFER_LEN: usize = 50;

/// Maximum context depth (number of previously committed words to track).
pub const MAX_CONTEXT_DEPTH: usize = 2;

// ---------------------------------------------------------------------------
// N-gram parameters (used by ngram.rs)
// ---------------------------------------------------------------------------

/// Default minimum count threshold for trigram filtering.
pub const DEFAULT_TRIGRAM_MIN_COUNT: u64 = 10;

/// Floor probability for unseen words to avoid log(0).
pub const FLOOR_PROB: f64 = 6e-6;

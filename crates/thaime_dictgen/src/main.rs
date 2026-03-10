// SPDX-License-Identifier: MPL-2.0

//! Dictionary generator for THAIME.
//!
//! Reads the JSON dataset produced by the thaime-nlp pipeline and writes
//! two binary files consumed by `thaime_engine`:
//!
//! - `trie.bin`     — serialized yada DoubleArray (romanization keys → group IDs)
//! - `metadata.bin` — bincode-serialized word metadata + posting lists

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{env, fs, process};
use yada::builder::DoubleArrayBuilder;

// ---------------------------------------------------------------------------
// JSON dataset types (input from thaime-nlp)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Dataset {
    metadata: DatasetMeta,
    entries: Vec<DatasetEntry>,
}

#[derive(Deserialize)]
struct DatasetMeta {
    version: String,
    vocab_size: usize,
    #[allow(dead_code)]
    total_romanization_keys: usize,
    unique_romanization_keys: usize,
}

#[derive(Deserialize)]
struct DatasetEntry {
    word_id: u32,
    thai: String,
    frequency: f64,
    romanizations: Vec<String>,
}

// ---------------------------------------------------------------------------
// Binary output types (must match thaime_engine::trie::DictData)
// ---------------------------------------------------------------------------

/// Metadata for a single Thai word, indexed by word_id.
#[derive(serde::Serialize)]
struct WordMeta {
    thai: String,
    frequency: f64,
}

/// Complete dictionary data: word metadata + CSR posting lists.
///
/// The trie maps `romanization_key → group_id (u32)`.
/// The posting lists map `group_id → [word_id, ...]` via CSR encoding:
///   word_ids for group `g` = group_word_ids[group_offsets[g]..group_offsets[g+1]]
#[derive(serde::Serialize)]
struct DictData {
    words: Vec<WordMeta>,
    group_offsets: Vec<u32>,
    group_word_ids: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: thaime_dictgen <input.json> [output_dir]");
        eprintln!();
        eprintln!("  input.json   Path to trie_dataset.json from thaime-nlp");
        eprintln!("  output_dir   Directory for output files (default: data/dict)");
        process::exit(1);
    }

    let input_path = &args[1];
    let output_dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/dict"));

    // --- Parse JSON dataset ---

    eprintln!("Reading {}...", input_path);
    let json_str = fs::read_to_string(input_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", input_path, e);
        process::exit(1);
    });

    let dataset: Dataset = serde_json::from_str(&json_str).unwrap_or_else(|e| {
        eprintln!("Failed to parse JSON: {}", e);
        process::exit(1);
    });

    eprintln!(
        "Dataset v{}: {} words, {} unique romanization keys",
        dataset.metadata.version,
        dataset.metadata.vocab_size,
        dataset.metadata.unique_romanization_keys,
    );

    // --- Build word metadata + romanization map ---

    let mut words: Vec<WordMeta> = Vec::with_capacity(dataset.entries.len());
    let mut roman_to_word_ids: BTreeMap<String, Vec<u32>> = BTreeMap::new();

    for entry in &dataset.entries {
        assert_eq!(
            entry.word_id as usize,
            words.len(),
            "word_ids must be sequential starting from 0 (got {} at index {})",
            entry.word_id,
            words.len(),
        );

        words.push(WordMeta {
            thai: entry.thai.clone(),
            frequency: entry.frequency,
        });

        for roman in &entry.romanizations {
            roman_to_word_ids
                .entry(roman.clone())
                .or_default()
                .push(entry.word_id);
        }
    }

    // --- Build CSR posting lists + yada keyset ---
    //
    // BTreeMap iterates in sorted key order, which is exactly what yada requires.
    // We collect into a Vec first so the String keys live long enough to borrow.

    let sorted_entries: Vec<(String, Vec<u32>)> = roman_to_word_ids.into_iter().collect();

    let mut group_offsets: Vec<u32> = Vec::with_capacity(sorted_entries.len() + 1);
    let mut group_word_ids: Vec<u32> = Vec::new();
    let mut keyset: Vec<(&[u8], u32)> = Vec::with_capacity(sorted_entries.len());

    for (i, (key, word_ids)) in sorted_entries.iter().enumerate() {
        group_offsets.push(group_word_ids.len() as u32);
        group_word_ids.extend(word_ids);
        keyset.push((key.as_bytes(), i as u32));
    }
    group_offsets.push(group_word_ids.len() as u32); // sentinel

    eprintln!(
        "Built {} posting groups, {} total postings",
        keyset.len(),
        group_word_ids.len(),
    );

    // --- Build yada double-array trie ---

    let trie_bytes = DoubleArrayBuilder::build(&keyset).unwrap_or_else(|| {
        eprintln!("Failed to build double-array trie (yada returned None)");
        process::exit(1);
    });

    eprintln!(
        "Trie size: {} bytes ({:.1} MB)",
        trie_bytes.len(),
        trie_bytes.len() as f64 / 1_048_576.0,
    );

    // --- Serialize metadata ---

    let dict_data = DictData {
        words,
        group_offsets,
        group_word_ids,
    };

    let metadata_bytes = bincode::serialize(&dict_data).unwrap_or_else(|e| {
        eprintln!("Failed to serialize metadata: {}", e);
        process::exit(1);
    });

    eprintln!(
        "Metadata size: {} bytes ({:.1} KB)",
        metadata_bytes.len(),
        metadata_bytes.len() as f64 / 1024.0,
    );

    // --- Write output files ---

    fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        eprintln!("Failed to create {}: {}", output_dir.display(), e);
        process::exit(1);
    });

    let trie_path = output_dir.join("trie.bin");
    fs::write(&trie_path, &trie_bytes).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", trie_path.display(), e);
        process::exit(1);
    });

    let meta_path = output_dir.join("metadata.bin");
    fs::write(&meta_path, &metadata_bytes).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", meta_path.display(), e);
        process::exit(1);
    });

    eprintln!("Dictionary written to {}/", output_dir.display());
}

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

    // Parse arguments: <input.json> [output_dir] [--version-tag VTAG]
    let mut positional: Vec<String> = Vec::new();
    let mut version_tag: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--version-tag" {
            if i + 1 < args.len() {
                version_tag = Some(args[i + 1].clone());
                i += 2;
                continue;
            } else {
                eprintln!("Error: --version-tag requires a value");
                process::exit(1);
            }
        }
        positional.push(args[i].clone());
        i += 1;
    }

    if positional.is_empty() || positional.len() > 2 {
        eprintln!("Usage: thaime_dictgen <input.json> [output_dir] [--version-tag VTAG]");
        eprintln!();
        eprintln!("  input.json      Path to trie_dataset.json from thaime-nlp");
        eprintln!("  output_dir      Directory for output files (default: data/dict)");
        eprintln!("  --version-tag   Version tag for output filenames (e.g. v1_0_0)");
        process::exit(1);
    }

    let input_path = &positional[0];
    let output_dir = positional
        .get(1)
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

    // Output filenames: versioned (e.g. trie-v1_0_0.bin) or unversioned (trie.bin)
    let trie_name = match &version_tag {
        Some(vt) => format!("trie-{}.bin", vt),
        None => "trie.bin".to_string(),
    };
    let meta_name = match &version_tag {
        Some(vt) => format!("metadata-{}.bin", vt),
        None => "metadata.bin".to_string(),
    };
    let combined_name = match &version_tag {
        Some(vt) => format!("thaime-{}.dict", vt),
        None => "thaime.dict".to_string(),
    };

    let trie_path = output_dir.join(&trie_name);
    fs::write(&trie_path, &trie_bytes).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", trie_path.display(), e);
        process::exit(1);
    });

    let meta_path = output_dir.join(&meta_name);
    fs::write(&meta_path, &metadata_bytes).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", meta_path.display(), e);
        process::exit(1);
    });

    // --- Write combined blob (thaime.dict) for WASM / web demo ---
    //
    // Format: [4 bytes: trie_len as u32 LE][trie_bytes][metadata_bytes]
    // The WASM wrapper splits this blob at load time.

    let trie_len = trie_bytes.len() as u32;
    let mut combined = Vec::with_capacity(4 + trie_bytes.len() + metadata_bytes.len());
    combined.extend_from_slice(&trie_len.to_le_bytes());
    combined.extend_from_slice(&trie_bytes);
    combined.extend_from_slice(&metadata_bytes);

    let combined_path = output_dir.join(&combined_name);
    fs::write(&combined_path, &combined).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", combined_path.display(), e);
        process::exit(1);
    });

    eprintln!(
        "Combined blob: {} bytes ({:.1} MB)",
        combined.len(),
        combined.len() as f64 / 1_048_576.0,
    );

    eprintln!("Dictionary written to {}/", output_dir.display());
}

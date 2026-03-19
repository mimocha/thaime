// SPDX-License-Identifier: MPL-2.0

//! Trie-based dictionary for romanization → Thai word lookup.
//!
//! The dictionary consists of two parts:
//! - A yada double-array trie mapping romanization keys to posting group IDs
//! - A metadata table mapping word IDs to Thai text and frequency
//!
//! The posting lists (CSR format) connect trie matches to word entries:
//!   trie: romanization_key → group_id
//!   postings: group_id → [word_id, ...]
//!   metadata: word_id → { thai, frequency }

use serde::{Deserialize, Serialize};
use yada::DoubleArray;

// ---------------------------------------------------------------------------
// Binary data types (must match thaime_dictgen output)
// ---------------------------------------------------------------------------

/// Metadata for a single Thai word, indexed by word_id.
#[derive(Serialize, Deserialize)]
struct WordMeta {
    thai: String,
    frequency: f64,
}

/// Complete dictionary data: word metadata + CSR posting lists.
#[derive(Serialize, Deserialize)]
struct DictData {
    words: Vec<WordMeta>,
    group_offsets: Vec<u32>,
    group_word_ids: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A word entry returned from a dictionary lookup.
#[derive(Debug, Clone)]
pub struct WordEntry {
    pub word_id: u32,
    pub thai: String,
    pub frequency: f64,
}

/// A single prefix match: a romanization key that matched a prefix of the input,
/// along with all the Thai words it maps to.
#[derive(Debug, Clone)]
pub struct PrefixMatch {
    /// Number of bytes consumed from the input string.
    pub prefix_len: usize,
    /// Thai words that this romanization maps to.
    pub entries: Vec<WordEntry>,
}

// ---------------------------------------------------------------------------
// Dictionary
// ---------------------------------------------------------------------------

/// The loaded dictionary: trie + metadata.
///
/// Created once at engine startup and shared across input contexts.
pub struct Dictionary {
    trie: DoubleArray<Vec<u8>>,
    data: DictData,
}

impl Dictionary {
    /// Load the dictionary from compile-time embedded binary data.
    ///
    /// The binary files are resolved by `build.rs`, which prefers versioned
    /// names (e.g. `trie-v0_4_2.bin`) and falls back to unversioned names.
    /// This method is only available when the `embed-dict` feature is enabled
    /// (the default).
    #[cfg(feature = "embed-dict")]
    pub fn from_embedded() -> Self {
        static TRIE_BYTES: &[u8] = include_bytes!(env!("THAIME_TRIE_PATH"));
        static METADATA_BYTES: &[u8] = include_bytes!(env!("THAIME_METADATA_PATH"));

        let trie = DoubleArray::new(TRIE_BYTES.to_vec());
        let data: DictData = bincode::deserialize(METADATA_BYTES)
            .expect("Failed to deserialize embedded dictionary metadata");
        Self { trie, data }
    }

    /// Build a dictionary from pre-built components.
    ///
    /// `trie_bytes` is the raw yada DoubleArray data.
    /// `metadata_bytes` is the bincode-serialized DictData.
    pub fn from_bytes(trie_bytes: Vec<u8>, metadata_bytes: &[u8]) -> Self {
        let trie = DoubleArray::new(trie_bytes);
        let data: DictData = bincode::deserialize(metadata_bytes)
            .expect("Failed to deserialize dictionary metadata");
        Self { trie, data }
    }

    /// Number of Thai words in the dictionary.
    pub fn word_count(&self) -> usize {
        self.data.words.len()
    }

    /// Find all romanization keys that are prefixes of `input`.
    ///
    /// For example, if the input is `"mainai"` and the dictionary contains
    /// romanization keys `"mai"` and `"ma"`, both will be returned with their
    /// respective prefix lengths (3 and 2) and associated Thai word entries.
    ///
    /// Results are ordered by prefix length (shortest first), matching the
    /// iteration order of yada's `common_prefix_search`.
    pub fn prefix_search(&self, input: &str) -> Vec<PrefixMatch> {
        let mut matches = Vec::new();

        for (group_id, prefix_len) in self.trie.common_prefix_search(input.as_bytes()) {
            let gid = group_id as usize;
            let start = self.data.group_offsets[gid] as usize;
            let end = self.data.group_offsets[gid + 1] as usize;

            let entries: Vec<WordEntry> = self.data.group_word_ids[start..end]
                .iter()
                .map(|&wid| {
                    let meta = &self.data.words[wid as usize];
                    WordEntry {
                        word_id: wid,
                        thai: meta.thai.clone(),
                        frequency: meta.frequency,
                    }
                })
                .collect();

            matches.push(PrefixMatch {
                prefix_len,
                entries,
            });
        }

        matches
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use yada::builder::DoubleArrayBuilder;

    /// Build a small test dictionary from inline data.
    ///
    /// Test vocabulary:
    ///   word_id 0: ไม่   freq=0.013  romanizations: ["mai", "maai"]
    ///   word_id 1: ใน   freq=0.012  romanizations: ["nai"]
    ///   word_id 2: ไหม  freq=0.005  romanizations: ["mai"]  (collision with word 0)
    ///   word_id 3: สวัสดี freq=0.003  romanizations: ["sawatdee"]
    ///   word_id 4: ใหม่  freq=0.004  romanizations: ["mai"]  (collision with word 0, 2)
    ///   word_id 5: มา   freq=0.008  romanizations: ["ma", "maa"]
    pub(crate) fn build_test_dict() -> Dictionary {
        let words = vec![
            WordMeta {
                thai: "ไม่".to_string(),
                frequency: 0.013,
            },
            WordMeta {
                thai: "ใน".to_string(),
                frequency: 0.012,
            },
            WordMeta {
                thai: "ไหม".to_string(),
                frequency: 0.005,
            },
            WordMeta {
                thai: "สวัสดี".to_string(),
                frequency: 0.003,
            },
            WordMeta {
                thai: "ใหม่".to_string(),
                frequency: 0.004,
            },
            WordMeta {
                thai: "มา".to_string(),
                frequency: 0.008,
            },
        ];

        // Romanization → word_ids mapping (must be sorted by key for yada)
        // "ma"       → [5]
        // "maa"      → [5]
        // "maai"     → [0]
        // "mai"      → [0, 2, 4]
        // "nai"      → [1]
        // "sawatdee" → [3]
        let groups: Vec<(&str, Vec<u32>)> = vec![
            ("ma", vec![5]),
            ("maa", vec![5]),
            ("maai", vec![0]),
            ("mai", vec![0, 2, 4]),
            ("nai", vec![1]),
            ("sawatdee", vec![3]),
        ];

        let mut group_offsets: Vec<u32> = Vec::new();
        let mut group_word_ids: Vec<u32> = Vec::new();
        let mut keyset: Vec<(&[u8], u32)> = Vec::new();

        for (i, (key, wids)) in groups.iter().enumerate() {
            group_offsets.push(group_word_ids.len() as u32);
            group_word_ids.extend(wids);
            keyset.push((key.as_bytes(), i as u32));
        }
        group_offsets.push(group_word_ids.len() as u32);

        let trie_bytes = DoubleArrayBuilder::build(&keyset).expect("Failed to build test trie");

        let data = DictData {
            words,
            group_offsets,
            group_word_ids,
        };
        let metadata_bytes = bincode::serialize(&data).expect("Failed to serialize test metadata");

        Dictionary::from_bytes(trie_bytes, &metadata_bytes)
    }

    #[test]
    fn test_word_count() {
        let dict = build_test_dict();
        assert_eq!(dict.word_count(), 6);
    }

    #[test]
    fn test_exact_single_match() {
        let dict = build_test_dict();
        let matches = dict.prefix_search("nai");

        // "nai" matches exactly one group with one word
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].prefix_len, 3);
        assert_eq!(matches[0].entries.len(), 1);
        assert_eq!(matches[0].entries[0].thai, "ใน");
        assert_eq!(matches[0].entries[0].word_id, 1);
    }

    #[test]
    fn test_collision_multiple_words() {
        let dict = build_test_dict();
        let matches = dict.prefix_search("mai");

        // "ma" matches at prefix_len=2, "mai" matches at prefix_len=3
        assert_eq!(matches.len(), 2);

        // First match: "ma" → word 5 (มา)
        assert_eq!(matches[0].prefix_len, 2);
        assert_eq!(matches[0].entries.len(), 1);
        assert_eq!(matches[0].entries[0].thai, "มา");

        // Second match: "mai" → words 0, 2, 4 (ไม่, ไหม, ใหม่)
        assert_eq!(matches[1].prefix_len, 3);
        assert_eq!(matches[1].entries.len(), 3);

        let thais: Vec<&str> = matches[1].entries.iter().map(|e| e.thai.as_str()).collect();
        assert!(thais.contains(&"ไม่"));
        assert!(thais.contains(&"ไหม"));
        assert!(thais.contains(&"ใหม่"));
    }

    #[test]
    fn test_prefix_search_longer_input() {
        let dict = build_test_dict();

        // "mainai" should match prefixes "ma" (len 2) and "mai" (len 3)
        // It should NOT match "nai" because that's not a prefix of "mainai"
        let matches = dict.prefix_search("mainai");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].prefix_len, 2); // "ma"
        assert_eq!(matches[1].prefix_len, 3); // "mai"
    }

    #[test]
    fn test_prefix_search_from_offset() {
        let dict = build_test_dict();

        // Simulate lattice construction: search from position 3 in "mainai"
        let matches = dict.prefix_search(&"mainai"[3..]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].prefix_len, 3); // "nai"
        assert_eq!(matches[0].entries[0].thai, "ใน");
    }

    #[test]
    fn test_no_match() {
        let dict = build_test_dict();
        let matches = dict.prefix_search("xyz");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_empty_input() {
        let dict = build_test_dict();
        let matches = dict.prefix_search("");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_long_key_match() {
        let dict = build_test_dict();
        let matches = dict.prefix_search("sawatdee");

        // "sawatdee" is an exact match
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].prefix_len, 8);
        assert_eq!(matches[0].entries[0].thai, "สวัสดี");
    }

    #[test]
    fn test_long_key_as_prefix() {
        let dict = build_test_dict();

        // "sawatdeemai" should match "sawatdee" as a prefix
        let matches = dict.prefix_search("sawatdeemai");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].prefix_len, 8); // "sawatdee"
    }
}

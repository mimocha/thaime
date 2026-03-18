// SPDX-License-Identifier: MPL-2.0

//! THAIME CLI - Interactive test harness for the engine.
//!
//! Type Latin characters and see the Thai candidates the engine produces.
//! This is the primary feedback loop until the IBus frontend exists.
//!
//! ## Usage
//!
//! - Type Latin characters (a-z) to build the input buffer and see candidates
//! - Enter a number (1-9) to commit that candidate
//! - Press Enter with no number to commit candidate 1 (top result)
//! - `:b` to backspace (remove last character)
//! - `:r` to reset (clear buffer without committing)
//! - `:cc` to clear committed context
//! - `:q` or Ctrl+D to quit
//!
//! ## N-gram loading
//!
//! Pass `--ngram-bin <path>` to load a pre-built binary n-gram file.
//! Pass `--ngram-dir <path>` to load raw TSV count files (dev/debug).
//! Without flags, auto-discovers binary files in `data/input/`, falling
//! back to TSV files if no binary is found.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use thaime_engine::context::InputContext;
use thaime_engine::ngram::NgramData;
use thaime_engine::trie::Dictionary;

fn main() {
    let args = parse_args();
    let dict = Dictionary::from_embedded();

    let ngram = load_ngram(&args);

    let mut ctx = match ngram {
        Some(ng) => InputContext::with_ngram(dict, ng),
        None => {
            println!("  (no n-gram data loaded — unigram-only mode)");
            InputContext::new(dict)
        }
    };

    println!("THAIME CLI v{}", env!("CARGO_PKG_VERSION"));
    println!("Commands: :q quit, :r reset, :b backspace, :cc clear context\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Show context and preedit
        let context = ctx.committed_context();
        let context_display = if context.is_empty() {
            "<BOS>".to_string()
        } else {
            format!("[{}]", context.join(", "))
        };

        let preedit = ctx.preedit();
        if preedit.is_empty() {
            print!("ctx:{} > ", context_display);
        } else {
            print!("ctx:{} [{}] > ", context_display, preedit);
        }
        stdout.flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF (Ctrl+D)
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() {
                    // Enter with no input: commit top candidate
                    if !ctx.candidates().is_empty() {
                        if let Some(thai) = ctx.commit(0) {
                            println!("  -> {}", thai);
                        }
                    }
                    continue;
                }

                match input {
                    ":q" => break,
                    ":r" => {
                        ctx.reset();
                        println!("  (reset)");
                        continue;
                    }
                    ":b" => {
                        if ctx.pop_key() {
                            display_candidates(&ctx);
                        }
                        continue;
                    }
                    ":cc" => {
                        ctx.clear_context();
                        println!("  (context cleared)");
                        continue;
                    }
                    _ => {}
                }

                // Check if input is a number (candidate selection)
                if let Ok(n) = input.parse::<usize>() {
                    if n >= 1 {
                        if let Some(thai) = ctx.commit(n - 1) {
                            println!("  -> {}", thai);
                        } else {
                            println!("  (invalid selection)");
                        }
                    }
                    continue;
                }

                // Otherwise, feed characters into the engine
                for ch in input.chars() {
                    ctx.push_key(ch);
                }
                display_candidates(&ctx);
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    println!("\nBye!");
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

struct CliArgs {
    ngram_bin: Option<PathBuf>,
    ngram_dir: Option<PathBuf>,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut ngram_bin = None;
    let mut ngram_dir = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--ngram-bin" => {
                if let Some(path) = args.get(i + 1) {
                    ngram_bin = Some(PathBuf::from(path));
                    i += 1;
                }
            }
            "--ngram-dir" => {
                if let Some(path) = args.get(i + 1) {
                    ngram_dir = Some(PathBuf::from(path));
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    CliArgs {
        ngram_bin,
        ngram_dir,
    }
}

// ---------------------------------------------------------------------------
// N-gram loading (binary preferred, TSV fallback)
// ---------------------------------------------------------------------------

fn load_ngram(args: &CliArgs) -> Option<NgramData> {
    // Explicit --ngram-bin takes priority
    if let Some(path) = &args.ngram_bin {
        return load_ngram_binary(path);
    }

    // Explicit --ngram-dir for TSV
    if let Some(dir) = &args.ngram_dir {
        return load_ngram_tsv(dir);
    }

    // Auto-discover: try binary first, then TSV
    let input_dir = PathBuf::from("data/input");
    if input_dir.is_dir() {
        if let Some(ng) = auto_discover_binary(&input_dir) {
            return Some(ng);
        }
        return load_ngram_tsv(&input_dir);
    }

    None
}

/// Auto-discover the best binary n-gram file in a directory.
fn auto_discover_binary(dir: &Path) -> Option<NgramData> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with("thaime_ngram_v1_mc") && n.ends_with(".bin"))
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();
    candidates.sort();
    // Prefer highest min_count (last alphabetically)
    let path = candidates.last()?;
    load_ngram_binary(path)
}

/// Load n-gram data from a v1 binary file.
fn load_ngram_binary(path: &Path) -> Option<NgramData> {
    print!("Loading n-gram binary from {} ... ", path.display());
    io::stdout().flush().unwrap();

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            println!("failed: {}", e);
            return None;
        }
    };

    match NgramData::from_bytes(&data) {
        Ok(ng) => {
            println!(
                "done ({} unigrams, {} bigrams, {} trigrams, alpha={:.2})",
                ng.unigram_count(),
                ng.bigram_count(),
                ng.trigram_count(),
                ng.alpha(),
            );
            Some(ng)
        }
        Err(e) => {
            println!("failed: {}", e);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// TSV loading (local dev helper — parsing lives here, not in engine)
// ---------------------------------------------------------------------------

/// Load n-gram data from raw TSV count files in a directory.
fn load_ngram_tsv(dir: &Path) -> Option<NgramData> {
    let unigram_path = dir.join("ngrams_1_merged_raw.tsv");
    let bigram_path = dir.join("ngrams_2_merged_raw.tsv");
    let trigram_path = dir.join("ngrams_3_merged_raw.tsv");

    if !unigram_path.exists() || !bigram_path.exists() {
        eprintln!("Warning: n-gram TSV files not found in {}", dir.display());
        return None;
    }

    print!("Loading n-gram TSV data from {} ... ", dir.display());
    io::stdout().flush().unwrap();

    let unigram_counts = match load_tsv_unigrams(&unigram_path) {
        Ok(c) => c,
        Err(e) => {
            println!("failed: {}", e);
            return None;
        }
    };
    let bigram_counts = match load_tsv_bigrams(&bigram_path) {
        Ok(c) => c,
        Err(e) => {
            println!("failed: {}", e);
            return None;
        }
    };
    let trigram_counts = if trigram_path.exists() {
        match load_tsv_trigrams(
            &trigram_path,
            thaime_engine::config::DEFAULT_TRIGRAM_MIN_COUNT,
        ) {
            Ok(c) => c,
            Err(e) => {
                println!("trigram load failed (continuing without): {}", e);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    let ng = NgramData::from_raw(unigram_counts, bigram_counts, trigram_counts);
    println!(
        "done ({} unigrams, {} bigrams, {} trigrams)",
        ng.unigram_count(),
        ng.bigram_count(),
        ng.trigram_count(),
    );
    Some(ng)
}

fn load_tsv_unigrams(path: &Path) -> io::Result<HashMap<String, u64>> {
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
        counts.insert(word, count);
    }
    Ok(counts)
}

fn load_tsv_bigrams(path: &Path) -> io::Result<HashMap<(String, String), u64>> {
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
        counts.insert((w1, w2), count);
    }
    Ok(counts)
}

fn load_tsv_trigrams(
    path: &Path,
    min_count: u64,
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
        counts.insert((w1, w2, w3), count);
    }
    Ok(counts)
}

fn display_candidates(ctx: &InputContext) {
    let candidates = ctx.candidates();
    if candidates.is_empty() {
        if !ctx.preedit().is_empty() {
            println!("  (no candidates)");
        }
        return;
    }

    // Header
    println!(
        "  {:>2}  {:16} {:>7} {:>7} {:>7} {:>7}  {:>5}",
        "#", "Thai", "Total", "Freq", "Ngram", "SegPen", "Words"
    );

    for (i, c) in candidates.iter().enumerate() {
        println!(
            "  {:>2}  {:16} {:>7.2} {:>7.2} {:>7.2} {:>7.2}  {:>5}",
            i + 1,
            c.thai,
            c.score,
            c.freq_cost,
            c.ngram_cost,
            c.seg_penalty,
            c.word_count(),
        );
    }
}

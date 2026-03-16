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
//! Pass `--ngram-dir <path>` to load bigram data for context-dependent ranking.
//! Falls back to `data/input/` relative to the workspace root if the directory
//! exists and the flag is not provided.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use thaime_engine::context::InputContext;
use thaime_engine::ngram::NgramData;
use thaime_engine::trie::Dictionary;

fn main() {
    let ngram_dir = parse_ngram_dir();
    let dict = Dictionary::from_embedded();

    let ngram = ngram_dir.and_then(|dir| load_ngram_data(&dir));

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

/// Parse `--ngram-dir <path>` from CLI args, falling back to conventional path.
fn parse_ngram_dir() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--ngram-dir" {
            if let Some(path) = args.get(i + 1) {
                let p = PathBuf::from(path);
                if p.is_dir() {
                    return Some(p);
                } else {
                    eprintln!("Warning: --ngram-dir path does not exist: {}", path);
                    return None;
                }
            }
        }
    }
    // Fallback: try conventional path relative to workspace root
    let conventional = PathBuf::from("data/input");
    if conventional.is_dir() {
        Some(conventional)
    } else {
        None
    }
}

/// Load n-gram data from TSV files in the given directory.
fn load_ngram_data(dir: &std::path::Path) -> Option<NgramData> {
    let unigram_path = dir.join("ngrams_1_merged_raw.tsv");
    let bigram_path = dir.join("ngrams_2_merged_raw.tsv");
    let trigram_path = dir.join("ngrams_3_merged_raw.tsv");

    if !unigram_path.exists() || !bigram_path.exists() {
        eprintln!("Warning: n-gram files not found in {}", dir.display());
        return None;
    }

    let trigram_arg = if trigram_path.exists() {
        Some(trigram_path.as_path())
    } else {
        None
    };

    print!("Loading n-gram data from {} ... ", dir.display());
    io::stdout().flush().unwrap();

    match NgramData::from_tsv_files(
        &unigram_path,
        &bigram_path,
        trigram_arg,
        thaime_engine::ngram::DEFAULT_TRIGRAM_MIN_COUNT,
        None,
    ) {
        Ok(ng) => {
            println!(
                "done ({} unigrams, {} bigrams, {} trigrams)",
                ng.unigram_count(),
                ng.bigram_count(),
                ng.trigram_count(),
            );
            Some(ng)
        }
        Err(e) => {
            println!("failed: {}", e);
            None
        }
    }
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

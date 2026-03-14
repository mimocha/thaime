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
//! - `:q` or Ctrl+D to quit

use std::io::{self, BufRead, Write};

use thaime_engine::context::InputContext;
use thaime_engine::trie::Dictionary;

fn main() {
    let dict = Dictionary::from_embedded();
    let mut ctx = InputContext::new(dict);

    println!("THAIME CLI v{}", env!("CARGO_PKG_VERSION"));
    println!(
        "Type Latin characters to see Thai candidates. Commands: :q quit, :r reset, :b backspace\n"
    );

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Show preedit if non-empty
        let preedit = ctx.preedit();
        if preedit.is_empty() {
            print!("> ");
        } else {
            print!("[{}] > ", preedit);
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

fn display_candidates(ctx: &InputContext) {
    let candidates = ctx.candidates();
    if candidates.is_empty() {
        if !ctx.preedit().is_empty() {
            println!("  (no candidates)");
        }
        return;
    }

    for (i, c) in candidates.iter().enumerate() {
        println!("  {}. {:16} (score: {:.2})", i + 1, c.thai, c.score);
    }
}

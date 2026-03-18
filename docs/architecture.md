# Architecture

This document describes the high-level architecture of THAIME: how the components fit together, how data flows through the system, and why the design is the way it is.

## Overview

THAIME is a **Latin-to-Thai input method engine**. It lets users type Thai on a standard QWERTY keyboard using romanized keystrokes — similar to Pinyin input for Chinese or Romaji input for Japanese.

The core problem: given a Latin keystroke sequence like `sawatdee`, find the best Thai interpretation (`สวัสดี`). This requires a dictionary lookup, word segmentation (since Thai has no spaces), and a language model to disambiguate candidates.

## Data Flow

```
Keystroke (Latin char)
    │
    ▼
┌──────────────────┐
│  Input Context   │  context.rs — accumulates keystrokes, manages state
│  (buffer: "mai") │
└────────┬─────────┘
         │  on every keystroke: re-run full ranking pipeline
         ▼
┌──────────────────┐
│  Trie Lookup     │  trie.rs — prefix search on double-array trie
│   "mai" → [ไม่,   │  returns all romanization prefixes that match
│    ไหม, ใหม่, มา] │
└────────┬─────────┘
         │  prefix matches at every position → lattice edges
         ▼
┌──────────────────┐
│  Word Lattice    │  ranking.rs — DAG of all possible word spans
│  (edges by end   │  each edge: [start, end) → thai, freq, cost
│   position)      │
└────────┬─────────┘
         │  Viterbi forward pass with k-best tracking
         ▼
┌──────────────────┐
│  Viterbi DP      │  ranking.rs — scores paths through the lattice
│  + N-gram LM     │  ngram.rs — Stupid Backoff trigram scoring
└────────┬─────────┘
         │  deduplicate, sort by cost
         ▼
┌──────────────────┐
│  Ranked Candidates│  Top-k Thai interpretations, best first
│  1. ไม่  (4.33)   │
│  2. ไหม  (5.30)   │
│  3. ใหม่ (5.52)   │
└────────┬─────────┘
         │  user selects candidate
         ▼
┌──────────────────┐
│   Commit          │  context.rs — commits Thai text, updates context
│   context: [ไม่]  │  last 2 words retained for trigram window
└──────────────────┘
```

## Two-Repo Architecture

THAIME is split across two repositories:

| Repository | Purpose | Outputs |
|------------|---------|---------|
| **[thaime](https://github.com/mimocha/thaime)** | Engine, frontends, tools | `libthaime_engine.so`, CLI, TUI, WASM, web demo |
| **[thaime-nlp](https://github.com/mimocha/thaime-nlp)** | NLP research, data pipelines | `trie_dataset.json`, `thaime_ngram_v1_mc*.bin` |

The NLP repo handles corpus processing, romanization variant generation, and n-gram count extraction. Its outputs are copied into this repo's `data/` directory and consumed by the engine at compile time or runtime.

```
thaime-nlp                              thaime
──────────                              ──────
corpus processing                       data/input/trie_dataset.json
    │                                       │
    ├─ trie pipeline ──────────────────►    ├─ thaime_dictgen ──► data/dict/*.bin
    │                                       │                         │
    └─ ngram pipeline ─────────────────►    └─ data/input/*.bin       │
                                                    │                 │
                                              ┌─────┴─────────────────┘
                                              ▼
                                         thaime_engine
                                         (include_bytes! / runtime load)
```

## Workspace Layout

```
thaime/
├── crates/
│   ├── thaime_engine/     Core library (lib + cdylib + staticlib)
│   ├── thaime_cli/        Interactive CLI test harness
│   ├── thaime_tui/        Ratatui-based visual debugger
│   ├── thaime_dictgen/    Dictionary compiler (JSON → binary trie)
│   └── thaime_wasm/       wasm-bindgen wrapper for browser
├── web/                   React + TypeScript web demo
├── frontends/
│   └── ibus/              IBus engine frontend (planned)
├── data/
│   ├── dict/              Compiled dictionary binaries (versioned)
│   └── input/             Source JSON + n-gram binaries
├── build.sh               Full build pipeline script
└── tests/                 Regression test data (TOML)
```

### Crate Responsibilities

| Crate | Type | Description |
|-------|------|-------------|
| `thaime_engine` | lib + cdylib + staticlib | All core algorithms: trie, lattice, Viterbi, n-gram scoring. Produces the shared library consumed by frontends. |
| `thaime_cli` | binary | Interactive REPL for testing. Shows candidates with score breakdown (Total, Freq, Ngram, SegPen, Words). Supports binary and TSV n-gram loading. |
| `thaime_tui` | binary | Ratatui-based visual debugger with four modes: Main (candidate exploration with live parameter tuning), Lattice (word lattice visualization), Inspector (trie explorer with "why not?" diagnosis), Regression (run/view test results). |
| `thaime_dictgen` | binary | Compiles `trie_dataset.json` into binary trie + metadata files. Produces the combined `.dict` blob for WASM. |
| `thaime_wasm` | cdylib (WASM) | Thin wasm-bindgen wrapper. Exposes `WasmEngine` class to JavaScript with `push_key()`, `candidates()`, `commit()`, and `load_ngram()`. |

## Engine Modules

```
thaime_engine/src/
├── lib.rs          ThaiMeEngine struct + C ABI exports
├── context.rs      InputContext state machine
│                       │
│                       ▼ calls on every keystroke
├── ranking.rs      rank_candidates() — lattice + Viterbi DP
│                       │
│                       ├─ reads ──► trie.rs (prefix search)
│                       └─ reads ──► ngram.rs (trigram scoring)
│
├── trie.rs         Dictionary: double-array trie + metadata
├── ngram.rs        NgramData: Stupid Backoff scoring + binary parser
├── config.rs       All tunable parameters and constants
├── keymap.rs       Latin → Thai character mapping (planned)
└── validate.rs     Thai sequence validation / WTT 2.0 (planned)
```

### Module Dependencies

- **`context.rs`** → `ranking.rs`, `trie.rs`, `ngram.rs`, `config.rs`
- **`ranking.rs`** → `trie.rs`, `ngram.rs`, `config.rs`
- **`trie.rs`** — standalone (depends on `yada`, `bincode`, `serde`)
- **`ngram.rs`** → `config.rs`
- **`config.rs`** — standalone (constants only)

## C ABI Design

The engine exposes a C-compatible API for consumption by input method frameworks (IBus, Fcitx5). This follows the same pattern used by [kime](https://github.com/Riey/kime) (Korean IME) and [librime](https://github.com/rime/librime) (Chinese IME).

### Why C ABI?

- C ABI is the universal FFI boundary — every language can call it
- IBus and Fcitx5 both support C/C++ engine modules
- Decouples the Rust internals from the framework-specific glue code
- Allows the engine to be consumed from Python, Go, etc. for testing

### Opaque Pointer Pattern

The C API uses an opaque struct pointer. Callers never see the struct layout — they get a `ThaiMeEngine*` and call functions on it:

```c
// Lifecycle
ThaiMeEngine* engine = thaime_engine_new();    // create
thaime_process_key(engine, keyval, 0, 0);      // use
thaime_reset(engine);                           // reset
thaime_clear_context(engine);                   // clear context
thaime_engine_free(engine);                     // destroy
```

### Header Generation

The C header (`target/thaime.h`) is auto-generated by [cbindgen](https://github.com/mozilla/cbindgen) during `cargo build`. The build script (`crates/thaime_engine/build.rs`) invokes cbindgen, which scans for `#[no_mangle]` + `extern "C"` functions and produces the header.

Configuration: `cbindgen.toml` at the workspace root.

## WASM Architecture

The `thaime_wasm` crate wraps the engine for browser use via [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/):

```
Browser (JS)                          WASM
──────────────                        ────
fetch("thaime-v0_5_0.dict")
    │
    ▼
new WasmEngine(dictBlob)  ────────►  parse dict blob → Dictionary
    │                                     │
    ▼                                     ▼
engine.push_key('m')      ────────►  InputContext::push_key()
engine.candidates()        ◄────────  JSON-serialized candidates
engine.commit(0)           ────────►  InputContext::commit()
    │
    ▼  (fire-and-forget, after page load)
fetch("thaime_ngram_v1_mc20.bin")
    │
    ▼
engine.load_ngram(blob)    ────────►  NgramData::from_bytes() → hot-load
```

### Dict Blob Format

The WASM engine loads a combined `.dict` file rather than separate trie + metadata files:

```
[4 bytes: trie_len as u32 LE][trie_bytes][metadata_bytes]
```

This is produced by `thaime_dictgen` and allows a single HTTP fetch at page load.

### Fire-and-Forget N-gram Loading

The web demo loads the dictionary first (blocking — required for any functionality), then loads the n-gram binary asynchronously. The engine works in unigram-only mode until n-gram data arrives, then hot-loads it via `WasmEngine::load_ngram()` and re-ranks if there's active input.

## Dictionary Pipeline

```
thaime-nlp                          thaime
──────────                          ──────
Thai corpus
    │
    ▼
word segmentation + romanization
    │
    ▼
trie_dataset.json ──────────────►  data/input/trie_dataset.json
                                       │
                                       ▼
                                   thaime_dictgen
                                       │
                                       ├── trie.bin        (yada double-array)
                                       ├── metadata.bin    (bincode: words + CSR)
                                       └── thaime.dict     (combined blob for WASM)
                                       │
                                       ▼  (build.sh step 3)
                                   Versioned:
                                       ├── trie-v0_5_0.bin
                                       ├── metadata-v0_5_0.bin
                                       └── thaime-v0_5_0.dict
```

The build script (`crates/thaime_engine/build.rs`) discovers versioned files and sets environment variables so `include_bytes!()` embeds them at compile time.

## N-gram Pipeline

```
thaime-nlp                          thaime
──────────                          ──────
Thai corpus
    │
    ▼
n-gram counting + scoring
    │
    ▼
thaime_ngram_v1_mc*.bin ────────►  data/input/thaime_ngram_v1_mc*.bin
                                       │
                                   ┌───┴───────────────────────────┐
                                   │                               │
                                   ▼ (compile-time)                ▼ (runtime)
                              embed-ngram feature            CLI: --ngram-bin
                              include_bytes!()               WASM: load_ngram()
```

The v1 binary format stores pre-scored log₁₀ probabilities for unigrams, bigrams, and trigrams with sorted ID arrays for binary search lookup. See [Algorithm — N-gram Language Model](algorithm.md#n-gram-language-model) for scoring details.

Multiple variants with different `min_count` thresholds are available. Higher min_count means smaller file size with fewer but more reliable n-gram entries. The build tooling automatically selects the highest available min_count.

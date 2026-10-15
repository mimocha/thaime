# Architecture

This document describes the high-level architecture of THAIME: how the components fit together, how data flows through the system, and why the design is the way it is.

## Overview

THAIME is a **Latin-to-Thai input method engine**. It lets users type Thai on a standard QWERTY keyboard using romanized keystrokes - similar to Pinyin input for Chinese or Romaji input for Japanese.

The core problem: given a Latin keystroke sequence like `sawatdee`, find the best Thai interpretation (`สวัสดี`). This requires a dictionary lookup, word segmentation (since Thai has no spaces), and a language model to disambiguate candidates.

## Data Flow

```
Keystroke (Latin char)
    │
    ▼
┌──────────────────┐
│  Input Context   │  context.rs - accumulates keystrokes, manages state
│  (buffer: "mai") │
└────────┬─────────┘
         │  on every keystroke: re-run full ranking pipeline
         ▼
┌──────────────────┐
│  Trie Lookup     │  trie.rs - prefix search on double-array trie
│   "mai" → [ไม่,   │  returns all romanization prefixes that match
│    ไหม, ใหม่, มา] │
└────────┬─────────┘
         │  prefix matches at every position → lattice edges
         ▼
┌──────────────────┐
│  Word Lattice    │  ranking.rs - DAG of all possible word spans
│  (edges by end   │  each edge: [start, end) → thai, freq, cost
│   position)      │
└────────┬─────────┘
         │  Viterbi forward pass with k-best tracking
         ▼
┌──────────────────┐
│  Viterbi DP      │  ranking.rs - scores paths through the lattice
│  + N-gram LM     │  ngram.rs - Stupid Backoff trigram scoring
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
│   Commit          │  context.rs - commits Thai text, updates context
│   context: [ไม่]  │  last 2 words retained for trigram window
└──────────────────┘
```

## Two-Repo Architecture

THAIME is split across two repositories:

| Repository | Purpose | Outputs |
|------------|---------|---------|
| **[thaime](https://github.com/mimocha/thaime)** | Engine, frontends, tools | `thaime_engine` crate, CLI, TUI, WASM, web demo |
| **[thaime-nlp](https://github.com/mimocha/thaime-nlp)** | NLP research, data pipelines | `trie_dataset.json`, `thaime_ngram_v1_mc*.bin` |

The NLP repo handles corpus processing, romanization variant generation, and n-gram count extraction. Its outputs are published as GitHub Release assets; `scripts/fetch-data.sh` downloads the release pinned in `thaime-data.toml` into this repo's `data/` directory, where the engine consumes them at compile time or runtime.

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
│   ├── thaime_engine/     Core library
│   ├── thaime_cli/        Interactive CLI test harness
│   ├── thaime_tui/        Ratatui-based visual debugger
│   ├── thaime_dictgen/    Dictionary compiler (JSON → binary trie)
│   └── thaime_wasm/       wasm-bindgen wrapper for browser
├── web/                   React + TypeScript web demo
├── data/
│   ├── dict/              Compiled dictionary binaries (fetched, not committed)
│   └── input/             Source JSON + n-gram binaries (fetched, not committed)
├── scripts/               fetch-data.sh (data), build-web.sh (web demo)
└── tests/                 Regression test data (TOML)
```

An IBus frontend (`crates/thaime_ibus`) is planned; see [plan 002](plans/002-linux-alpha.md).

### Crate Responsibilities

| Crate | Type | Description |
|-------|------|-------------|
| `thaime_engine` | lib | All core algorithms: trie, lattice, Viterbi, n-gram scoring. Rust API used by the other crates. |
| `thaime_cli` | binary | Interactive REPL for testing. Shows candidates with score breakdown (Total, Freq, Ngram, SegPen, Words). Supports binary and TSV n-gram loading. |
| `thaime_tui` | binary | Ratatui-based visual debugger with four modes: Main (candidate exploration with live parameter tuning), Lattice (word lattice visualization), Inspector (trie explorer with "why not?" diagnosis), Regression (run/view test results). |
| `thaime_dictgen` | binary | Compiles `trie_dataset.json` into binary trie + metadata files. Produces the combined `.dict` blob for WASM. |
| `thaime_wasm` | cdylib (WASM) | Thin wasm-bindgen wrapper. Exposes `WasmEngine` class to JavaScript with `push_key()`, `candidates()`, `commit()`, and `load_ngram()`. |

## Engine Modules

```
thaime_engine/src/
├── lib.rs          ThaiMeEngine struct (public Rust API)
├── context.rs      InputContext state machine
│                       │
│                       ▼ calls on every keystroke
├── ranking.rs      rank_candidates() - lattice + Viterbi DP
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
- **`trie.rs`** - standalone (depends on `yada`, `bincode`, `serde`)
- **`ngram.rs`** → `config.rs`
- **`config.rs`** - standalone (constants only)

## Frontend Interface

The engine is a plain Rust library; there is no C ABI today. Frontends are Rust crates in this workspace that call the Rust API directly: the CLI, TUI, WASM wrapper, and the planned IBus engine, which talks to `ibus-daemon` over D-Bus via `zbus` (plan 002, D6). A C API crate for C/C++ frameworks such as Fcitx5 will be designed when that work starts (plan 002, D8).

## WASM Architecture

The `thaime_wasm` crate wraps the engine for browser use via [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/):

```
Browser (JS)                          WASM
──────────────                        ────
fetch("thaime-v1_0_0.dict")
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
fetch("thaime_ngram_v1_mc15.bin")
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

The web demo loads the dictionary first (blocking - required for any functionality), then loads the n-gram binary asynchronously. The engine works in unigram-only mode until n-gram data arrives, then hot-loads it via `WasmEngine::load_ngram()` and re-ranks if there's active input.

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
                                       ├── trie-<vtag>.bin      (yada double-array)
                                       ├── metadata-<vtag>.bin  (bincode: words + CSR)
                                       └── thaime-<vtag>.dict   (combined blob for WASM)

<vtag> is the pinned data version (e.g. v1_0_0), passed by fetch-data.sh.
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

The v1 binary format stores pre-scored log₁₀ probabilities for unigrams, bigrams, and trigrams with sorted ID arrays for binary search lookup. See [Algorithm - N-gram Language Model](algorithm.md#n-gram-language-model) for scoring details.

Multiple variants with different `min_count` thresholds can be generated; each release ships one, pinned in `thaime-data.toml`. Higher min_count means smaller file size with fewer but more reliable n-gram entries.

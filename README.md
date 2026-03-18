# thaime

***Thai Input Method Engine | THA-IME - ไทยมี***

THAIME is a Latin-to-Thai input method engine. Type Thai using romanized Latin keystrokes on a standard QWERTY keyboard — the engine matches your input against a dictionary of Thai words and presents ranked candidates for selection. Think of it like Pinyin input for Chinese or Romaji input for Japanese, but for Thai.

## How It Works

1. You type Latin characters (e.g., `sawasdee`)
2. The engine runs prefix search on a trie-based dictionary at every position in the input
3. A word lattice is built from all possible word spans, then scored using Viterbi DP with n-gram context
4. Ranked Thai candidates are presented (e.g., สวัสดี)
5. You select a candidate, which is committed as Thai text and feeds back into the context window

## Web Demo

Try THAIME in your browser: [**Web Demo**](https://mimocha.github.io/thaime/)

The web demo runs the full engine client-side via WebAssembly — no server required.

## Installation

THAIME is under active development. IME framework frontends (IBus, Fcitx5) are planned for Q2–Q3 2026. For now, use the CLI, TUI, or web demo. See the [Installation Guide](docs/installation.md) for details.

## Project Structure

```
thaime/
├── crates/
│   ├── thaime_engine/     Core library (lib + cdylib + staticlib)
│   │   ├── lib.rs         ThaiMeEngine struct + C ABI exports
│   │   ├── trie.rs        Double-array trie dictionary (yada)
│   │   ├── ranking.rs     Viterbi k-best candidate ranking
│   │   ├── ngram.rs       N-gram language model (Stupid Backoff)
│   │   ├── context.rs     Input session state machine
│   │   ├── config.rs      Tunable parameters and constants
│   │   ├── keymap.rs      Latin → Thai mapping (planned)
│   │   └── validate.rs    Thai sequence validation (planned)
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

Dictionary and n-gram data is generated from the companion [thaime-nlp](https://github.com/mimocha/thaime-nlp) repository, which handles NLP research, corpus processing, and romanization variant generation.

## Building

Requires the stable Rust toolchain. Install via [rustup](https://rustup.rs/) if needed.

```bash
# Quick start (dictionary binaries are committed to the repo)
cargo build --workspace
cargo test --workspace

# Full pipeline: regenerate dict, build workspace, WASM, and web demo
./build.sh
```

See the [Build Guide](docs/build-guide.md) for prerequisites, feature flags, WASM setup, and CI details.

## CLI

```bash
cargo run -p thaime_cli
```

The CLI is an interactive REPL for testing the engine:

```
THAIME CLI v0.5.0
Commands: :q quit, :r reset, :b backspace, :cc clear context

<BOS> > sawatdee
   #  Thai              Total    Freq   Ngram  SegPen  Words
   1  สวัสดี              6.81    5.81    0.00    1.00      1

<BOS> > mai
   #  Thai              Total    Freq   Ngram  SegPen  Words
   1  ไม่                5.34    4.34    0.00    1.00      1
   2  ไหม                6.30    5.30    0.00    1.00      1
   3  ใหม่               6.52    5.52    0.00    1.00      1
```

- Type Latin characters (a-z) to build input and see candidates
- Enter a number (1-9) to commit that candidate
- Press Enter to commit the top candidate
- `:b` backspace, `:r` reset, `:cc` clear context, `:q` quit

## TUI

```bash
cargo run -p thaime_tui
```

The TUI is a ratatui-based visual debugger with four modes:

- **Main** — Live candidate exploration with score decomposition and real-time parameter tuning (lambda, ngram_weight, alpha, k, min_freq)
- **Lattice** — View all word lattice edges for the current input
- **Inspector** — Trie explorer with optional "why not?" target word diagnosis
- **Regression** — Run and view regression test results from TOML test files

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | High-level architecture, module map, data flow, two-repo design |
| [Algorithm](docs/algorithm.md) | Engine internals: trie, lattice, Viterbi, n-gram, scoring formulas |
| [Build Guide](docs/build-guide.md) | Prerequisites, `build.sh`, dict generation, WASM, CI/CD |
| [Installation](docs/installation.md) | Installation options and planned frontend support |

## Contributing

This is still a solo developer project, but I aim to open the project up for contributions before the end of 2026.

## License

[MPL-2.0](LICENSE)

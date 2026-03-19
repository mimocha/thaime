# thaime

***Thai Input Method Editor | THA-IME | ไทยมี***

THAIME is a Latin-to-Thai [input method editor](https://en.wikipedia.org/wiki/Input_method).
Type Thai using romanized Latin keystrokes on a standard QWERTY keyboard.
The engine takes in the sequence of latin characters and predicts your desired Thai characters with classical NLP technologies (no ML yet).
Think of it like typing in Chinese Pinyin or Japanese Romaji, but for Thai.

## How It Works

1. You type Latin characters (e.g., `sawasdee`)
2. The engine runs prefix search on a [trie-based](https://en.wikipedia.org/wiki/Trie) dictionary at every position in the input
3. A lattice of possible Thai words is built from all possible word spans, then scored using the [Viterbi algorithm](https://en.wikipedia.org/wiki/Viterbi_algorithm) with [n-gram](https://en.wikipedia.org/wiki/N-gram) context
4. Ranked Thai candidates are presented (e.g., สวัสดี)
5. You select a candidate, which is committed as Thai text and feeds back into the context window

## Web Demo

Try THAIME in your browser: [**Web Demo**](https://mimocha.github.io/thaime/)

The web demo runs the full engine client-side via WebAssembly - no servers required.

## Installation

THAIME is still under active development; no user-friendly installation methods are provided yet.

User friendly releases are planned for Q3-4 2026 and beyond. See [Roadmap](#roadmap) section below.

For developers, use the CLI/TUI, or try out the web demo.
See the [Installation Guide](docs/installation.md) for details.

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

- **Main** - Live candidate exploration with score decomposition and real-time parameter tuning (lambda, ngram_weight, alpha, k, min_freq)
- **Lattice** - View all word lattice edges for the current input
- **Inspector** - Trie explorer with optional "why not?" target word diagnosis
- **Regression** - Run and view regression test results from TOML test files

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | High-level architecture, module map, data flow, two-repo design |
| [Algorithm](docs/algorithm.md) | Engine internals: trie, lattice, Viterbi, n-gram, scoring formulas |
| [Build Guide](docs/build-guide.md) | Prerequisites, `build.sh`, dict generation, WASM, CI/CD |
| [Installation](docs/installation.md) | Installation options and planned frontend support |

## Roadmap

THAIME is in **pre-alpha** - the core engine works, but packaging and platform support are still in progress.

### Now - Pre-alpha development

- Core engine: trie dictionary, Viterbi ranking, n-gram context scoring
- NLP systems improvements - see [thaime-nlp repo](https://www.github.com/mimocha/thaime-nlp) for research progress
- Interactive CLI and TUI debugger for development
- Web demo via WebAssembly (client-side, no server)

### Q3 2026 - Linux community package alpha

First packaged releases targeting early adopters on Linux:

- **Fedora** via [COPR](https://copr.fedorainfracloud.org/)
- **Ubuntu** via [Launchpad PPA](https://launchpad.net/)
- IBus frontend with basic compose/commit workflow
- Feedback-driven iteration on dictionary coverage and ranking quality

### Q4 2026 - Wider Linux beta

Broader distribution packaging and more frontend support:

- **Fedora / RHEL** via dnf
- **Ubuntu / Debian** via apt
- **Arch** via AUR
- Other major distros as demand warrants
- Wider frontend support (Fcitx5 and more)
- Open for community contributions

### 2027 and beyond - Windows / macOS public release

- Windows IME (TSF) and macOS input method frontends
- Cross-platform installer / package manager support
- Continued improvements to dictionary, ranking, and language model

This roadmap is tentative and subject to change.

## Contributing

This is still a solo developer project, but I aim to open the project up for contributions by the end of 2026.

## License

[MPL-2.0](LICENSE)

# thaime

***Thai Input Method Engine | THA-IME - ไทยมี***

THAIME is a Latin-to-Thai input method engine. Type Thai using romanized Latin keystrokes on a standard QWERTY keyboard — the engine matches your input against a dictionary of Thai words and presents ranked candidates for selection.

Think of it like Pinyin input for Chinese or Romaji input for Japanese, but for Thai.

## How It Works

1. You type Latin characters (e.g., `sawatdee`)
2. The engine matches input against a trie-based dictionary of romanized Thai words
3. A ranked list of Thai candidates is presented (e.g., สวัสดี)
4. You select the correct candidate, which is committed as Thai text

## Project Structure

THAIME is a Rust workspace that produces a shared library (`libthaime_engine.so`) with a C ABI, designed for consumption by input method framework frontends (IBus, Fcitx5, etc.).

```
thaime/
├── crates/
│   ├── thaime_engine/     Core library (lib + cdylib + staticlib)
│   │   ├── lib.rs         Rust API + C ABI exports
│   │   ├── trie.rs        Double-array trie dictionary (yada)
│   │   ├── ranking.rs     Viterbi k-best candidate ranking
│   │   ├── context.rs     Input session state machine
│   │   ├── keymap.rs      Latin → Thai mapping (planned)
│   │   └── validate.rs    Thai sequence validation (planned)
│   ├── thaime_cli/        Interactive CLI test harness
│   └── thaime_dictgen/    Dictionary compiler (JSON → binary trie)
├── frontends/
│   └── ibus/              IBus engine frontend (planned)
├── data/dict/             Compiled dictionary binaries (generated)
└── scripts/               Build and install utilities
```

Dictionary data is generated from the companion [thaime-candidate](https://github.com/mimocha/thaime-candidate) repository, which handles NLP research, corpus processing, and romanization variant generation.

## Building

Requires the stable Rust toolchain. Install via [rustup](https://rustup.rs/) if needed.

```bash
# Generate dictionary (required once, before first build)
cargo run -p thaime_dictgen -- data/trie_dataset.json

# Build
cargo build --workspace

# Run tests
cargo test --workspace

# Lint
cargo fmt --all
cargo clippy --all-targets --all-features
```

## Running the CLI

```bash
cargo run -p thaime_cli
```

The CLI is an interactive REPL for testing the engine:

```
THAIME CLI v0.1.0
Type Latin characters to see Thai candidates. Commands: :q quit, :r reset, :b backspace

> sawatdee
  1. สวัสดี           (score: 6.31)

> mai
  1. ไม่              (score: 4.84)
  2. ไหม              (score: 5.80)
  3. ใหม่             (score: 6.02)

> 1
  -> ไม่
```

- Type Latin characters to build input and see candidates
- Enter a number (1-9) to commit that candidate
- Press Enter to commit the top candidate
- `:b` backspace, `:r` reset, `:q` quit

## Contributing

Contributions are welcome. Please open an issue to discuss changes before submitting a pull request.

## License

[MPL-2.0](LICENSE)

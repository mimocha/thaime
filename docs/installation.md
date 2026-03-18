# Installation Guide

THAIME is under active development. Native IME frontend integration (IBus, Fcitx5) is planned for Q2–Q3 2026.

## Current Options

Until framework frontends are available, you can try THAIME through:

- **Web Demo** — Try it in your browser at the [GitHub Pages demo](https://mimocha.github.io/thaime/)
- **CLI** — Interactive REPL for testing the engine (`cargo run -p thaime_cli`)
- **TUI** — Visual debugger with score decomposition (`cargo run -p thaime_tui`)

## Building from Source

See the [Build Guide](build-guide.md) for prerequisites and build instructions.

## Planned Frontends

| Frontend | Platform | Status |
|----------|----------|--------|
| IBus     | Linux (GNOME, etc.) | Planned |
| Fcitx5   | Linux (KDE, etc.)   | Planned |

The engine produces a shared library (`libthaime_engine.so`) with a C ABI, designed for consumption by these input method frameworks. See [Architecture](architecture.md) for details on the C ABI design.

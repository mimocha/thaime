# Installation Guide

THAIME is still under active development; user installation is currently not supported.

## Current Options

Until framework frontends are available, you can try THAIME through:

- **Web Demo** - Try it in your browser at the [GitHub Pages demo](https://mimocha.github.io/thaime/)
- **CLI** - Interactive REPL for testing the engine (`cargo run -p thaime_cli`)
- **TUI** - Visual debugger with score decomposition (`cargo run -p thaime_tui`)

## Building from Source

See the [Build Guide](build-guide.md) for prerequisites and build instructions.

## Planned Frontends

| Frontend | Platform | Status |
|----------|----------|--------|
| IBus     | Linux (GNOME, etc.) | In progress: alpha via Fedora COPR and Ubuntu PPA, end of 2026 |
| Fcitx5   | Linux (KDE, etc.)   | Planned |

The IBus frontend is a Rust program (`ibus-engine-thaime`) that talks to `ibus-daemon` over D-Bus; see [plan 002](plans/002-linux-alpha.md). Fcitx5 support needs a C API, which will be designed when that work starts. See [Architecture](architecture.md#frontend-interface).

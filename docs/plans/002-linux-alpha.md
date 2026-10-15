---
id: "002"
title: Linux alpha (IBus engine, COPR/PPA packages)
status: accepted
created: 2026-10-15
updated: 2026-10-15
authors:
  - Chawit Leosrisook
  - Claude (agent)
parent: null
milestone: linux-alpha
supersedes: []
superseded_by: null
related:
  - docs/plans/001-crowdsourcing-demo.md
  - tests/harness-regression.toml
  - thaime-data.toml
---

# Plan 002: Linux alpha (IBus engine, COPR/PPA packages)

## Objective

Ship a first Linux alpha by the end of 2026: a native IBus engine (`ibus-engine-thaime`)
that the maintainer can use daily on Fedora GNOME Wayland, packaged for Fedora COPR and
an Ubuntu PPA. To get there, reshape the engine boundary (shared immutable `Engine`,
per-input `Session` state machine, fallible runtime data loading) without changing
ranking results.

This is an umbrella plan. It records the decisions shared by all workstreams; WS1, WS2,
and WS5 each get a child plan (`parent: "002"`) before implementation starts.

## Context

The algorithm core (yada double-array trie, word lattice, k-best Viterbi, Stupid
Backoff n-gram scoring) is sound and fast enough: 11–16 ms per keystroke for a 50-char
input in release builds. What blocks a native frontend and distro packaging is the
**engine boundary**, which was shaped by the web demo:

- one struct owns ~10 MB of data *and* per-input state;
- the IME's interaction logic (Space/Enter/digits/Escape, partial commit, raw-Latin
  fallback) lives in TypeScript (`web/src/hooks/useIME.ts`), not in the engine;
- data is embedded at compile time, and loading panics on missing or bad data;
- the C ABI has no candidate, preedit, or commit path.

### Findings

Reviewed by reading and running the code at `dc07f07` (2026-10-15). F1–F15 come from
the original design review; F16–F21 were found while verifying it.

| # | Finding | Where |
|---|---------|-------|
| F1 | `ThaiMeEngine` → `InputContext` owns `Dictionary`, `NgramData`, buffer, candidates, and context. Every instance deserializes the full dictionary: the first engine costs +41.7 MB RSS, each further one +21.6 MB. | `lib.rs` `ThaiMeEngine`; `context.rs` `InputContext` |
| F2 | IME interaction logic lives in the web app (hybrid candidate list, key handling, `committedPrefix`). Native frontends would have to re-implement it. | `web/src/hooks/useIME.ts` |
| F3 | `reset()` also clears the n-gram context. Escape and the pass-through commit call `engine.reset()`, wiping context; Escape also drops words already accepted through partial commit. `setMode` wipes context too. | `context.rs:135`; `useIME.ts` Escape and pass-through handlers |
| F4 | `commit_partial(thai_word, consume_bytes)` trusts a caller-supplied Thai string and byte count; nothing checks they match a lattice edge. | `context.rs:214` |
| F5 | `refresh_candidates` hard-codes `RankingParams::default()`; params are tunable only in the TUI. | `context.rs:250` |
| F6 | Ranking returns no candidates when the buffer cannot be fully tiled (e.g. mid-word). The web UI masks this with first-word and pass-through items; Space then does nothing. | `ranking.rs` `test_partial_coverage_returns_empty` |
| F7 | Viterbi state is keyed by `(Option<String>, Option<String>)`; every path extension clones the whole `Vec<CandidateWord>`. The n-gram model keeps a separate `HashMap<String, u16>` vocab, although its vocab (16,566 words) is identical in content and order to the dictionary's `word_id`s. | `ranking.rs:254`, `ngram.rs:83` |
| F8 | `embed-dict` is a default feature; `build.rs` panics when dict files are missing, even for `cargo check -p thaime_wasm`; runtime loading uses `.expect()`. A panic inside an IME process kills input for the session. | `build.rs:114`, `:155`; `trie.rs:84`, `:95`; `ngram.rs:199` |
| F9 | Dictionary metadata is raw `bincode` 1.3 with no magic or version header (the n-gram format has one: `TNLM` v1). | `trie.rs` `DictData` |
| F10 | `build.rs` runs cbindgen and writes `target/thaime.h` (outside `OUT_DIR`) for every consumer of the engine crate, including WASM. | `build.rs:14` |
| F11 | The C ABI exposes no candidates, preedit, commit, or partial commit; `thaime_process_key` lacks the null check its siblings have. Nothing consumes the C ABI today. | `lib.rs:304` |
| F12 | The CI clippy line has a stray `]` (`-- -D warnings]`): clippy runs but reports `unknown lint: 'warnings]'` and exits 0, so warnings are not enforced. (A correct `-D warnings` run currently reports 0 warnings.) CI only runs on pushes/PRs to `main`. | `.github/workflows/ci.yml:45` |
| F13 | Docs drift: README and `docs/build-guide.md` say dict binaries are committed, but `.gitignore` ignores `data/dict/*`. README references `build.sh` and `frontends/ibus/`, neither of which exists (the script is `scripts/build-web.sh`). The README roadmap still puts the crowdsourcing demo before the Linux alpha. | README, `docs/build-guide.md:30` |
| F14 | Fresh clones need the `gh` CLI to fetch data. Release assets are public (`curl` gets HTTP 200 without auth). | `scripts/fetch-data.sh` L74–81 |
| F15 | Mobile "ranking corruption" issue: memory eviction is unlikely (WASM linear memory isn't silently zero-filled). More likely: tab discarded → reload → n-gram fetch failed or raced → unigram-only ranking. Without n-gram data, `malongchaithaime` ranks มาโล่งใจไทยมี first, matching the report. | `docs/issues/mobile-background-ranking-corruption.md` |
| F16 | `scripts/fetch-data.sh` uses `THAIME_DATA_DIR` for "local thaime-nlp artifacts", which collides with the runtime data-path variable this plan introduces. | `scripts/fetch-data.sh:18`, `:58` |
| F17 | yada lookups use unchecked indexing, so a truncated or corrupt trie file is undefined behaviour, not an error. Integrity checking on load is a safety requirement. | `trie.rs` (yada `get_unchecked`) |
| F18 | Candidate ties are broken in `HashMap` iteration order. 0/50 runs showed nondeterminism, but the order is not guaranteed. | `ranking.rs` |
| F19 | Plan 001 assumed the commit log "builds on the existing `commit_partial`", which this plan removes (F4). Plan 001 has been updated. | `docs/plans/001-crowdsourcing-demo.md` |
| F20 | Regression suite (`tests/harness-regression.toml`, 20 cases, default params, mc15 n-gram, no context): 17 pass, 3 fail. The suite is only run interactively in the TUI (F5 key); `cargo test` does not run it. | `crates/thaime_tui/src/main.rs:48` |
| F21 | The web demo deployed to GitHub Pages already redistributes the dictionary and n-gram data publicly, with no data license or attribution notice. | `.github/workflows/deploy-web.yml` |

Regression cases failing today (kept as known targets, see D14):

| Input | Expected | Current top-1 |
|---|---|---|
| `peemaineeyakkeunngernduean` | ปีใหม่นี้อยากขึ้นเงินเดือน | ปีใหม่นี้ยากขึ้นเงินเดือน |
| `peekrubpomhiwkao` | พี่ครับผมหิวข้าว | ปีครับผมหิวข้าว |
| `sawaddee` | สวัสดี | สวัสดิ์ดี |

### Linux input stack (October 2026)

```
App toolkit (GTK3/GTK4/Qt5/Qt6/Chromium/Electron/XWayland apps)
   │  Wayland text-input-v3 (native) or an IM module (GTK_IM_MODULE=ibus …)
Compositor (Mutter / KWin / wlroots-family / COSMIC)
   │  GNOME: Mutter talks to IBus directly over D-Bus
   │  KDE: KWin launches the IM chosen as "Virtual Keyboard" (input-method-v1)
   │  Sway/Hyprland/COSMIC: input-method-v2
IM framework (IBus daemon / Fcitx5)      ← draws candidate panel, routes keys
   │  IBus: D-Bus org.freedesktop.IBus.Engine   Fcitx5: C++ addon .so
Engine  ← THAIME lives here
```

An engine targets the framework's API; the framework absorbs compositor and toolkit
differences. IBus first remains correct: it is the default on GNOME (Fedora, Ubuntu
26.04 LTS).

- X11 sessions are gone from GNOME 50 and KDE Plasma 6.8; test Wayland sessions plus
  XWayland apps.
- IBus 1.5.32 added Wayland input-method-v2 (Sway, Hyprland, COSMIC). 1.5.34's
  documented addition is surrounding-text deletion. 1.5.35 is accepted for Fedora 45.
- KDE: IBus works as the "IBus Wayland" virtual keyboard; Fcitx5 integrates better and
  is the beta target.
- The experimental `xx-*` Wayland IM protocols are not in wayland-protocols 1.45. Nothing
  to act on.
- Pure-Rust IBus engines are proven: `librush` 0.2.3 (LGPL-2.1+ OR GPL-3.0+) implements
  one over `zbus` without GObject. `zbus` (MIT) supports the IBus private bus.
- `IBUS_ENGINE_PREEDIT_COMMIT` (commit preedit on focus loss) is ignored on GNOME's
  native Wayland text-input path in at least GNOME 48 (ibus#2792). It commits the
  displayed preedit, which for THAIME is Latin, not Thai.
- Known weak spot across frameworks: Chromium/Electron apps on native Wayland.

## Design Decisions

1. **D-PRIO: IBus alpha before the crowdsourcing demo.** The maintainer needs a working
   prototype on their own machine first. Plan 001 moves to `milestone: post-alpha`.

2. **D1: n-gram hot-load swaps the `Arc<Engine>`.** The web demo builds a new `Engine`
   when the n-gram file arrives and gives sessions the new `Arc` on their next
   keystroke. Alternative: `RwLock<Option<NgramData>>` inside `Engine`. Rejected: adds
   locking to every lookup for a web-only concern.

3. **D2: partial commit sends the chosen word to the application immediately**
   (`Output.commit`). A native IME has no frontend-held "committed prefix". The web demo
   can still buffer it for display.

4. **D3: focus loss and reset discard the composition and clear the n-gram context.**
   The dominant convention for conversion IMEs (ibus-libpinyin, ibus-rime, ibus-mozc,
   ibus-anthy, Fcitx5 pinyin/rime/mozc) is to discard unfinished input on reset.
   Alternatives: commit the top candidate (no precedent; surprising when IBus sends
   `Reset` for a click inside the same field); commit raw Latin via
   `IBUS_ENGINE_PREEDIT_COMMIT` (what ibus-hangul and typing-booster do, but for THAIME
   it commits Latin, and it is ignored on GNOME native Wayland, see ibus#2792).

5. **D4: keep bincode 1 for the alpha, add a version check and integrity check.** A
   new single-file `TDIC` format waits for thaime-nlp CP12 (romanization weights), so
   the format changes once, not twice. The integrity check (size + checksum) is
   mandatory because of F17.

6. **D5: prefix completion is deferred.** The alpha does the cheap part only: best path
   over the longest tileable prefix plus the raw Latin tail (WS2).

7. **D6: own minimal IBus engine on `zbus`, after a spike.** Alternatives: depend on
   `librush` (LGPL/GPL dependency in an MPL project; static linking brings relinking
   obligations); a C frontend on `libibus` (requires a C ABI first, GObject
   boilerplate). `librush` may be read as reference only. If the spike fails, fall back
   to the C frontend.

8. **D7: three input modes, all enabled by default: Romanization, Latin, Kedmanee.**
   Exposed the standard IBus way: one top-level `InputMode` property with three radio
   items (GNOME Shell shows it in the input-source menu and uses its symbol as the
   indicator), plus a configurable engine-local switch key. The default key must avoid
   Super+Space, Shift+Super+Space (GNOME, Fcitx5), Meta+Space (KDE), and Ctrl+Space
   (Fcitx5 toggle, some IBus setups); the web demo's Ctrl+Space is not carried over.
   The component XML fixes the engine layout to `us`, so the existing `kedmanee_map`
   works unchanged. Docs also mention the system `th` XKB layout as an alternative.

9. **D8: delete the C ABI and cbindgen from `thaime_engine` now.** Nothing consumes
   them (F11), and they force cbindgen on every build (F10). The engine crate becomes a
   plain `lib` (drop `cdylib`/`staticlib`). A `thaime_capi` crate is designed against
   `Session` when Fcitx5 work starts (WS6).

10. **D9: clean cutover to `Engine`/`Session`.** All callers (CLI, TUI, WASM) migrate
    in WS1; `ThaiMeEngine` is removed. Alternative: a compatibility wrapper. Rejected:
    no external users to protect, and a wrapper would outlive its purpose.

11. **D10: data ships as separate files, loaded at runtime.**
    - `Engine::load(dir) -> Result<Engine, LoadError>` and
      `Engine::from_bytes(..) -> Result<Engine, LoadError>`; no panics on data paths.
    - Search path: `$THAIME_DATA_DIR`, then `$XDG_DATA_HOME/thaime`, then each
      `$XDG_DATA_DIRS/thaime` (covers `/usr/share/thaime`).
    - `build.rs` and the `embed-dict` feature are removed. The web demo already loads
      from bytes. CLI, TUI, and tests find `data/` through `.cargo/config.toml` `[env]`.
    - `fetch-data.sh`'s variable is renamed to `THAIME_NLP_LOCAL_DIR` (F16).
    - Alternative: keep data embedded. Rejected: distro guidelines prefer
      architecture-independent data under `/usr/share` in its own package, and separate
      files let data update without an engine rebuild.

12. **D11: one source package builds both binary packages; packaging lives in this
    repo.**
    - RPM: one SRPM `thaime` (Source0 thaime tarball, vendored-crates tarball, pinned
      thaime-nlp release assets; `%build` runs `thaime_dictgen`) producing
      `ibus-thaime` (arch) and `thaime-data` (noarch).
    - Debian: one source package producing `ibus-thaime` (any) and `thaime-data` (all),
      with the data as an `orig-data` component tarball (`3.0 (quilt)`).
    - Build environments have no network (Launchpad always; COPR by default), hence
      vendored crates and data as package sources.
    - Specs live in `packaging/`. Split `thaime-data` into its own source package only
      if data and engine release cadences diverge.

13. **D12: data license is CC-BY-SA-4.0, with an attribution NOTICE.** Code licenses
    are unchanged (thaime-nlp MIT, thaime MPL-2.0). The n-gram data is derived from
    wisesight (CC0-1.0), wongnai (LGPL-3.0), prachathai (Apache-2.0), and Thai Wikipedia
    (CC-BY-SA-4.0; older revisions 3.0); the word list also uses PyThaiNLP `words_th.txt`
    (CC0-1.0) and the project's own component romanization dictionary. Licensing the
    data CC-BY-SA-4.0 covers the share-alike case if it applies. The license travels
    with the data: copying it into thaime does not relicense it to MPL. Not legal
    advice; revisit before submitting to official Fedora/Debian repositories.

14. **D13: process.** Plans follow [docs/plans/README.md](README.md). For now, branches
    are created off `dev` and merged into `dev` locally, without PRs; the maintainer
    opens `dev` → `main` PRs manually.

15. **D14: regression baseline is a snapshot.** A headless test in `cargo test` asserts
    every case's current top-1, including the three known failures (recorded as
    known targets). A refactor that changes any top-1 is a bug unless the maintainer
    accepts it. An intentional ranking change (e.g. new data fixing a known failure)
    updates the snapshot in the same commit.

## Non-Goals

- Changing the scoring formula, smoothing, or default ranking parameters.
- Changing preedit behaviour: preedit keeps showing the Latin buffer.
- Fcitx5, Windows TSF, macOS, Android.
- Crowdsourcing backend (plan 001).
- WTT 2.0 validation (`validate.rs` stub).
- Implementing Wayland input-method protocols directly.

## Implementation Plan

### Target Architecture

```
thaime/
├── crates/
│   ├── thaime_engine/     pure Rust lib: data model, ranking, Session state machine
│   │                      (no build.rs, no cbindgen, no embedded data)
│   ├── thaime_ibus/       ibus-engine-thaime binary over zbus (WS5)
│   ├── thaime_wasm/       wasm-bindgen wrapper (Session in WS7)
│   ├── thaime_capi/       cdylib/staticlib + cbindgen (WS6, post-alpha)
│   └── thaime_cli/, thaime_tui/, thaime_dictgen/   (roles unchanged)
├── packaging/             RPM spec, debian/, IBus component XML (WS5d)
└── ...
```

```rust
/// Immutable, shareable. Loaded once per process.
pub struct Engine { dict: Dictionary, ngram: Option<NgramData>, config: Config }

/// Cheap, one per input context (text field / IBus input context).
pub struct Session { engine: Arc<Engine>, buffer: String, context: ContextWindow,
                     candidates: CandidateList, mode: InputMode }

pub enum Key { Char(char), Backspace, Escape, Enter, Space, Tab,
               Up, Down, PageUp, PageDown, Digit(u8) }  // frontend maps keysyms → Key

pub struct Output {
    pub handled: bool,             // false → frontend passes the key to the app
    pub commit: Option<String>,    // text to commit to the app now
    pub preedit: String,           // Latin buffer
    pub candidates: CandidateView, // items + highlighted index + page info
}

impl Session {
    pub fn process(&mut self, key: Key) -> Output;
    pub fn select(&mut self, index_on_page: usize) -> Output;  // mouse click
    pub fn set_mode(&mut self, mode: InputMode) -> Output;
    pub fn reset(&mut self) -> Output;  // D3: discard buffer + clear context
}
```

The exact API is settled in the WS1 and WS2 child plans; the sketch fixes the shape:
immutable shared data, per-context state, one `Output` per event.

### WS0 — Housekeeping

Small, independent fixes. Covered by this plan; no child plan.

- [x] Fix the clippy line in `ci.yml` so `-D warnings` is enforced (F12); run CI on
      pushes and PRs to `dev` as well as `main`.
- [x] Headless regression test: run `tests/harness-regression.toml` in `cargo test`
      against the snapshot (D14, F20). CI already fetches data.
- [x] Delete the C ABI, cbindgen, `cbindgen.toml`, and the `cdylib`/`staticlib`
      crate types (D8); update docs that describe the C ABI (`docs/architecture.md`,
      `docs/build-guide.md`, README, and the maintainer's local `AGENTS.md`).
- [x] Docs: dict binaries are fetched, not committed; `build.sh` →
      `scripts/build-web.sh`; drop or mark `frontends/` as planned; README and web
      demo roadmap show the alpha before the crowdsourcing demo (F13).
- [x] `fetch-data.sh` downloads with `curl` from the public release URL and no longer
      uses `gh` (F14); duplicated SPDX header line removed.
- [x] WASM: expose `has_ngram()`; web: log it on `visibilitychange`; update the mobile
      issue doc with the alternative theory (F15).
- [ ] Data license notice: `fetch-data.sh` also downloads `LICENSE-DATA` and
      `NOTICE-DATA.md` from the pinned release; `data/README.md` and the web demo state
      the data license and attributions (D12, F21). Needs the thaime-nlp cross-repo
      tasks below; the pin stays at v1.0.0.

**Done when:** CI enforces clippy warnings on `dev`; `cargo test --workspace` runs the
regression snapshot; `thaime_engine` builds only a `lib` with no cbindgen dependency;
a fresh clone with `curl` but no `gh` fetches data and builds following the README.

### WS5a — IBus spike (time-box: 1–2 evenings)

Minimal `zbus` engine that registers with `ibus-daemon` (component XML + private bus +
`org.freedesktop.IBus.Factory.CreateEngine`), receives `ProcessKeyEvent`, and emits
`CommitText` for a hard-coded string, on Fedora GNOME Wayland. Must serialize `IBusText`
(`(sa{sv}sv)`) correctly; `IBusLookupTable` serialization is tried if time allows.

**Done when:** typing a key in a GTK4 app on GNOME Wayland commits the hard-coded
string. If not achieved within the time-box, report findings and revisit D6.

Throwaway code; it lives on a spike branch and is not merged. Findings feed the WS5
child plan.

### WS1 — Engine/Session split and runtime loading

Child plan required. Scope:

- `Engine` (immutable, `Arc`) and `Session`; migrate CLI, TUI, WASM; remove
  `ThaiMeEngine` (D9).
- Fallible loading with `LoadError`; no `.expect()`/`panic!` on data paths (F8).
- XDG search path and `THAIME_DATA_DIR` (D10); remove `build.rs` and `embed-dict`.
- Data version check and integrity check (size + checksum) on load (D4, F17).
- Read `RankingParams` from `Config` (F5).
- Deterministic tie-breaking in ranking (F18), without changing any top-1 result.

**Done when:** two sessions share one `Engine` (test asserts `Arc::strong_count`);
the regression snapshot is unchanged; truncated or garbage data returns `LoadError`
(tests over truncated byte slices); `cargo build --workspace` succeeds on a fresh
clone with no data; a test loads from a temporary `XDG_DATA_DIRS`.

### WS2 — Session state machine

Child plan required; needs the maintainer's decisions on the web demo's input quirks
(see Open Questions). Scope:

- Port the hybrid candidate list (full sentence, first-word alternatives,
  pass-through) and key handling from `useIME.ts` into `Session`. Page sizes stay as
  defaults (4 first-word items on page 0, 6 on later pages, cap 30), made configurable.
- Partial commit by index into the engine's own list; remove the free-text
  `commit_partial(thai, bytes)` (F4). Commits go out immediately (D2).
- Escape discards the buffer only; `reset()` discards and clears context (F3, D3).
- Modes move into `Session` (D7).
- Untileable input yields best-effort output: best path over the longest tileable
  prefix plus the raw Latin tail (F6, D5).
- Table-driven tests replaying key sequences, e.g. `["m","a","i","Space"] → commit
  "ไม่"`, covering partial commit, pass-through, paging, Backspace into an empty buffer,
  and mode switches.

**Done when:** every key the web demo handles has a Rust test with the agreed outcome;
regression snapshot unchanged.

### WS5 — IBus engine and packaging

Child plan required (after WS5a). Scope:

- **WS5b engine** (`crates/thaime_ibus`, binary `ibus-engine-thaime`):
  - Component file `/usr/share/ibus/component/thaime.xml`, `<exec>` →
    `/usr/libexec/ibus-engine-thaime --ibus`; engine name `thaime`, language `th`,
    layout `us`.
  - Methods: `ProcessKeyEvent`, `FocusIn`, `FocusOut`, `Reset`, `Enable`, `Disable`,
    `PageUp`, `PageDown`, `CursorUp`, `CursorDown`, `CandidateClicked`,
    `PropertyActivate`, `SetCapabilities` (`SetSurroundingText` may be ignored).
  - Signals: `CommitText`, `UpdatePreeditText`, `HidePreeditText`,
    `UpdateLookupTable`, `HideLookupTable`, `RegisterProperties`, `UpdateProperty`,
    optionally `UpdateAuxiliaryText`.
  - Ignore key releases; pass through keys with Ctrl/Alt/Super held; map keysyms to
    `Key`; return `Output.handled`.
  - `FocusOut`/`Reset` → `Session::reset()` (D3). One `Engine` per process, one
    `Session` per IBus engine object.
  - `InputMode` property and switch key (D7).
- **WS5c manual test matrix** (results recorded in the child plan): GNOME 50/51
  Wayland (Fedora 44/45, Ubuntu 26.04); KDE Plasma with "IBus Wayland"; Sway or
  Hyprland (IBus ≥ 1.5.32). Apps: GTK4, GTK3, Qt6, Firefox, a Chromium/Electron app,
  an XWayland app, a terminal.
- **WS5d packaging** (D11): `packaging/` RPM spec for COPR and `debian/` for the PPA;
  installs the binary, component XML, data under `/usr/share/thaime/`, icon, and the
  data license/NOTICE.

**Done when:** the maintainer installs from COPR on Fedora GNOME Wayland and types
Thai in the WS5c apps; the PPA package installs on Ubuntu 26.04.

### Post-alpha

- **WS3 — data format:** single `TDIC` file with header (magic, format version, vocab
  size, section offsets), one shared vocabulary with `u32` word IDs used by trie
  postings and n-gram tables, a per-posting romanization weight slot for CP12;
  bincode replacement (D4). Changes the thaime ↔ thaime-nlp contract.
- **WS4 — ranking internals:** criterion benchmark first; ID-keyed Viterbi state and
  arena back-pointers instead of cloning paths (F7); top-1 identical.
- **WS6 — C ABI crate** (when Fcitx5 work starts): `thaime_capi` over `Session`;
  opaque `ThaimeEngine*`/`ThaimeSession*`, null-checked entry points, cbindgen writing
  to `OUT_DIR`.
- **WS7 — web demo on `Session`:** `thaime_wasm` wraps `Session`; `useIME.ts` shrinks
  to rendering and key forwarding; `Output.commit` events provide plan 001's commit log.
- **Plan 001** (crowdsourcing demo): M1 needs little engine work; M2 waits for WS7.

### Cross-repo Tasks (thaime-nlp)

- [ ] Add `LICENSE-DATA` (CC-BY-SA-4.0) and `NOTICE-DATA.md` with source attributions;
      include both in every release's assets (D12). The data license also covers
      `benchmarks/word-conversion/`; hand-authored `data/dictionaries/` stay MIT.
- [ ] Attach both files to the existing v1.0.0 release without touching its data
      assets, so thaime's pinned checksums stay valid.
- [ ] Correct the Thai Wikipedia license in `src/data/registry.py` and
      `data/corpora/README.md` (current dumps are CC-BY-SA-4.0).
- [ ] Document the raw n-gram TSVs (unnormalized, `count >= 2`, cannot rebuild the
      binary alone); no extra reproducibility assets for now.

## Execution Order

1. **WS0** — unblocks everything: enforced CI, regression snapshot in `cargo test`,
   C ABI gone before the engine is reshaped.
2. **WS5a** — de-risks D6 early; independent of WS1/WS2.
3. **WS1** — the engine boundary every later workstream builds on.
4. **WS2** — needs `Session` from WS1 and the maintainer's quirk decisions.
5. **WS5b → WS5d → WS5c** — engine, packaging, then the test matrix on packaged builds.
6. **Post-alpha** — WS3, WS4, WS6, WS7, then plan 001.

## Acceptance Criteria

- [ ] WS0, WS1, WS2, and WS5 meet their "Done when" criteria above (WS1, WS2, WS5 via
      their child plans).
- [ ] `cargo test --workspace` and `cargo clippy --all-targets --all-features -- -D warnings`
      pass on `dev`, and CI runs both.
- [ ] The regression snapshot is unchanged from WS0 through WS5, except for changes
      the maintainer explicitly accepted.
- [ ] No code path reachable from `ibus-engine-thaime` panics on missing or corrupt
      data: the engine logs the `LoadError` and passes keys through.
- [ ] COPR and PPA packages install `ibus-thaime` and `thaime-data`, with the data
      package declaring CC-BY-SA-4.0 and shipping the NOTICE.
- [ ] The maintainer uses the COPR build on Fedora GNOME Wayland for daily typing.

## Open Questions

1. **Should thaime-nlp's public releases keep shipping the raw n-gram TSVs?** The
   `ngrams_{1,2,3}_merged_raw.tsv.gz` assets (~79 MB of the ~87 MB v1.0.0 release) are
   unnormalized counts merged across corpora, Thai tokens only, already filtered at
   `count >= 2` by the count stage (`pipelines/ngram/count.py`). They are not
   sufficient to rebuild the shipped binary: scoring also needs the normalized
   `ngrams_{1,2,3}_merged.tsv` and the per-corpus unigram files, which are not
   released. The engine packages use only the trie dataset and the n-gram binary, so
   the TSVs never enter distro packages.

   **Decision (2026-10-15):** keep publishing them, under the CC-BY-SA-4.0 data license
   with the attribution NOTICE (D12). Documenting what they are, and whether to close
   the reproducibility gap, is handled in thaime-nlp.

2. **Web demo input quirks: which become intended `Session` behaviour?** Deferred to the
   WS2 child plan. Current behaviour:
   - Space does nothing when no full-sentence candidate exists (F6).
   - ArrowUp onto the previous page lands on its first item, not its last.
   - Punctuation and digits 7–9/0 typed mid-composition are dropped (IME convention:
     commit the top candidate, then insert the character).
   - First-word candidates are sorted by unigram frequency, ignoring context.
   - Enter commits the highlighted candidate (many IMEs commit the raw input instead).
   - Switching modes discards pending input (ibus-libpinyin commits it first).

3. **Default mode-switch key.** Deferred to the WS5 child plan, within the constraints
   of D7.

4. **Should the wongnai corpus stay in the n-gram data?** LGPL-3.0 is awkward next to
   CC-BY-SA-4.0. Proposed as a thaime-nlp research topic: measure the ranking impact of
   removing it; drop it if the loss is small. Does not block the alpha.

## Branch

One branch per workstream, off `dev`, merged into `dev` locally:

- WS0: `chore/002-ws0-housekeeping` (may be split into several small branches)
- WS5a: `spike/002-ws5a-ibus` (not merged)
- WS1: `refactor/002-ws1-engine-session`
- WS2: `feat/002-ws2-session-state-machine`
- WS5: `feat/002-ws5-ibus-engine`, `packaging/002-ws5d-copr-ppa`

## Risks

- **Hand-written IBus GVariant serialization** (`IBusText`, `IBusLookupTable`) is the
  fiddliest part of D6. Mitigated by the WS5a spike and `librush` as a reference.
- **GNOME native Wayland quirks** (e.g. ibus#2792). Mitigated by D3 not relying on
  `PREEDIT_COMMIT`, and by the WS5c matrix.
- **Schedule:** ~11 weeks to the end of 2026 with limited hours. WS3, WS4, WS6, and
  WS7 are already deferred; if time runs short, the PPA is the next thing to slip.
- **Data license interpretation** (D12) may be challenged during official distro
  review. COPR/PPA proceed with the conservative declaration.

## Decision Log

- 2026-10-15: Plan created from the design review handover and the maintainer's
  answers to its decision markers (D1–D7) plus follow-up decisions (D-PRIO, D8–D14).
  Changes from the handover: no `ThaiMeEngine` compatibility wrapper (D9); C ABI
  deleted rather than patched (D8, F11 null check dropped); runtime loading moved from
  WS3 into WS1 (D10); best-effort output for untileable input moved from WS4 into WS2;
  D3 resolved to discard; D7 resolved to three modes.
- 2026-10-15: Open question 1 resolved (keep raw n-gram TSVs). Corrected the earlier
  claim that they were unfiltered: the count stage already drops n-grams with count 1.
- 2026-10-15: Data license scope settled with the maintainer: the license files are
  attached to the existing v1.0.0 release (no new data release), the license covers
  release artifacts and corpus-derived benchmarks, and no release-script license guard.
- 2026-10-15: The maintainer approved curl-only data fetching.

## Deviations

- 2026-10-15: WS0 replaced `gh` with `curl` as the only download path, instead of a
  gh→curl fallback. Release assets are public; an installed but unauthenticated `gh`
  fails where `curl` works; one path is simpler. The `GH_TOKEN` environment variable
  was removed from `ci.yml` and `deploy-web.yml`. Approved by the maintainer.

## References

- IBus engine API: https://ibus.github.io/docs/ibus-1.5/IBusEngine.html
- IBus Wayland desktop setup: https://github.com/ibus/ibus/wiki/WaylandDesktop
- IBus `PREEDIT_COMMIT` ignored on GNOME native Wayland: https://github.com/ibus/ibus/issues/2792
- ibus-rime focus/reset handling: https://github.com/rime/ibus-rime/blob/master/rime_engine.c
- ibus-mozc input-mode properties: https://github.com/google/mozc/blob/master/src/unix/ibus/property_handler.cc
- ibus-hangul focus/reset handling: https://github.com/libhangul/ibus-hangul/blob/main/src/engine.c
- Fcitx5 default hotkeys: https://github.com/fcitx/fcitx5/blob/master/src/lib/fcitx/globalconfig.cpp
- librush (pure-Rust IBus engine, reference only): https://github.com/fm-elpac/librush
- zbus: https://github.com/dbus2/zbus
- Fedora change, IBus 1.5.35: https://discussion.fedoraproject.org/t/f45-change-proposal-ibus-1-5-35-self-contained/197425
- kime (Rust IME reference): https://github.com/Riey/kime

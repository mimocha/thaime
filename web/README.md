# THAIME Web Demo

Browser-based demo of the THAIME Latin-to-Thai input method engine. The real Rust engine runs client-side via WebAssembly - no server required.

## Quick Start

**Prerequisites:** Rust toolchain, [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/), Node.js 22+.

```bash
# 1. Build the WASM package (from repo root)
wasm-pack build crates/thaime_wasm --target web

# 2. Copy the dictionary blob
mkdir -p web/public/dict
cp crates/thaime_wasm/dict/thaime.dict web/public/dict/

# 3. Install dependencies and start dev server
cd web
npm install
npm run dev
```

The dev server starts at `http://localhost:5173`.

## Build for Production

```bash
npm run build    # outputs to web/dist/
npm run preview  # preview the production build locally
```

The `prebuild` script automatically copies the dictionary blob into `public/dict/`.

## Project Structure

```
web/
├── index.html                      # Entry point
├── package.json                    # thaime_wasm linked via file: dependency
├── vite.config.ts
├── tsconfig.json
├── public/
│   └── dict/
│       └── thaime.dict             # Combined dictionary blob (gitignored, copied at build)
└── src/
    ├── main.tsx                    # React root
    ├── App.tsx                     # Root layout
    ├── engine/
    │   ├── wasm-loader.ts          # WASM + dictionary fetch & init
    │   └── engine-bridge.ts        # Typed TS interface over WASM calls
    ├── hooks/
    │   └── useIME.ts               # IME state machine (composing, candidates, commit)
    ├── components/
    │   ├── IMEInput.tsx            # Text input area with preedit + cursor
    │   ├── CandidateList.tsx       # Dropdown candidate selector
    │   ├── PreeditDisplay.tsx      # Underlined Latin composition text
    │   └── InfoPopover.tsx         # "What is THAIME?" help dialog
    └── styles/
        └── app.css                 # All styles (dark theme)
```

## How It Works

1. **WASM module** (`thaime_wasm`) wraps the Rust `thaime_engine` crate via `wasm-bindgen`
2. **Dictionary blob** (`thaime.dict`) is fetched at page load and passed to the WASM engine constructor
3. **`useIME` hook** manages the IME state machine - translates keyboard events into engine API calls (`push_key`, `pop_key`, `commit`, `reset`)
4. **Components** render the preedit buffer, candidate dropdown, and committed Thai text

The engine runs entirely in the browser. No data is sent to any server.

## WASM Package Wiring

The WASM package is consumed as a local npm dependency:

```json
"thaime_wasm": "file:../crates/thaime_wasm/pkg"
```

After `wasm-pack build`, the `pkg/` directory is a valid npm package with JS glue, `.wasm` binary, and TypeScript definitions. Vite resolves imports from `'thaime_wasm'` to this local package.

## Deployment

Automated via GitHub Actions (`.github/workflows/deploy-web.yml`). On push to `main`:

1. Build WASM with `wasm-pack`
2. Copy dictionary blob to `web/public/dict/`
3. `npm install` + `npm run build`
4. Deploy `web/dist/` to GitHub Pages

The entire site is static files - HTML, JS, CSS, WASM binary, and dictionary blob.

## License

MPL-2.0

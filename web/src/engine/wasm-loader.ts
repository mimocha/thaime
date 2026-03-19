/*
 * SPDX-License-Identifier: MPL-2.0
 */

// Async loader: initializes the WASM module and fetches the dictionary blob.
// Supports progress tracking for the dictionary download.

import init, { WasmEngine } from 'thaime_wasm';

let engineInstance: WasmEngine | null = null;
let initPromise: Promise<WasmEngine> | null = null;

/**
 * Initialize the WASM engine. Safe to call multiple times — returns the same
 * instance after the first successful init.
 *
 * @param onProgress Optional callback reporting download progress (loaded bytes, total bytes).
 */
export function loadEngine(
  onProgress?: (loaded: number, total: number) => void,
): Promise<WasmEngine> {
  if (engineInstance) return Promise.resolve(engineInstance);
  if (initPromise) return initPromise;

  initPromise = (async () => {
    // Initialize the WASM module (fetches .wasm binary automatically)
    await init();

    // Fetch the combined dictionary blob from public/dict/
    const base = import.meta.env.BASE_URL ?? '/';
    const dictFile = import.meta.env.VITE_DICT_FILE ?? 'thaime.dict';
    const resp = await fetch(`${base}dict/${dictFile}`);
    if (!resp.ok) {
      throw new Error(`Failed to fetch dictionary: ${resp.status} ${resp.statusText}`);
    }

    let dictBytes: Uint8Array;

    // Use ReadableStream for progress tracking when available
    if (onProgress && resp.body) {
      const contentLength = resp.headers.get('Content-Length');
      const total = contentLength ? parseInt(contentLength, 10) : 10_000_000; // fallback ~10MB

      const reader = resp.body.getReader();
      const chunks: Uint8Array[] = [];
      let loaded = 0;

      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        loaded += value.length;
        onProgress(loaded, total);
      }

      // Combine chunks into a single Uint8Array
      dictBytes = new Uint8Array(loaded);
      let offset = 0;
      for (const chunk of chunks) {
        dictBytes.set(chunk, offset);
        offset += chunk.length;
      }
    } else {
      dictBytes = new Uint8Array(await resp.arrayBuffer());
    }

    engineInstance = new WasmEngine(dictBytes);

    // Fire-and-forget: fetch n-gram data in the background
    fetchNgram(engineInstance).catch((err) => {
      console.warn('N-gram loading failed (dict-only mode):', err);
    });

    return engineInstance;
  })();

  initPromise.catch(() => {
    // Allow retry on failure
    initPromise = null;
  });

  return initPromise;
}

/**
 * Fetch and load the n-gram binary in the background.
 * Does not block engine availability — the UI works dict-only until this completes.
 */
async function fetchNgram(engine: WasmEngine): Promise<void> {
  const base = import.meta.env.BASE_URL ?? '/';
  const ngramFile = import.meta.env.VITE_NGRAM_FILE ?? 'thaime_ngram_v1_mc20.bin';
  const resp = await fetch(`${base}dict/${ngramFile}`);
  if (!resp.ok) {
    throw new Error(`Failed to fetch ngram: ${resp.status} ${resp.statusText}`);
  }
  const bytes = new Uint8Array(await resp.arrayBuffer());
  engine.load_ngram(bytes);
  console.log(`N-gram loaded: ${ngramFile} (${bytes.length} bytes)`);
}

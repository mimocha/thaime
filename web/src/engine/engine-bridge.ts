// SPDX-License-Identifier: MPL-2.0
// Typed TypeScript interface wrapping the raw WASM engine calls.

import { WasmEngine } from 'thaime_wasm';
import { loadEngine } from './wasm-loader';

export interface Candidate {
  thai: string;
  score: number;
}

export interface ThaiEngine {
  pushKey(ch: string): boolean;
  popKey(): boolean;
  candidates(): Candidate[];
  commit(index: number): string | null;
  reset(): void;
  preedit(): string;
}

class EngineBridge implements ThaiEngine {
  constructor(private wasm: WasmEngine) {}

  pushKey(ch: string): boolean {
    if (ch.length !== 1) return false;
    return this.wasm.push_key(ch);
  }

  popKey(): boolean {
    return this.wasm.pop_key();
  }

  candidates(): Candidate[] {
    const raw = this.wasm.candidates();
    if (!Array.isArray(raw)) return [];
    return raw as Candidate[];
  }

  commit(index: number): string | null {
    return this.wasm.commit(index) ?? null;
  }

  reset(): void {
    this.wasm.reset();
  }

  preedit(): string {
    return this.wasm.preedit();
  }
}

/**
 * Async factory — loads WASM + dictionary, returns initialized engine.
 *
 * @param onProgress Optional callback for dictionary download progress.
 */
export async function createEngine(
  onProgress?: (loaded: number, total: number) => void,
): Promise<ThaiEngine> {
  const wasm = await loadEngine(onProgress);
  return new EngineBridge(wasm);
}

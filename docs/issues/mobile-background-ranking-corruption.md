# Issue: Ranking Corruption After Mobile Background/Resume

**Status:** Open — not yet reproduced reliably
**Severity:** Low (transient, refresh fixes it)
**Affected:** Android mobile browsers (Chrome observed)

## Symptoms

1. Load the web demo, verify "malongchaithaime" produces "มาลองใช้ไทยมี" as the #1 candidate
2. Switch away to another app for a while (minutes, not seconds)
3. Return to the web demo tab
4. The conversion for the same input now produces a different top candidate (e.g., "มาโล่งใจไทยมี")
5. Refreshing the page restores correct behavior

The wrong result is a *valid alternative segmentation*, not garbage — suggesting the statistical scoring data (n-gram model) has been corrupted, shifting which word boundaries the Viterbi algorithm prefers.

## Leading Theory: WASM Linear Memory Corruption Under Memory Pressure

Android aggressively reclaims memory from backgrounded browser tabs. The WASM linear memory holds the trie data structure and n-gram binary blob as raw bytes. If Android evicts and zero-fills those memory pages, the engine continues to "work" but produces different rankings because its statistical data is corrupted.

Key evidence supporting this theory:
- Refreshing the page fixes the issue (re-loads all WASM memory from scratch)
- The wrong result is a different valid segmentation, not an error — consistent with corrupted frequency/score data rather than structural damage
- The n-gram blob is the largest contiguous allocation in WASM memory, making it the most likely victim of page eviction
- No `visibilitychange` or lifecycle handlers exist to detect or recover from this

## Alternative Theories (less likely)

### Stale committed_context
The engine maintains a `committed_context` vector (max 2 words) for bigram/trigram scoring. If previous interactions left context words that happen to shift scoring for "malongchaithaime", this could change the top candidate. However, this context persists across all in-session interactions and wouldn't change behavior specifically after backgrounding.

### N-gram fetch race condition
The n-gram data is loaded asynchronously after the engine initializes (`wasm-loader.ts:70-72`). If the page is backgrounded during the narrow window of initial n-gram fetch, the fetch could fail silently. The engine would then operate in dict-only mode (no n-gram scoring), which produces different rankings. However, this only applies during initial page load, not after the engine is fully initialized.

### JIT deoptimization
After a long background period, the browser may discard JIT-compiled WASM code and recompile on resume. WASM semantics are deterministic, so this should not change behavior. Browser bugs are theoretically possible but extremely unlikely.

## Diagnostic Steps

### Step 1: Canary health check on visibilitychange

Add a `visibilitychange` event listener that runs a known input through the engine on tab resume and checks the output:

```typescript
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible' && engineInstance) {
    // Save current state
    const savedPreedit = engineInstance.preedit();

    // Canary test: push known input, check top candidate
    engineInstance.reset();
    for (const ch of 'malongchaithaime') {
      engineInstance.push_key(ch);
    }
    const candidates = engineInstance.candidates();
    const topCandidate = candidates.length > 0 ? candidates[0].thai : null;

    // Restore state
    engineInstance.reset();
    for (const ch of savedPreedit) {
      engineInstance.push_key(ch);
    }

    if (topCandidate !== 'มาลองใช้ไทยมี') {
      console.error(`[THAIME] Canary check FAILED on resume. Got: ${topCandidate}`);
      // Engine state may be corrupted
    }
  }
});
```

Note: this canary check has limitations — it destroys and rebuilds engine state, and doesn't restore `committed_context`. A more robust version would need engine support for state snapshot/restore.

### Step 2: Test with n-gram disabled

Skip the `fetchNgram()` call (e.g., via a query parameter `?no-ngram=1`). If the bug does not occur without n-grams, it confirms the n-gram blob in WASM memory is the corruption target.

### Step 3: Checksum integrity check

Store a CRC32 or simple checksum of the n-gram binary data at load time (in JS, outside WASM memory). On `visibilitychange` to `'visible'`, read the n-gram bytes back from WASM memory and compare checksums. This directly detects bit-level corruption.

This requires exposing a method on the WASM engine to return the raw n-gram bytes (or a checksum computed Rust-side).

## Potential Recovery Strategy

If corruption is confirmed via diagnostics:

1. **Retain blob copies in JS memory**: Keep `dictBytes` and `ngramBytes` in JS-side `ArrayBuffer` variables (outside WASM linear memory) after passing them to the engine. Currently these are discarded after initialization.

2. **Re-instantiate on corruption**: On `visibilitychange` to `'visible'`, run the canary check. If it fails, create a new `WasmEngine` instance from the retained blobs. This doubles memory usage for those blobs (~10-15 MB) but enables instant recovery without re-fetching.

3. **State restoration**: After re-instantiation, replay the current preedit buffer into the new engine. The `committed_context` would be lost, but this is acceptable since it only holds the last 2 words and affects ranking marginally.

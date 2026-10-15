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

The wrong result is a *valid alternative segmentation*, not garbage. It is exactly what the engine returns with **no n-gram data**: unigram-only ranking puts มาโล่งใจไทยมี first for `malongchaithaime` (checked 2026-10-15 with the v1.0.0 data).

## Leading Theory: Tab Discarded, Reloaded Without N-gram Data

Android discards backgrounded tabs under memory pressure. Returning to a discarded tab reloads the page. On reload, the engine starts in dict-only mode and the n-gram binary (~10 MB) is fetched in the background (`wasm-loader.ts`, `fetchNgram`). If that fetch fails, is slow, or is throttled while the page is still resuming, the engine ranks unigram-only. A fetch failure is logged with `console.warn` only, so nothing on the page shows it. Typing again before the fetch completes gives the wrong top candidate.

Evidence:
- The observed wrong result matches the no-n-gram ranking exactly.
- A reloaded tab looks the same as the original to the user (same URL, demo restored).
- Refreshing (with the network now active) loads the n-gram data and fixes the result.

## Less Likely Theories

### WASM linear memory corruption under memory pressure
The original leading theory: Android evicts and zero-fills pages of the WASM linear memory holding the n-gram data. Unlikely: the OS does not silently zero-fill a live process's memory. A backgrounded tab is either kept intact or discarded and reloaded. Corrupted (rather than missing) scores would also be unlikely to land exactly on the no-n-gram result.

### Stale committed_context
The engine maintains a `committed_context` vector (max 2 words) for bigram/trigram scoring. If previous interactions left context words that happen to shift scoring for "malongchaithaime", this could change the top candidate. However, this context persists across all in-session interactions and wouldn't change behavior specifically after backgrounding.

### JIT deoptimization
After a long background period, the browser may discard JIT-compiled WASM code and recompile on resume. WASM semantics are deterministic, so this should not change behavior. Browser bugs are theoretically possible but extremely unlikely.

## Diagnostic Steps

### Step 0: Log n-gram state on resume (implemented)

The web demo logs `[THAIME] Tab visible; n-gram loaded: <true|false>` (via `WasmEngine.has_ngram()`) whenever the tab becomes visible. Reproduce with remote debugging (`chrome://inspect`) attached: `false` after a resume confirms the leading theory; a `N-gram loading failed` warning shows why. If it confirms, the fix is to surface the dict-only state in the UI and retry the fetch.

The steps below apply only if the log shows `true` while the ranking is wrong.

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

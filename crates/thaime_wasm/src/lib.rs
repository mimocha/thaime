// SPDX-License-Identifier: MPL-2.0

//! THAIME WebAssembly bindings.
//!
//! Thin wrapper over `thaime_engine` exposing a JS-callable API via
//! `wasm-bindgen`. The engine runs entirely client-side in the browser.

use thaime_engine::{InputMode, KeyResult, ThaiMeEngine};
use wasm_bindgen::prelude::*;

/// WASM-exposed engine handle.
///
/// Wraps [`ThaiMeEngine`] and provides a JS-friendly API.
#[wasm_bindgen]
pub struct WasmEngine {
    inner: ThaiMeEngine,
}

#[wasm_bindgen]
impl WasmEngine {
    /// Initialize from a combined dictionary blob.
    ///
    /// The blob format is:
    /// `[4 bytes: trie_len as u32 LE][trie_bytes][metadata_bytes]`
    ///
    /// This is the `thaime.dict` file fetched at page load.
    #[wasm_bindgen(constructor)]
    pub fn new(dict_blob: &[u8]) -> Result<WasmEngine, JsValue> {
        let min_len = 4;
        if dict_blob.len() < min_len {
            return Err(JsValue::from_str("dictionary blob too small"));
        }

        let trie_len = u32::from_le_bytes(dict_blob[..4].try_into().unwrap()) as usize;

        if dict_blob.len() < 4 + trie_len {
            return Err(JsValue::from_str(
                "dictionary blob truncated: trie_len exceeds blob size",
            ));
        }

        let trie_bytes = dict_blob[4..4 + trie_len].to_vec();
        let metadata_bytes = &dict_blob[4 + trie_len..];

        let inner = ThaiMeEngine::from_dict_bytes(trie_bytes, metadata_bytes);
        Ok(WasmEngine { inner })
    }

    /// Append a Latin character to the input buffer.
    ///
    /// Returns `true` if the key was consumed by the engine.
    pub fn push_key(&mut self, ch: char) -> bool {
        self.inner.push_key(ch)
    }

    /// Remove the last character from the input buffer (backspace).
    ///
    /// Returns `true` if there was a character to remove.
    pub fn pop_key(&mut self) -> bool {
        self.inner.pop_key()
    }

    /// Get the current ranked candidate list as a JS array.
    ///
    /// Each element is an object with `thai` (string) and `score` (number).
    pub fn candidates(&self) -> JsValue {
        let candidates: Vec<_> = self
            .inner
            .candidates()
            .iter()
            .map(|c| CandidateJs {
                thai: c.thai.clone(),
                score: c.score,
            })
            .collect();
        serde_wasm_bindgen::to_value(&candidates).unwrap_or(JsValue::NULL)
    }

    /// Commit the candidate at the given index.
    ///
    /// Returns the Thai text string, or `null` if the index is out of bounds.
    pub fn commit(&mut self, index: usize) -> Option<String> {
        self.inner.commit(index)
    }

    /// Reset the engine state, clearing the input buffer and candidates.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Get the current Latin input buffer (for preedit display).
    pub fn preedit(&self) -> String {
        self.inner.preedit().to_string()
    }

    /// Hot-load n-gram data from a binary blob.
    ///
    /// Parses the binary and enables context-dependent ranking.
    /// Can be called after construction to upgrade dict-only mode.
    pub fn load_ngram(&mut self, ngram_blob: &[u8]) -> Result<(), JsValue> {
        let ngram = thaime_engine::ngram::NgramData::from_bytes(ngram_blob)
            .map_err(|e| JsValue::from_str(&format!("ngram parse error: {e}")))?;
        self.inner.load_ngram(ngram);
        Ok(())
    }

    /// Get the current input mode.
    ///
    /// Returns `"romanization"`, `"kedmanee"`, or `"latin"`.
    pub fn mode(&self) -> String {
        match self.inner.mode() {
            InputMode::Romanization => "romanization".into(),
            InputMode::Kedmanee => "kedmanee".into(),
            InputMode::Latin => "latin".into(),
        }
    }

    /// Set the input mode. Resets any active composition when the mode changes.
    ///
    /// Accepts `"romanization"`, `"kedmanee"`, or `"latin"`.
    pub fn set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        let m = match mode {
            "romanization" => InputMode::Romanization,
            "kedmanee" => InputMode::Kedmanee,
            "latin" => InputMode::Latin,
            _ => return Err(JsValue::from_str(&format!("unknown mode: {mode}"))),
        };
        self.inner.set_mode(m);
        Ok(())
    }

    /// Process a key with mode-aware behavior.
    ///
    /// Returns:
    /// - `null` — key rejected (not handled)
    /// - `""` (empty string) — key consumed into composition buffer (Romanization)
    /// - `"ฟ"` (non-empty string) — committed character (Kedmanee/Latin)
    pub fn process_key(&mut self, ch: char) -> Option<String> {
        match self.inner.process_key(ch) {
            KeyResult::Rejected => None,
            KeyResult::Consumed => Some(String::new()),
            KeyResult::Committed(c) => Some(c.to_string()),
        }
    }
}

/// Lightweight serializable candidate for JS consumption.
#[derive(serde::Serialize)]
struct CandidateJs {
    thai: String,
    score: f64,
}

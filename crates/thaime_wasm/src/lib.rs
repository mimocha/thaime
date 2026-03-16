// SPDX-License-Identifier: MPL-2.0

//! THAIME WebAssembly bindings.
//!
//! Thin wrapper over `thaime_engine` exposing a JS-callable API via
//! `wasm-bindgen`. The engine runs entirely client-side in the browser.

use thaime_engine::ThaiMeEngine;
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
}

/// Lightweight serializable candidate for JS consumption.
#[derive(serde::Serialize)]
struct CandidateJs {
    thai: String,
    score: f64,
}

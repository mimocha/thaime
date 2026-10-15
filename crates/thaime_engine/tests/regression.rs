// SPDX-License-Identifier: MPL-2.0

//! Headless regression snapshot over `tests/harness-regression.toml`.
//!
//! Every case must return its recorded top-1 candidate: `snapshot_top1` for
//! known targets, `expected_thai` otherwise. Uses the same setup as the TUI
//! regression mode: embedded dictionary, pinned n-gram binary, default
//! ranking params, no context.

use serde::Deserialize;
use thaime_engine::ngram::NgramData;
use thaime_engine::ranking::{rank_candidates, RankingParams};
use thaime_engine::trie::Dictionary;

const CASES_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/harness-regression.toml"
);

#[derive(Deserialize)]
struct CaseFile {
    test: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    input: String,
    expected_thai: String,
    snapshot_top1: Option<String>,
}

#[test]
fn regression_snapshot() {
    let ngram_path = env!(
        "THAIME_NGRAM_PATH",
        "n-gram binary not found at build time; run ./scripts/fetch-data.sh"
    );
    let ngram_bytes = std::fs::read(ngram_path)
        .unwrap_or_else(|e| panic!("failed to read n-gram binary {ngram_path}: {e}"));
    let ngram = NgramData::from_bytes(&ngram_bytes).expect("n-gram binary is malformed");
    let dictionary = Dictionary::from_embedded();
    let params = RankingParams::default();

    let content = std::fs::read_to_string(CASES_PATH)
        .unwrap_or_else(|e| panic!("failed to read {CASES_PATH}: {e}"));
    let cases: CaseFile = toml::from_str(&content).expect("malformed regression TOML");
    assert!(
        !cases.test.is_empty(),
        "no regression cases in {CASES_PATH}"
    );

    let mismatches: Vec<String> = cases
        .test
        .iter()
        .filter_map(|case| {
            let want = case.snapshot_top1.as_ref().unwrap_or(&case.expected_thai);
            let result = rank_candidates(&case.input, &dictionary, Some(&ngram), &[], &params);
            let got = result.candidates.first().map(|c| c.thai.as_str());
            (got != Some(want.as_str())).then(|| {
                format!(
                    "{}: snapshot {want}, got {}",
                    case.input,
                    got.unwrap_or("<no candidates>")
                )
            })
        })
        .collect();

    assert!(
        mismatches.is_empty(),
        "top-1 changed for {} of {} cases (update tests/harness-regression.toml \
         if the change is intended):\n{}",
        mismatches.len(),
        cases.test.len(),
        mismatches.join("\n")
    );
}

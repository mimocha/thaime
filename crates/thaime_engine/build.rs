// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // --- C header generation (cbindgen) ---
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(cbindgen::Config::from_file("../../cbindgen.toml").unwrap())
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file("../../target/thaime.h");

    // --- Dictionary path discovery ---
    // Resolve versioned dict files (e.g. trie-v1_0_0.bin) with fallback to
    // unversioned names (trie.bin) for dev builds.
    let workspace_root = PathBuf::from(&crate_dir)
        .join("../..")
        .canonicalize()
        .unwrap();
    let dict_dir = workspace_root.join("data/dict");

    let vtag = read_version_tag(&workspace_root);

    let trie_path = resolve_dict_path(&dict_dir, "trie", &vtag, "bin");
    let metadata_path = resolve_dict_path(&dict_dir, "metadata", &vtag, "bin");

    println!("cargo:rustc-env=THAIME_TRIE_PATH={}", trie_path.display());
    println!(
        "cargo:rustc-env=THAIME_METADATA_PATH={}",
        metadata_path.display()
    );

    // Re-run when dict files or data config change
    println!("cargo:rerun-if-changed={}", dict_dir.display());
    println!("cargo:rerun-if-changed=../../thaime-data.toml");

    // --- N-gram binary discovery ---
    // Use the pinned filename from thaime-data.toml when available,
    // falling back to directory scanning for dev builds.
    let input_dir = workspace_root.join("data/input");
    let ngram_found = if let Some(ngram_file) = read_ngram_filename(&workspace_root) {
        let ngram_path = input_dir.join(&ngram_file);
        if ngram_path.exists() {
            println!("cargo:rustc-env=THAIME_NGRAM_PATH={}", ngram_path.display());
            true
        } else {
            false
        }
    } else {
        false
    };

    // Fallback: scan directory for highest min_count variant
    if !ngram_found {
        if let Ok(entries) = std::fs::read_dir(&input_dir) {
            let mut candidates: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("thaime_ngram_v1_mc") && n.ends_with(".bin"))
                        .unwrap_or(false)
                })
                .map(|e| e.path())
                .collect();
            candidates.sort();
            // Prefer highest min_count variant (last alphabetically)
            if let Some(path) = candidates.last() {
                println!("cargo:rustc-env=THAIME_NGRAM_PATH={}", path.display());
            }
        }
    }
    println!("cargo:rerun-if-changed={}", input_dir.display());
}

/// Read the NLP data version from thaime-data.toml and convert to a
/// version tag (e.g. "v1.0.0" → "v1_0_0").
/// Falls back to workspace Cargo.toml version if thaime-data.toml is missing.
fn read_version_tag(workspace_root: &std::path::Path) -> String {
    let config_path = workspace_root.join("thaime-data.toml");
    if let Ok(contents) = std::fs::read_to_string(&config_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.starts_with("version") && line.contains('=') {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let version = &line[start + 1..start + 1 + end];
                        // "v1.0.0" → "v1_0_0"
                        return version.replace('.', "_");
                    }
                }
            }
        }
    }

    // Fallback: read from workspace Cargo.toml
    let cargo_toml = std::fs::read_to_string(workspace_root.join("Cargo.toml"))
        .expect("Failed to read workspace Cargo.toml");

    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with("version") && line.contains('=') {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let version = &line[start + 1..start + 1 + end];
                    return format!("v{}", version.replace('.', "_"));
                }
            }
        }
    }
    panic!("Could not find version in thaime-data.toml or workspace Cargo.toml");
}

/// Read the n-gram filename from thaime-data.toml (without .gz suffix).
fn read_ngram_filename(workspace_root: &std::path::Path) -> Option<String> {
    let config_path = workspace_root.join("thaime-data.toml");
    let contents = std::fs::read_to_string(&config_path).ok()?;

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with("ngram") && line.contains("file") {
            // Extract: file = "thaime_ngram_v1_mc15.bin.gz"
            if let Some(start) = line.find("file") {
                let rest = &line[start..];
                if let Some(q1) = rest.find('"') {
                    if let Some(q2) = rest[q1 + 1..].find('"') {
                        let filename = &rest[q1 + 1..q1 + 1 + q2];
                        // Strip .gz suffix
                        return Some(filename.strip_suffix(".gz").unwrap_or(filename).to_string());
                    }
                }
            }
        }
    }
    None
}

/// Find the dict file, preferring versioned name over unversioned fallback.
fn resolve_dict_path(dict_dir: &std::path::Path, stem: &str, vtag: &str, ext: &str) -> PathBuf {
    // Try versioned first: e.g. trie-v1_0_0.bin
    let versioned = dict_dir.join(format!("{}-{}.{}", stem, vtag, ext));
    if versioned.exists() {
        return versioned;
    }

    // Fallback to unversioned: e.g. trie.bin
    let unversioned = dict_dir.join(format!("{}.{}", stem, ext));
    if unversioned.exists() {
        return unversioned;
    }

    panic!(
        "Dictionary file not found: tried {} and {}",
        versioned.display(),
        unversioned.display()
    );
}

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
    // Resolve versioned dict files (e.g. trie-v0_4_2.bin) with fallback to
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

    // Re-run when dict files change
    println!("cargo:rerun-if-changed={}", dict_dir.display());

    // --- N-gram binary discovery ---
    let input_dir = workspace_root.join("data/input");
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
    println!("cargo:rerun-if-changed={}", input_dir.display());
}

/// Read the workspace version from the root Cargo.toml and convert to a
/// version tag (e.g. "0.4.2" → "v0_4_2").
fn read_version_tag(workspace_root: &std::path::Path) -> String {
    let cargo_toml = std::fs::read_to_string(workspace_root.join("Cargo.toml"))
        .expect("Failed to read workspace Cargo.toml");

    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with("version") && line.contains('=') {
            // Extract the quoted version string
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let version = &line[start + 1..start + 1 + end];
                    return format!("v{}", version.replace('.', "_"));
                }
            }
        }
    }
    panic!("Could not find version in workspace Cargo.toml");
}

/// Find the dict file, preferring versioned name over unversioned fallback.
fn resolve_dict_path(dict_dir: &std::path::Path, stem: &str, vtag: &str, ext: &str) -> PathBuf {
    // Try versioned first: e.g. trie-v0_4_2.bin
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

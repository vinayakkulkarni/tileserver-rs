//! Guard: public files must not name private repositories or products.
//!
//! The forbidden names are stored only as SHA-256 hex digests of the
//! lowercase word, hostname, or `owner/repo` pair, so this test never
//! spells them out. Each scanned file is split into candidate tokens, and
//! the digest of every candidate is compared against the list.

use std::collections::BTreeSet;
use std::path::PathBuf;

use regex::Regex;
use sha2::{Digest, Sha256};

/// SHA-256 digests of forbidden lowercase words, hostnames, and repo pairs.
const FORBIDDEN_DIGESTS: &[&str] = &[
    "72165796b67119a7b57759b350a2663346c1cefbf4a17c88d9bbb2fe7f35d478",
    "7559a9a1a2623c9ec9bb334230a1005eaa7f4aebbf8fb9266313e49d57f84fea",
    "45df6e6712bdafb88a8b133ffbcf435d5bc458b5e90de18daf6f2245f540613f",
    "f1a3ec0ea4f47cd20634df7acee8a0583eaffc3772dfefa33989d0971f1549c0",
    "e4f743a670ac2a340e23542b6a72869b339e418de43406685bcd2705d1b694a5",
    "85d808232e01037fa6a8055cd090c1173854d72f6513f23ebcee6a3260d31660",
    "e2520c990efb01e3dd6542f071a56d7b1c8eb66dad3da1e89db544c56ddee0cf",
    "43daf55a26ade264c25dfcd151281ce46940f5508a573b17b2ba18d4fe41f6f5",
    "dc50f77b1cd9db35fd7b734401cee296869fe08612ae07904b6309f938d884a2",
    "0b9a5d20aa9c44fac80f4b7ae6ba685b59d71caf16301f1354eb4b148d623cc0",
    "15a489db9e9f88a9aed5059b9c3bdd12db4dad837cb14b06ea09cbc31f279430",
    "0c918cc13d6af28552094e90198f77c9072783088938c69c946e2c99fa95419f",
    "7dc93e700df37018813392832eaf7efd4ebbd84a820c54068764b21a9fa7d505",
];

/// Public files to scan, relative to the workspace root.
const GUARDED_FILES: &[&str] = &[
    "Cargo.toml",
    "README.md",
    "CLAUDE.md",
    "benchmarks/postgres/Dockerfile",
    "crates/tileserver-rs/CHANGELOG.md",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sha256_hex(input: &str) -> String {
    Sha256::digest(input.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Every lowercase word, hostname (and its parent domains), and
/// `owner/repo` pair found in `text`.
fn candidates(text: &str) -> BTreeSet<String> {
    let token_re = Regex::new(r"[a-z0-9][a-z0-9._/-]*[a-z0-9]|[a-z0-9]").unwrap();
    let word_re = Regex::new(r"[a-z0-9]+").unwrap();
    let lower = text.to_lowercase();
    let mut out = BTreeSet::new();

    for token in token_re.find_iter(&lower).map(|m| m.as_str()) {
        out.insert(token.to_string());
        let segments: Vec<&str> = token.split('/').filter(|s| !s.is_empty()).collect();
        for pair in segments.windows(2) {
            out.insert(format!("{}/{}", pair[0], pair[1]));
        }
        for segment in &segments {
            out.insert((*segment).to_string());
            let labels: Vec<&str> = segment.split('.').collect();
            for start in 0..labels.len() {
                out.insert(labels[start..].join("."));
            }
        }
        for word in word_re.find_iter(token) {
            out.insert(word.as_str().to_string());
        }
    }
    out
}

#[test]
fn public_files_do_not_name_private_repos() {
    let root = workspace_root();
    let mut hits = Vec::new();

    for rel in GUARDED_FILES {
        let path = root.join(rel);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        for (line_no, line) in text.lines().enumerate() {
            let found = candidates(line)
                .iter()
                .any(|candidate| FORBIDDEN_DIGESTS.contains(&sha256_hex(candidate).as_str()));
            if found {
                hits.push(format!("{rel}:{}", line_no + 1));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "private repo or product names found at:\n{}",
        hits.join("\n")
    );
}

#[test]
fn digest_matcher_catches_owner_repo_pairs_and_subdomains() {
    // Self-check with harmless names so the matcher logic is covered
    // without spelling out a forbidden name.
    let pair = sha256_hex("acme/widgets");
    let host = sha256_hex("acme.example");
    let found = candidates("See https://docs.acme.example and acme/widgets#4.");
    assert!(found.iter().any(|c| sha256_hex(c) == pair));
    assert!(found.iter().any(|c| sha256_hex(c) == host));
}

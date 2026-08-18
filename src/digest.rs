//! SHA-256 helpers. Digests are how this tool refuses to trust a summary:
//! evidence, locks and pinned sources are all verified by recomputation.

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn file(path: &Path) -> Result<String> {
    Ok(hex(&std::fs::read(path)?))
}

/// Digest of a set of files as a set: order-independent in input, stable in
/// output. This is what makes evidence expire when the code it certified moves.
pub fn tree(entries: &mut [(String, String)]) -> String {
    entries.sort();
    let joined = entries
        .iter()
        .map(|(path, digest)| format!("{path} {digest}"))
        .collect::<Vec<_>>()
        .join("\n");
    hex(joined.as_bytes())
}

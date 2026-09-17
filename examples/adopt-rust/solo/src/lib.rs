//! Manifest-free crate: no Cargo.toml anywhere near `solo/`.
//! Proves source-only fallback with deterministic naming.
pub mod net;

/// Joins words for the worker-style digest demo.
pub fn join(words: &[&str]) -> String {
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::join;

    #[test]
    fn join_matches_std_join() {
        assert_eq!(join(&["a", "b"]), "a b");
    }
}

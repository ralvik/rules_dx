//! CXX graph identity fixture library (issue #474).
//!
//! The `#[cxx::bridge]` lives here once the corpus wires `cxx = "=1.0.200"`
//! from the single `crates` graph (full wiring under #499); until then this
//! crate stays buildable with no external deps so the composition shape
//! (Rust `CrateInfo` plus C++ `CcInfo`) is proven without a second graph.

/// Pinned CXX identity version (mirrors `CXX_VERSION` in `cxx_bridge.bzl`).
pub fn identity_version() -> &'static str {
    "1.0.200"
}

/// Adds two integers with documented wrapping semantics.
pub fn add(left: u64, right: u64) -> u64 {
    left.wrapping_add(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_pin() {
        assert_eq!(identity_version(), "1.0.200");
    }

    #[test]
    fn adds_numbers() {
        assert_eq!(add(2, 3), 5);
    }
}

//! Test-only scratch directories: the single test scratch policy.
//!
//! Single scratch policy for the repo (`#651`):
//! * Prod hermetic mirrors stay in `quality_adapter::exec::Scratch`
//!   (scratch-relative resolve/materialize/close with symlink-prefix
//!   guards; `quality_runner` reuses that type via `fresh_scratch`).
//! * Prod per-run temp dirs stay in `dx_cli::plan::create_run_temp_dir`
//!   (nonce plus `dx-run-` prefix discipline, explicit-parent `tempdir_in`).
//! * Every unit-test scratch dir uses this crate's [`scratch`]: a unique
//!   prefixed directory under the ambient temp base that auto-cleans on
//!   drop. It replaces the repeated
//!   `tempfile::Builder::new().prefix(..).tempdir_in(..).expect(..)`
//!   chains in `dx_cli`, `dx_env`, `dx_adopt`, and `dx_process` tests.
//!   `dx_cli::exec::test_support::temp_dir` is a thin wrapper over
//!   [`scratch`] (same prefix discipline, `dx-exec-test-`), not a third
//!   policy: pid-suffixed persistent dirs are gone.
//!
//! Dependency evaluation (adopted): creation uses the upstream
//! `tempfile` crate directly (`Builder::new().prefix(..).tempdir()` plus the
//! `TempDir` handle); this crate stays a thin test-only discipline wrapper so
//! call sites name one prefix-plus-auto-clean path instead of repeating the
//! builder chain.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

/// Scratch-directory handle; re-exported so tests name the type without
/// depending on `tempfile` directly.
pub use tempfile::TempDir;

/// Creates a unique `prefix`-prefixed scratch directory under the ambient
/// temp base.
///
/// Creation failure (missing temp base) is unreachable in test
/// environments; the fallback names the invariant instead of `expect`.
pub fn scratch(prefix: &str) -> TempDir {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .unwrap_or_else(|err| unreachable!("test scratch creates: {err:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_uses_prefix_and_exists() {
        let dir = scratch("dx-test-scratch-");
        assert!(dir.path().is_dir(), "scratch exists: {:?}", dir.path());
        let name = dir.path().file_name().expect("tempdir has a name");
        assert!(
            name.to_string_lossy().starts_with("dx-test-scratch-"),
            "prefix honored: {name:?}"
        );
    }

    #[test]
    fn scratches_are_unique() {
        let first = scratch("dx-test-scratch-");
        let second = scratch("dx-test-scratch-");
        assert_ne!(first.path(), second.path(), "unique dirs");
    }

    #[test]
    fn scratch_cleans_on_drop() {
        let path = {
            let dir = scratch("dx-test-scratch-");
            let path = dir.path().to_path_buf();
            assert!(path.is_dir(), "exists before drop");
            path
        };
        assert!(!path.exists(), "removed on drop: {path:?}");
    }
}

//! Per-run temporary-directory and nonce helpers.
//!
//! Split from `super` (`plan.rs`): owns [`bep_path`], [`intended_path`],
//! [`run_nonce`], and [`create_run_temp_dir`]. Re-exported through
//! `super` so the public paths stay `crate::plan::{bep_path, ...}`.
//! These are leaf utilities with no planner dependencies.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// BEP stream destination under `temp_dir`, unique per process
/// invocation. `nonce` distinguishes repeated runs inside one process
/// (tests, retries); production callers pass a per-run counter.
pub fn bep_path(temp_dir: &Path, pid: u32, nonce: u64) -> PathBuf {
    temp_dir.join(format!("dx-bep-{pid}-{nonce}.json"))
}

/// Intended-manifest destination under `temp_dir`, unique per process
/// invocation like [`bep_path`]. The wrapper passes it as
/// [`super::GENERATE_ENV_INTENDED`] so the Gazelle extension witnesses
/// its exact BUILD changes there for the finalizer.
pub fn intended_path(temp_dir: &Path, pid: u32, nonce: u64) -> PathBuf {
    temp_dir.join(format!("dx-generate-{pid}-{nonce}.json"))
}

/// Per-process run counter feeding [`run_nonce`]. `Relaxed` suffices:
/// no happens-before edge is needed, only atomicity — every fetch
/// yields a distinct value even under concurrent callers.
///
/// Cross-process uniqueness comes from the [`tempfile`] directory itself
/// (exclusive create with a random suffix); the nonce only needs to be
/// unique within this process because BEP/intended files live inside the
/// unique directory.
static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Fresh nonce for one launcher run.
///
/// A process-local monotonic counter: every call in this process yields a
/// distinct value. Cross-process collisions are harmless — each run owns a
/// unique [`tempfile::TempDir`], so identical nonces in different processes
/// name files in different directories.
pub fn run_nonce() -> u64 {
    RUN_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Creates a fresh unique run directory and returns it with the nonce
/// the caller must forward as [`crate::exec::Env::nonce`] so BEP and
/// intended-manifest paths share the run's uniqueness.
///
/// Uses [`tempfile::Builder`] with prefix `dx-run-`: exclusive create plus
/// internal retry closes the PID-recycle collision window that hand-rolled
/// `create_dir` loops used to cover. The returned [`tempfile::TempDir`]
/// auto-cleans on drop; callers that need a removal warning should call
/// [`tempfile::TempDir::close`] explicitly.
pub fn create_run_temp_dir(base: &Path) -> std::io::Result<(tempfile::TempDir, u64)> {
    let dir = tempfile::Builder::new()
        .prefix("dx-run-")
        .tempdir_in(base)?;
    let nonce = run_nonce();
    Ok((dir, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bep_paths_are_unique_per_run() {
        let dir = Path::new("/tmp/dx");
        assert_eq!(
            bep_path(dir, 42, 0),
            PathBuf::from("/tmp/dx/dx-bep-42-0.json")
        );
        assert_ne!(bep_path(dir, 42, 0), bep_path(dir, 42, 1));
        assert_ne!(bep_path(dir, 42, 0), bep_path(dir, 43, 0));
    }

    #[test]
    fn run_nonces_are_unique_in_process() {
        use std::collections::HashSet;
        let seen: HashSet<u64> = (0..512).map(|_| run_nonce()).collect();
        assert_eq!(seen.len(), 512, "counter-backed nonces must not repeat");
    }

    #[test]
    fn create_run_temp_dir_is_unique_and_real() {
        let base = std::env::temp_dir();
        let (first, first_nonce) = create_run_temp_dir(&base).expect("first run dir");
        let (second, second_nonce) = create_run_temp_dir(&base).expect("second run dir");
        assert_ne!(
            first.path(),
            second.path(),
            "concurrent runs must never share a directory"
        );
        assert_ne!(first_nonce, second_nonce);
        assert!(first.path().is_dir() && second.path().is_dir());
        assert_eq!(first.path().parent(), Some(base.as_path()));
        assert_eq!(second.path().parent(), Some(base.as_path()));
        for dir in [&first, &second] {
            let name = dir
                .path()
                .file_name()
                .expect("run dir has a file name")
                .to_string_lossy();
            assert!(
                name.starts_with("dx-run-"),
                "run dir {name:?} must carry the dx-run- prefix"
            );
        }
        // `TempDir` auto-cleans on drop; explicit close asserts removal works.
        first.close().expect("cleanup first");
        second.close().expect("cleanup second");
    }
}

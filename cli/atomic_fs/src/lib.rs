//! Race-free atomic filesystem writes shared by every apply/commit path.
//!
//! Extracted for #74: `dx_apply::RealFileSystem::write_atomic`
//! (sibling+rename) duplicated the staging discipline that
//! `quality_adapter::exec::Scratch`, `dx_process::Fs`, and
//! `dx_env::acquire_lock` each reimplement around `tempfile`.
//! This crate owns the single write path plus the single lock-contention
//! loop (`try_lock` + timeout); `dx_env::acquire_lock` owns the lock-file
//! open, `dx_setup` reuses that route, so no new lock file, mechanism, or
//! deadline appears elsewhere.
//!
//! Contract: stage in the target directory so the final persist stays an
//! atomic same-filesystem rename; OS-random `O_EXCL`-claimed staging names
//! so concurrent writers never collide; `NamedTempFile` drop-cleanup so a
//! crash leaves no stale sibling; preserve the existing file mode on
//! overwrite (new files get `0644` on unix to match `std::fs::write`
//! defaults, since `NamedTempFile` creates `0600`).
//!
//! Newline contract (issue #84): bytes are preserved byte-for-byte with no
//! normalization. LF, CRLF, and missing-final-newline variants stay
//! distinct and round-trip exactly; the runner proves this with
//! `newline_variants_yield_distinct_manifests` and whole-file splice
//! checks, and every apply path writes the candidate bytes verbatim.
//!
//! Lock contract: contention-only retry on `WouldBlock` until the deadline;
//! any other flock failure aborts immediately so platform errors are never
//! misreported as busy. Locks release when the holding `File` drops
//! (fd close).
//!
//! Dependency evaluation (issue #229, rejected; qualified under issue #315:
//! stays hand-rolled): no `fs2`/`fslock` — the
//! stable `std::fs::File::try_lock` API is the upstreamed equivalent and
//! already owns the flock here, so the crates would add supply-chain
//! review, lockfile churn, and `MODULE.bazel` manifests for zero behavior
//! gain. No `fs-err` either: every `write_atomic` call site maps failures
//! into typed errors carrying the target path (e.g. `cannot publish
//! <path>`, `WriteFile { path, .. }`, `LockFailed { path, .. }`), so
//! runtime path-wrapping would duplicate the existing discipline at the
//! same dependency cost. This crate owns only the timeout loop.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::fs::File;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Atomically replaces `path` with `content`, creating parent directories
/// as needed.
///
/// Stages via an OS-random `O_EXCL`-claimed `NamedTempFile` in the target
/// directory, then persists with an atomic same-filesystem rename.
/// Bare file names (no parent) stage in the current directory for the
/// same reason.
pub fn write_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write as _;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    // Stage in the target directory so the final persist stays an atomic
    // same-filesystem rename. Bare file names (no parent) stage in the
    // current directory for the same reason.
    let staging_dir: &Path = parent.unwrap_or(Path::new("."));
    let mut staging = tempfile::NamedTempFile::new_in(staging_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        // Preserve the existing file mode on overwrite so apply never
        // strips executable bits or widens private modes (issue #84
        // "Preserve file modes"). New files get `0666 & !umask`
        // (typically 0644) to match `std::fs::write`; `NamedTempFile`
        // creates 0600, so restore the conventional non-executable
        // file mode before persisting.
        let mode = std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o777)
            .unwrap_or(0o644);
        staging
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(mode))?;
    }
    staging.write_all(content)?;
    staging.persist(path).map_err(|err| err.error)?;
    Ok(())
}

/// Poll interval while contending for an exclusive file lock.
pub const LOCK_POLL: Duration = Duration::from_millis(50);

/// Contends for an exclusive lock on the already-opened `file` until
/// `timeout`.
///
/// Returns `Ok(())` once this `File` holds the lock. Returns
/// `Err(TryLockError::WouldBlock)` when another holder keeps the lock past
/// the deadline (busy); any other `TryLockError` aborts immediately so
/// platform errors are never misreported as busy. The caller keeps the
/// returned `File` alive: dropping it releases the lock.
pub fn lock_exclusive(file: &File, timeout: Duration) -> Result<(), std::fs::TryLockError> {
    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                if start.elapsed() >= timeout {
                    return Err(std::fs::TryLockError::WouldBlock);
                }
                std::thread::sleep(LOCK_POLL);
            }
            Err(other) => return Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn writes_and_overwrites_without_stray_staging_files() {
        let scratch = dx_test_scratch::scratch("dx-atomic-fs-");
        let dir = scratch.path().to_path_buf();
        let nested = dir.join("sub").join("a.txt");
        assert_eq!(
            std::fs::read(&nested).map_err(|e| e.kind()),
            Err(io::ErrorKind::NotFound)
        );
        write_atomic(&nested, b"hello\n").expect("write");
        assert_eq!(std::fs::read(&nested).expect("read back"), b"hello\n");
        // No stray staging file remains beside the target after success.
        let entries: Vec<_> = std::fs::read_dir(dir.join("sub"))
            .expect("list target dir")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(entries, vec!["a.txt".to_owned()]);
        // Overwrites replace the target atomically through the same path.
        write_atomic(&nested, b"updated\n").expect("overwrite");
        assert_eq!(
            std::fs::read(&nested).expect("read overwrite"),
            b"updated\n"
        );
        let entries: Vec<_> = std::fs::read_dir(dir.join("sub"))
            .expect("list target dir after overwrite")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(entries, vec!["a.txt".to_owned()]);
        scratch.close().expect("cleanup");
    }

    #[test]
    fn creates_parent_directories() {
        let scratch = dx_test_scratch::scratch("dx-atomic-fs-parents-");
        let nested = scratch.path().join("a").join("b").join("c.txt");
        write_atomic(&nested, b"deep\n").expect("deep write");
        assert_eq!(std::fs::read(&nested).expect("deep read"), b"deep\n");
        scratch.close().expect("cleanup");
    }

    #[test]
    fn bare_name_stages_in_current_directory() {
        // A bare file name has no parent directory to create; staging
        // falls back to the current directory. Run in a scratch cwd so
        // the checkout is never left dirty.
        let scratch = dx_test_scratch::scratch("dx-atomic-fs-bare-");
        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(scratch.path()).expect("enter scratch");
        let bare = PathBuf::from("dx-atomic-fs-bare-tmp.txt");
        write_atomic(&bare, b"bare\n").expect("bare write");
        assert_eq!(std::fs::read(&bare).expect("bare read"), b"bare\n");
        std::fs::remove_file(&bare).expect("bare cleanup");
        std::env::set_current_dir(original).expect("leave scratch");
        scratch.close().expect("cleanup");
    }

    #[test]
    fn preserves_existing_mode_on_overwrite() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // file modes preserved. Overwriting an executable must keep the
        // executable bit; overwriting a private mode must keep it
        // private; bytes still round-trip exactly (no newline
        // normalization).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let scratch = dx_test_scratch::scratch("dx-atomic-fs-mode-");
            let executable = scratch.path().join("run.sh");
            write_atomic(&executable, b"#!/bin/sh\necho hi\n").expect("create executable");
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))
                .expect("chmod 755");
            write_atomic(&executable, b"#!/bin/sh\necho updated\n").expect("overwrite executable");
            assert_eq!(
                std::fs::read(&executable).expect("read back"),
                b"#!/bin/sh\necho updated\n"
            );
            assert_eq!(
                std::fs::metadata(&executable)
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
            let private = scratch.path().join("secret.txt");
            write_atomic(&private, b"secret\n").expect("create private");
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600))
                .expect("chmod 600");
            write_atomic(&private, b"rotated\n").expect("overwrite private");
            assert_eq!(std::fs::read(&private).expect("read back"), b"rotated\n");
            assert_eq!(
                std::fs::metadata(&private)
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            // CRLF bytes round-trip exactly through the same path.
            let crlf = scratch.path().join("crlf.txt");
            write_atomic(&crlf, b"line\r\n").expect("create crlf");
            write_atomic(&crlf, b"updated\r\n").expect("overwrite crlf");
            assert_eq!(std::fs::read(&crlf).expect("read back"), b"updated\r\n");
            scratch.close().expect("cleanup");
        }
    }

    fn open_lock_file(path: &Path) -> File {
        use std::fs::OpenOptions;
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .expect("open lock file")
    }

    #[test]
    fn lock_exclusive_acquires_uncontended() {
        let scratch = dx_test_scratch::scratch("dx-atomic-fs-lock-");
        let path = scratch.path().join(".commit.lock");
        let file = open_lock_file(&path);
        lock_exclusive(&file, Duration::from_secs(10)).expect("acquire");
        // Dropping the holder releases the lock; a fresh open acquires.
        drop(file);
        let next = open_lock_file(&path);
        lock_exclusive(&next, Duration::from_secs(10)).expect("reacquire after release");
        drop(next);
        scratch.close().expect("cleanup");
    }

    #[test]
    fn lock_exclusive_times_out_while_held() {
        let scratch = dx_test_scratch::scratch("dx-atomic-fs-lock-busy-");
        let path = scratch.path().join(".commit.lock");
        let held = open_lock_file(&path);
        lock_exclusive(&held, Duration::from_secs(10)).expect("hold lock");
        // A second open description contends with the first: zero timeout
        // reports busy instead of blocking.
        let waiter = open_lock_file(&path);
        assert!(matches!(
            lock_exclusive(&waiter, Duration::ZERO),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        drop(held);
        drop(waiter);
        scratch.close().expect("cleanup");
    }
}

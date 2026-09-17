//! Race-free atomic filesystem writes shared by every apply/commit path.
//!
//! Extracted for #74: `dx_apply::RealFileSystem::write_atomic`
//! (sibling+rename) duplicated the staging discipline that
//! `quality_adapter::exec::Scratch`, `dx_process::Fs`, and
//! `dx_env::acquire_lock` each reimplement around `tempfile`.
//! This crate owns the single write path; lock (`try_lock` + timeout)
//! and scratch/spawn unification arrive as follow-ups.
//!
//! Contract: stage in the target directory so the final persist stays an
//! atomic same-filesystem rename; OS-random `O_EXCL`-claimed staging names
//! so concurrent writers never collide; `NamedTempFile` drop-cleanup so a
//! crash leaves no stale sibling; `0644` on unix to match `std::fs::write`
//! defaults (`NamedTempFile` creates `0600`).

use std::io;
use std::path::Path;

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
        // `std::fs::write` creates `0666 & !umask` (typically 0644);
        // `NamedTempFile` creates 0600, so restore the conventional
        // non-executable file mode before persisting.
        staging
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o644))?;
    }
    staging.write_all(content)?;
    staging.persist(path).map_err(|err| err.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn writes_and_overwrites_without_stray_staging_files() {
        let scratch = tempfile::Builder::new()
            .prefix("dx-atomic-fs-")
            .tempdir_in(std::env::temp_dir())
            .expect("scratch");
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
        let scratch = tempfile::Builder::new()
            .prefix("dx-atomic-fs-parents-")
            .tempdir_in(std::env::temp_dir())
            .expect("scratch");
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
        let scratch = tempfile::Builder::new()
            .prefix("dx-atomic-fs-bare-")
            .tempdir_in(std::env::temp_dir())
            .expect("scratch");
        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(scratch.path()).expect("enter scratch");
        let bare = PathBuf::from("dx-atomic-fs-bare-tmp.txt");
        write_atomic(&bare, b"bare\n").expect("bare write");
        assert_eq!(std::fs::read(&bare).expect("bare read"), b"bare\n");
        std::fs::remove_file(&bare).expect("bare cleanup");
        std::env::set_current_dir(original).expect("leave scratch");
        scratch.close().expect("cleanup");
    }
}

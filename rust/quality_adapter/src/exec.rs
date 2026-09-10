//! Hermetic scratch mirrors and child execution.
//!
//! Tools never observe the real workspace: the adapter materializes the
//! exact input bytes plus symlinked native-closure files into a fresh
//! scratch directory, then spawns the tool with an empty-derived
//! environment (no `PATH`, no inherited config variables) and a pinned
//! working directory. `Scratch` removes its tree on drop.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Contents of one mirror entry, at a scratch-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorContents {
    /// Exact bytes to write (virtual source or generated defaults).
    Bytes(Vec<u8>),
    /// Absolute path to symlink (checked-in native-closure files).
    Link(PathBuf),
}

/// One scratch-relative mirror entry. Parent directories are created on
/// materialization; `mirror_rel` must stay inside the scratch root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorFile {
    pub mirror_rel: PathBuf,
    pub contents: MirrorContents,
}

/// A fresh scratch tree, removed on drop.
#[derive(Debug)]
pub struct Scratch {
    root: PathBuf,
}

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Scratch {
    /// Creates `parent/dx-scratch-<pid>-<counter>`. Parent is normally
    /// `TMPDIR`, already action-scoped under Bazel.
    pub fn create(parent: &Path) -> io::Result<Scratch> {
        for _ in 0..100 {
            let root = parent.join(format!(
                "dx-scratch-{}-{}",
                std::process::id(),
                SCRATCH_COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
            match std::fs::create_dir(&root) {
                Ok(()) => return Ok(Scratch { root }),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not claim a scratch directory",
        ))
    }

    /// Absolute scratch root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves a scratch-relative path, rejecting escapes.
    pub fn resolve(&self, rel: &Path) -> io::Result<PathBuf> {
        let mut absolute = self.root.clone();
        for component in rel.components() {
            use std::path::Component::{CurDir, Normal, ParentDir, Prefix, RootDir};
            match component {
                Normal(part) => absolute.push(part),
                CurDir => {}
                ParentDir => {
                    // `absolute` starts at the scratch root and every pop
                    // is range-checked below, so it always has depth to
                    // pop here; a failed pop would leave `absolute`
                    // outside the root and fail the check anyway.
                    absolute.pop();
                    if absolute != self.root && !absolute.starts_with(&self.root) {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("mirror path escapes scratch: {}", rel.display()),
                        ));
                    }
                }
                RootDir | Prefix(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("mirror path is absolute: {}", rel.display()),
                    ));
                }
            }
        }
        // Invariant: every component either descends (Normal), is neutral
        // (CurDir), was range-checked (ParentDir), or was rejected
        // (RootDir/Prefix), so `absolute` is still inside the root here.
        Ok(absolute)
    }

    /// Writes byte entries and links closure entries, creating parents.
    pub fn materialize(&self, files: &[MirrorFile]) -> io::Result<()> {
        for file in files {
            let absolute = self.resolve(&file.mirror_rel)?;
            // `resolve` only returns paths inside the scratch root, which
            // always has a parent, so this never fails on real filesystems.
            let parent = absolute
                .parent()
                .expect("scratch paths always have a parent");
            std::fs::create_dir_all(parent)?;
            match &file.contents {
                MirrorContents::Bytes(bytes) => std::fs::write(&absolute, bytes)?,
                MirrorContents::Link(target) => std::os::unix::fs::symlink(target, &absolute)?,
            }
        }
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Captured child outcome. `code` is [`None`] when a signal killed the
/// child; callers treat that as an action failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Builds the hermetic child environment: exactly `TMPDIR` plus caller
/// extras (such as `LD_LIBRARY_PATH` for toolchain binaries). `PATH` is
/// never set; every argv element is absolute, so tools cannot observe or
/// depend on ambient lookup.
pub fn hermetic_env(tmpdir: &Path, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut env = Vec::with_capacity(1 + extra.len());
    env.push(("TMPDIR".to_owned(), tmpdir.to_string_lossy().into_owned()));
    for (key, value) in extra {
        env.push((key.to_string(), value.to_string()));
    }
    env
}

/// Spawns one absolute tool binary with a cleared environment.
pub fn spawn(
    argv: &[impl AsRef<OsStr>],
    cwd: &Path,
    env: &[(String, String)],
) -> io::Result<ChildOutput> {
    let (binary, args) = argv
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invocation needs a binary"))?;
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .output()?;
    Ok(ChildOutput {
        code: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_materializes_and_cleans_up() {
        let parent = std::env::temp_dir();
        let scratch = Scratch::create(&parent).expect("scratch");
        let root = scratch.root().to_owned();
        assert!(root.is_dir());
        scratch
            .materialize(&[
                MirrorFile {
                    mirror_rel: PathBuf::from("src/main.rs"),
                    contents: MirrorContents::Bytes(b"fn main() {}\n".to_vec()),
                },
                MirrorFile {
                    mirror_rel: PathBuf::from("taplo.toml"),
                    contents: MirrorContents::Link(PathBuf::from("/checked/in/taplo.toml")),
                },
            ])
            .expect("materialize");
        assert_eq!(
            std::fs::read(root.join("src/main.rs")).expect("read back"),
            b"fn main() {}\n"
        );
        assert_eq!(
            std::fs::read_link(root.join("taplo.toml")).expect("link"),
            PathBuf::from("/checked/in/taplo.toml")
        );
        drop(scratch);
        assert!(!root.exists(), "scratch is removed on drop");
    }

    #[test]
    fn resolve_rejects_escapes_and_absolute_paths() {
        let scratch = Scratch::create(&std::env::temp_dir()).expect("scratch");
        assert!(scratch.resolve(Path::new("../../evil")).is_err());
        assert!(scratch.resolve(Path::new("/absolute")).is_err());
        // Enough `..` components to pop past the filesystem root fails
        // the pop itself, not just the containment check.
        assert!(scratch
            .resolve(Path::new("../../../../../../../../evil"))
            .is_err());
        assert_eq!(
            scratch
                .resolve(Path::new("sub/../ok.rs"))
                .expect("contained dot-dot"),
            scratch.root().join("ok.rs")
        );
        assert_eq!(
            scratch
                .resolve(Path::new("./ok.rs"))
                .expect("dot component"),
            scratch.root().join("ok.rs")
        );
    }

    #[test]
    fn create_reports_unusable_parents() {
        let missing = std::env::temp_dir().join("dx-no-such-parent-9f2c1d");
        let _ = std::fs::remove_dir_all(&missing);
        assert!(Scratch::create(&missing.join("child")).is_err());
    }

    #[test]
    fn create_skips_collisions_and_gives_up_when_full() {
        // A dedicated parent isolates the pre-created collisions from
        // the other tests sharing the system temporary directory. The
        // global claim counter only advances a handful of steps per
        // test binary, so counters 0..512 always cover its candidates.
        let parent = std::env::temp_dir().join(format!("dx-collision-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        std::fs::create_dir_all(&parent).expect("collision parent");
        for counter in 0..512 {
            // The final assertions carry the check: if any pre-create
            // failed, creation below would succeed and fail the test.
            let claimed = parent.join(format!("dx-scratch-{}-{counter}", std::process::id()));
            let _ = std::fs::create_dir(&claimed);
        }
        // Every candidate collides, so creation exhausts its retries.
        assert!(Scratch::create(&parent).is_err());
        std::fs::remove_dir_all(&parent).expect("collision cleanup");
        // With the collisions gone the same parent works again.
        std::fs::create_dir_all(&parent).expect("collision parent again");
        let scratch = Scratch::create(&parent).expect("scratch after cleanup");
        assert!(scratch.root().is_dir());
    }

    #[test]
    fn hermetic_env_has_no_path() {
        let env = hermetic_env(Path::new("/tmp/dx"), &[("LD_LIBRARY_PATH", "/lib")]);
        assert_eq!(
            env,
            vec![
                ("TMPDIR".to_owned(), "/tmp/dx".to_owned()),
                ("LD_LIBRARY_PATH".to_owned(), "/lib".to_owned()),
            ]
        );
    }

    #[test]
    fn spawn_runs_absolute_binaries_without_path() {
        let env = hermetic_env(&std::env::temp_dir(), &[]);
        let ok = spawn(&[OsStr::new("/bin/true")], Path::new("/"), &env).expect("spawn");
        assert_eq!(ok.code, Some(0));
        let fail = spawn(&[OsStr::new("/bin/false")], Path::new("/"), &env).expect("spawn");
        assert_eq!(fail.code, Some(1));
        assert!(spawn(&[OsStr::new("/nonexistent-dx-tool")], Path::new("/"), &env).is_err());
        let empty: Vec<&OsStr> = Vec::new();
        assert!(spawn(&empty, Path::new("/"), &env).is_err());
    }
}

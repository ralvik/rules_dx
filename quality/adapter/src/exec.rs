//! Hermetic scratch mirrors and child execution.
//!
//! Tools never observe the real workspace: the adapter materializes the
//! exact input bytes plus native-closure files into a fresh scratch
//! directory, then spawns the tool with an empty-derived environment
//! (no `PATH`, no inherited config variables, pinned `LANG`/`TZ`)
//! and a pinned working directory. Native-closure entries follow the portable
//! route: symlinked where the platform allows and copied otherwise, so
//! Windows works without privileges. The copy fallback is intentional
//! (tools only read these entries); `materialize_copies_closure_entry_
//! when_link_path_exists` plus the non-unix no-symlink assertion in
//! `scratch_materializes_and_cleans_up` prove both branches.
//! `Scratch` removes its tree on drop as a best-effort fallback;
//! owners call [`Scratch::close`] on success paths so cleanup failures
//! surface as action errors instead of vanishing.
//!
//! Dependency evaluation (rejected): no `strict-path` — the
//! scratch threat model stays TempDir-internal (fresh OS-random `TempDir`
//! plus internal `mirror_rel` from Bazel action inputs, never untrusted
//! archives/HTTP/LLM paths), so the lexical `Component` walk plus
//! `starts_with` boundary plus the explicit empty/null/backslash guards
//! and the on-disk symlink-prefix guard below own the 19+ CVE-pattern
//! class here. Adopting `strict-path 0.2` (`PathBoundary`/`StrictPath`
//! over `soft-canonicalize` plus `dunce`, single maintainer, on-disk
//! resolve per join, `interop_path`/`StrictPath` API churn) would add
//! supply-chain review, lockfile churn, and `MODULE.bazel` manifests for
//! zero behavior gain today; `soft-canonicalize` alone carries no
//! boundary policy. `normpath` is adopted for lexical accumulation only
//! (`BasePathBuf` owns Windows `Prefix`/verbatim edge semantics
//! instead of the former hand `PathBuf`); on-disk symlink escapes stay owned
//! by the symlink-prefix guard below, not by normalization. Re-evaluate with
//! `VirtualRoot`-style boundary plus safe-I/O only if adapters ever accept
//! untrusted entries.

use normpath::BasePathBuf;
use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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
///
/// Single prod scratch policy for `#651`: this is the hermetic mirror
/// (`resolve`/`materialize`/`close` with symlink-prefix guards).
/// `quality_runner` reuses this type (no second `Scratch`); unit-test
/// scratch dirs use `dx_test_scratch::scratch`, and `dx` per-run temp dirs
/// use `dx_cli::plan::create_run_temp_dir`.
/// Scratch discipline (See: `docs/testing/README.md`, issue #750): test
/// parents below stay on `tempfile::Builder` directly because they are the
/// explicit parents under test for `Scratch::create`, not second policies.
///
/// The tree is a `tempfile::TempDir`: OS-random `O_EXCL`-claimed names
/// with internal collision retries replace the former
/// wall-clock/pid/counter suffix mixer, and `TempDir`'s own drop is the
/// best-effort removal fallback. Owners still call [`Scratch::close`] on
/// success paths so cleanup failures surface as action errors.
#[derive(Debug)]
pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    /// Creates `parent/dx-scratch-<os-random>`. Parent is normally
    /// `TMPDIR`, already action-scoped under Bazel. Fails when the
    /// parent is unusable; name collisions retry inside `tempfile`
    /// instead of a caller-visible suffix loop.
    pub fn create(parent: &Path) -> io::Result<Scratch> {
        let dir = tempfile::Builder::new()
            .prefix("dx-scratch-")
            .tempdir_in(parent)?;
        Ok(Scratch { dir })
    }

    /// Absolute scratch root.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// Resolves a scratch-relative path, rejecting escapes.
    ///
    /// Lexical boundary (`normpath::BasePathBuf` accumulation
    /// owns Windows `Prefix`/verbatim edge semantics instead of hand
    /// `PathBuf`) plus explicit shape guards (empty, null byte, backslash
    /// for portable Unix/Windows behavior) plus a final containment check.
    /// On-disk symlink escapes are owned by [`Scratch::materialize`]'s
    /// symlink-prefix guard, not by this lexical join: this returns the
    /// lexical location, materialize refuses to traverse a symlink to
    /// reach it.
    pub fn resolve(&self, rel: &Path) -> io::Result<PathBuf> {
        let root = self.dir.path();
        // Encoded bytes keep the shape check exact on non-UTF8 inputs:
        // null and backslash are ASCII, so byte and lossy views agree.
        let raw = rel.as_os_str().as_encoded_bytes();
        if raw.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "mirror path is empty",
            ));
        }
        if raw.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("mirror path contains null byte: {}", rel.display()),
            ));
        }
        if raw.contains(&b'\\') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("mirror path contains backslash: {}", rel.display()),
            ));
        }
        let mut absolute = BasePathBuf::new(root.to_owned()).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("scratch root is not a base path: {error}"),
            )
        })?;
        for component in rel.components() {
            use std::path::Component::{CurDir, Normal, ParentDir, Prefix, RootDir};
            match component {
                Normal(part) => absolute.push(part),
                CurDir => {}
                ParentDir => {
                    // `absolute` starts at the scratch root and every pop
                    // is range-checked below, so it always has depth to
                    // pop here; a failed pop would leave `absolute`
                    // outside the root and fail the check anyway. `pop`
                    // errors only on a future walk divergence (non-`Normal`/
                    // non-`RootDir` tail), which fails closed as an escape.
                    if absolute.pop().is_err() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("mirror path escapes scratch: {}", rel.display()),
                        ));
                    }
                    if absolute.as_path() != root && !absolute.starts_with(root) {
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
        // Defense in depth beyond the per-pop checks: every component
        // either descended (Normal), was neutral (CurDir), was
        // range-checked (ParentDir), or was rejected (RootDir/Prefix),
        // so this only fires on a future walk divergence.
        if absolute.as_path() != root && !absolute.starts_with(root) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("mirror path escapes scratch: {}", rel.display()),
            ));
        }
        Ok(absolute.into_path_buf())
    }

    /// Writes byte entries and links closure entries, creating parents.
    /// Closure entries prefer symlinks but fall back to copies, so
    /// platforms without symlinks (or without the privilege to create
    /// them) still materialize working trees.
    ///
    /// Boundary enforcement is lexical (`resolve`) plus on-disk: the
    /// symlink-prefix guard refuses to traverse a symlink directory
    /// created by an earlier entry, and an existing symlink at the
    /// target itself is rejected instead of followed. Fresh scratch
    /// trees start symlink-free, so the guard only fires on escape
    /// attempts or future `Link`-to-directory misuse.
    pub fn materialize(&self, files: &[MirrorFile]) -> io::Result<()> {
        let root = self.dir.path();
        for file in files {
            let absolute = self.resolve(&file.mirror_rel)?;
            // `resolve` only returns paths inside the scratch root, which
            // always has a parent, so this only fires on a future
            // resolve/materialize divergence (fail closed, never panic).
            let parent = absolute.parent().ok_or_else(|| {
                // LCOV_EXCL_LINE - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("scratch path has no parent: {}", absolute.display()),
                )
            })?;
            // Lexical resolve cannot see symlinks: refuse to traverse a
            // symlink prefix before creating parents.
            ensure_no_symlink_prefix(root, parent, &file.mirror_rel)?;
            // Refuse to follow an existing symlink at the target: a byte
            // write or a fallback copy through it would land outside.
            if let Ok(meta) = std::fs::symlink_metadata(&absolute) {
                if meta.file_type().is_symlink() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "mirror path escapes scratch via symlink: {}",
                            file.mirror_rel.display()
                        ),
                    ));
                }
            }
            std::fs::create_dir_all(parent)?;
            // `create_dir_all` follows symlinks, so re-check after
            // creation (no concurrent actor today, but fail closed).
            ensure_no_symlink_prefix(root, parent, &file.mirror_rel)?;
            match &file.contents {
                MirrorContents::Bytes(bytes) => std::fs::write(&absolute, bytes)?,
                MirrorContents::Link(target) => link_or_copy(target, &absolute)?,
            }
        }
        Ok(())
    }

    /// Removes the tree, surfacing cleanup failures to the caller.
    /// Owners call this on success paths; early-error paths rely on the
    /// best-effort `TempDir` drop fallback (which cannot return errors).
    /// Consuming `self` skips that fallback: the removal below already
    /// ran, and a second attempt could only mask this result.
    pub fn close(self) -> io::Result<()> {
        self.dir.close()
    }
}

/// Rejects on-disk symlink prefixes between `root` (exclusive) and
/// `path` (inclusive): any existing prefix that is a symlink would make
/// a lexically inside `absolute` land outside on disk.
///
/// Missing prefixes cannot be symlinks yet; they become real directories
/// via `create_dir_all` after this guard. `root` itself is the fresh
/// `TempDir` and is never a symlink, so only its children are checked.
fn ensure_no_symlink_prefix(root: &Path, path: &Path, rel: &Path) -> io::Result<()> {
    let suffix = path.strip_prefix(root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("mirror path escapes scratch: {}", rel.display()),
        )
    })?;
    let mut current = root.to_owned();
    for component in suffix.components() {
        use std::path::Component::{CurDir, Normal, ParentDir, Prefix, RootDir};
        match component {
            Normal(part) => current.push(part),
            CurDir => {}
            ParentDir => {
                current.pop();
            }
            RootDir | Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("mirror path escapes scratch: {}", rel.display()),
                ));
            }
        }
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("mirror path escapes scratch via symlink: {}", rel.display()),
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// Links `target` at `link`, copying the file when symlinks are
/// unavailable (non-Unix platforms, or missing privileges): tools only
/// ever read these native-closure entries. portable route:
/// the copy fallback is intentional, not a silent privilege-gap hide;
/// directory targets fail fast via the `copy` error instead of a
/// half-materialized tree. Non-unix always copies; unix prefers a
/// symlink and copies only when linking fails.
#[cfg(unix)]
fn link_or_copy(target: &Path, link: &Path) -> io::Result<()> {
    match std::os::unix::fs::symlink(target, link) {
        Ok(()) => Ok(()),
        Err(_) => std::fs::copy(target, link).map(|_| ()),
    }
}

/// Links `target` at `link`, copying the file when symlinks are
/// unavailable (non-Unix platforms, or missing privileges): tools only
/// ever read these native-closure entries. portable route:
/// the copy fallback is intentional, not a silent privilege-gap hide;
/// directory targets fail fast via the `copy` error instead of a
/// half-materialized tree. Non-unix always copies; unix prefers a
/// symlink and copies only when linking fails.
#[cfg(not(unix))]
fn link_or_copy(target: &Path, link: &Path) -> io::Result<()> {
    std::fs::copy(target, link).map(|_| ())
}

/// Captured child outcome. `code` is [`None`] when a signal killed the
/// child; callers treat that as an action failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Builds the hermetic child environment: exactly one `TMPDIR` (the
/// scratch root) plus pinned `LANG=C.UTF-8` and `TZ=UTC` plus caller
/// extras (such as `LD_LIBRARY_PATH` for toolchain binaries). `PATH`
/// is never set; every argv element is absolute, so tools cannot
/// observe or depend on ambient lookup. The locale/time pin keeps
/// locale/time-sensitive tools (prettier, buf, clang-format, vale)
/// deterministic across runners (See: `docs/testing/tools.md`).
/// Extras naming `TMPDIR`, `LANG`, or `TZ` are dropped: the scratch
/// root owns temp files and the pinned locale/timezone own
/// determinism, never shadowable by tool configuration.
pub fn hermetic_env(tmpdir: &Path, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut env = Vec::with_capacity(3 + extra.len());
    env.push(("TMPDIR".to_owned(), tmpdir.to_string_lossy().into_owned()));
    env.push(("LANG".to_owned(), "C.UTF-8".to_owned()));
    env.push(("TZ".to_owned(), "UTC".to_owned()));
    for (key, value) in extra {
        if *key != "TMPDIR" && *key != "LANG" && *key != "TZ" {
            env.push((key.to_string(), value.to_string()));
        }
    }
    env
}

/// Maximum tool output wall-time plus max output size guard.
/// Tools that exceed either fail closed as action errors, never silent.
/// See: `docs/quality/tool-integrations.md#initial-adapter-qualification`
pub const SPAWN_TIMEOUT: Duration = Duration::from_secs(120);

/// Rejects oversized child output (max output size guard, output-byte
/// budget for check plus sandbox-apply-and-diff spawns).
fn check_child_output_size(stdout: &[u8], stderr: &[u8]) -> io::Result<()> {
    let limit = crate::parsers::MAX_OUTPUT_BYTES;
    if stdout.len() > limit || stderr.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tool output exceeds max size {limit} bytes"),
        ));
    }
    Ok(())
}

/// Drains one child pipe on a helper thread so a chatty child never
/// fills the pipe buffer and blocks on write (which would fake a
/// timeout). Stores at most `MAX_OUTPUT_BYTES + 1` so the size guard
/// still fails closed without unbounded memory; keeps reading past that
/// without storing so the child can finish or be reaped on kill.
fn drain_pipe<R: Read + Send + 'static>(mut pipe: R) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let limit = crate::parsers::MAX_OUTPUT_BYTES;
        let mut buf = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let room = limit.saturating_add(1).saturating_sub(buf.len());
                    let take = n.min(room);
                    buf.extend_from_slice(&chunk[..take]);
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        buf
    })
}

/// Joins one drain thread, mapping a reader panic to an action error.
fn join_drain(handle: Option<std::thread::JoinHandle<Vec<u8>>>) -> io::Result<Vec<u8>> {
    match handle {
        None => Ok(Vec::new()),
        Some(handle) => handle
            .join()
            .map_err(|_| io::Error::other("tool output reader thread panicked")),
    }
}

/// Spawns one absolute tool binary with a cleared environment.
pub fn spawn(
    argv: &[impl AsRef<OsStr>],
    cwd: &Path,
    env: &[(String, String)],
) -> io::Result<ChildOutput> {
    spawn_with_timeout(argv, cwd, env, SPAWN_TIMEOUT)
}

/// Spawns with an explicit timeout (fuzz/property harness uses short
/// timeouts; production uses [`SPAWN_TIMEOUT`]).
pub fn spawn_with_timeout(
    argv: &[impl AsRef<OsStr>],
    cwd: &Path,
    env: &[(String, String)],
    timeout: Duration,
) -> io::Result<ChildOutput> {
    let (binary, args) = argv
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invocation needs a binary"))?;
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout_reader = child.stdout.take().map(drain_pipe);
    let mut stderr_reader = child.stderr.take().map(drain_pipe);
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if Instant::now() >= deadline {
                    // Fail closed on timeout: kill then reap so no zombie
                    // remains; join drains (pipes close on kill) so no
                    // reader outlives the call; the caller surfaces
                    // TimedOut as an action failure.
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = join_drain(stdout_reader.take());
                    let _ = join_drain(stderr_reader.take());
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("tool timed out after {}s", timeout.as_secs()),
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    };
    let stdout = join_drain(stdout_reader)?;
    let stderr = join_drain(stderr_reader)?;
    check_child_output_size(&stdout, &stderr)?;
    Ok(ChildOutput {
        code: status.code(),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_materializes_and_cleans_up() {
        let parent_tmp = tempfile::Builder::new()
            .prefix("dx-materialize-")
            .tempdir_in(std::env::temp_dir())
            .expect("materialize parent");
        let parent = parent_tmp.path().to_path_buf();
        std::fs::create_dir_all(&parent).expect("materialize parent");
        let source = parent.join("native-taplo.toml");
        std::fs::write(&source, b"config = true\n").expect("closure source");
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
                    contents: MirrorContents::Link(source.clone()),
                },
            ])
            .expect("materialize");
        assert_eq!(
            std::fs::read(root.join("src/main.rs")).expect("read back"),
            b"fn main() {}\n"
        );
        // Content reads back identically whether the platform linked or
        // copied the closure entry.
        assert_eq!(
            std::fs::read(root.join("taplo.toml")).expect("closure entry"),
            b"config = true\n"
        );
        #[cfg(unix)]
        assert_eq!(
            std::fs::read_link(root.join("taplo.toml")).expect("symlink preferred on unix"),
            source,
        );
        // Portable route: non-unix always copies, so no
        // symlink must remain; content equality above already proves
        // the copy branch.
        #[cfg(not(unix))]
        assert!(
            !std::fs::symlink_metadata(root.join("taplo.toml"))
                .expect("metadata")
                .file_type()
                .is_symlink(),
            "non-unix closure entries copy instead of linking",
        );
        drop(scratch);
        assert!(!root.exists(), "scratch is removed on drop");
        parent_tmp.close().expect("materialize cleanup");
    }

    #[test]
    fn materialize_copies_closure_entry_when_link_path_exists() {
        // A pre-existing file at the link path makes symlinking fail,
        // so the closure entry falls back to a copy: identical content,
        // no symlink left behind.
        let parent_tmp = tempfile::Builder::new()
            .prefix("dx-fallback-")
            .tempdir_in(std::env::temp_dir())
            .expect("fallback parent");
        let parent = parent_tmp.path().to_path_buf();
        std::fs::create_dir_all(&parent).expect("fallback parent");
        let source = parent.join("native.toml");
        std::fs::write(&source, b"config = true\n").expect("closure source");
        let scratch = Scratch::create(&parent).expect("scratch");
        let dest = scratch.root().join("taplo.toml");
        std::fs::write(&dest, b"stale\n").expect("pre-existing link path");
        scratch
            .materialize(&[MirrorFile {
                mirror_rel: PathBuf::from("taplo.toml"),
                contents: MirrorContents::Link(source),
            }])
            .expect("fallback copy");
        assert_eq!(
            std::fs::read(&dest).expect("copied entry"),
            b"config = true\n"
        );
        assert!(
            !std::fs::symlink_metadata(&dest)
                .expect("metadata")
                .file_type()
                .is_symlink(),
            "fallback copies instead of linking"
        );
        scratch.close().expect("close");
        parent_tmp.close().expect("fallback cleanup");
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
    fn resolve_normalizes_dot_segments_via_normpath() {
        // Fixtures: `a/b/../c`, `./`, trailing-slash,
        // excessive-`..` (escapes fail closed here, unlike markdown's
        // clamped sibling-root semantics).
        let scratch = Scratch::create(&std::env::temp_dir()).expect("scratch");
        assert_eq!(
            scratch.resolve(Path::new("a/b/../c.rs")).expect("a/b/../c"),
            scratch.root().join("a/c.rs")
        );
        assert_eq!(
            scratch.resolve(Path::new("./a.rs")).expect("dot slash"),
            scratch.root().join("a.rs")
        );
        assert_eq!(
            scratch.resolve(Path::new("a/b/")).expect("trailing slash"),
            scratch.root().join("a/b")
        );
        assert!(scratch.resolve(Path::new("a/../../evil")).is_err());
        assert!(scratch.resolve(Path::new("a/b/../../../evil")).is_err());
    }

    #[test]
    fn resolve_rejects_empty_null_and_backslash() {
        // Explicit shape guards for the TempDir-internal
        // lexical boundary — portable across Unix/Windows, fail closed
        // without on-disk canonicalization.
        let scratch = Scratch::create(&std::env::temp_dir()).expect("scratch");
        assert!(scratch.resolve(Path::new("")).is_err(), "empty");
        assert!(scratch.resolve(Path::new("a\0b")).is_err(), "null byte");
        assert!(scratch.resolve(Path::new("a\\b")).is_err(), "backslash");
        assert!(
            scratch.resolve(Path::new("..\\evil")).is_err(),
            "mixed separators"
        );
        assert!(
            scratch.resolve(Path::new("\\\\server\\share")).is_err(),
            "UNC"
        );
        // Forward-slash nesting stays inside.
        assert_eq!(
            scratch.resolve(Path::new("a/b/c.rs")).expect("nested"),
            scratch.root().join("a/b/c.rs")
        );
    }

    #[test]
    #[cfg(unix)]
    fn materialize_rejects_symlink_directory_escape() {
        // Traversal fixture, fail-fast policy:
        // the symlink-prefix guard only fires when the platform can
        // plant a symlink. Non-unix `Link` entries always copy, so no
        // symlink prefix can arise from `Link` there; the copy branch
        // is proven by `materialize_copies_closure_entry_when_link_path_
        // exists` plus the non-unix assertion in
        // `scratch_materializes_and_cleans_up`.
        // Traversal fixture: a symlink directory prefix from
        // an earlier entry must not let a later lexically inside path
        // land outside on disk.
        let parent_tmp = tempfile::Builder::new()
            .prefix("dx-symlink-dir-")
            .tempdir_in(std::env::temp_dir())
            .expect("symlink parent");
        let parent = parent_tmp.path().to_path_buf();
        let outside = parent.join("outside");
        std::fs::create_dir_all(&outside).expect("outside dir");
        let scratch = Scratch::create(&parent).expect("scratch");
        let root = scratch.root().to_owned();
        // Plant a directory symlink via the Link entry shape.
        scratch
            .materialize(&[MirrorFile {
                mirror_rel: PathBuf::from("evil"),
                contents: MirrorContents::Link(outside.clone()),
            }])
            .expect("plant symlink dir");
        assert_eq!(
            std::fs::read_link(root.join("evil")).expect("symlink planted"),
            outside,
        );
        // A later entry through that prefix fails closed.
        let err = scratch
            .materialize(&[MirrorFile {
                mirror_rel: PathBuf::from("evil/pwned"),
                contents: MirrorContents::Bytes(b"pwned\n".to_vec()),
            }])
            .expect_err("symlink prefix must fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!outside.join("pwned").exists(), "outside stays clean");
        scratch.close().expect("close");
        parent_tmp.close().expect("cleanup");
    }

    #[test]
    #[cfg(unix)]
    fn materialize_rejects_symlink_file_overwrite() {
        // Traversal fixture, fail-fast policy:
        // planting a file symlink needs symlink privilege, unavailable
        // on non-unix `Link` paths (always copy). See the directory
        // escape test above for the non-unix cover.
        // Traversal fixture: an existing symlink at the
        // target must not be followed by a byte write or fallback copy.
        let parent_tmp = tempfile::Builder::new()
            .prefix("dx-symlink-file-")
            .tempdir_in(std::env::temp_dir())
            .expect("symlink parent");
        let parent = parent_tmp.path().to_path_buf();
        let outside = parent.join("secret");
        std::fs::write(&outside, b"secret\n").expect("outside file");
        let scratch = Scratch::create(&parent).expect("scratch");
        let root = scratch.root().to_owned();
        std::os::unix::fs::symlink(&outside, root.join("link")).expect("plant file symlink");
        let err = scratch
            .materialize(&[MirrorFile {
                mirror_rel: PathBuf::from("link"),
                contents: MirrorContents::Bytes(b"pwned\n".to_vec()),
            }])
            .expect_err("symlink target must fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            std::fs::read(&outside).expect("outside read"),
            b"secret\n",
            "outside unchanged"
        );
        scratch.close().expect("close");
        parent_tmp.close().expect("cleanup");
    }

    #[test]
    fn create_reports_unusable_parents() {
        let missing = std::env::temp_dir().join("dx-no-such-parent-9f2c1d");
        let _ = std::fs::remove_dir_all(&missing);
        assert!(Scratch::create(&missing.join("child")).is_err());
    }

    #[test]
    fn scratch_claims_distinct_prefixed_trees() {
        // Collision retries now live inside `tempfile` (`O_EXCL`
        // claims with OS-random names), so no deterministic suffix
        // injection point remains: prove the observable contract
        // instead — concurrent claims never share a tree, every tree
        // lives under the parent with the recognizable prefix.
        let parent_tmp = tempfile::Builder::new()
            .prefix("dx-distinct-")
            .tempdir_in(std::env::temp_dir())
            .expect("distinct parent");
        let parent = parent_tmp.path().to_path_buf();
        std::fs::create_dir_all(&parent).expect("distinct parent");
        let first = Scratch::create(&parent).expect("first");
        let second = Scratch::create(&parent).expect("second");
        assert_ne!(first.root(), second.root(), "claims never share a tree");
        for root in [first.root(), second.root()] {
            assert!(root.starts_with(&parent), "scratch lives under the parent");
            assert!(
                root.file_name()
                    .expect("scratch name")
                    .to_string_lossy()
                    .starts_with("dx-scratch-"),
                "scratch keeps the recognizable prefix",
            );
        }
        first.close().expect("close");
        second.close().expect("close");
        parent_tmp.close().expect("distinct cleanup");
    }

    #[test]
    fn scratch_names_are_unique() {
        let parent = std::env::temp_dir();
        let mut roots = Vec::new();
        for _ in 0..64 {
            let scratch = Scratch::create(&parent).expect("scratch");
            roots.push(scratch.root().to_owned());
            scratch.close().expect("close");
        }
        roots.sort();
        roots.dedup();
        assert_eq!(roots.len(), 64, "every scratch claims a distinct tree");
    }

    #[test]
    fn close_removes_tree_and_surfaces_errors() {
        let scratch = Scratch::create(&std::env::temp_dir()).expect("scratch");
        let root = scratch.root().to_owned();
        scratch.close().expect("close removes the tree");
        assert!(!root.exists(), "closed scratch is gone");
        // A tree that vanished out from under the scratch surfaces its
        // cleanup failure instead of dropping it in `Drop`.
        let scratch = Scratch::create(&std::env::temp_dir()).expect("scratch");
        let root = scratch.root().to_owned();
        std::fs::remove_dir_all(&root).expect("pre-remove");
        assert!(scratch.close().is_err(), "missing tree is an error");
    }

    #[test]
    fn hermetic_env_has_no_path() {
        let env = hermetic_env(Path::new("/tmp/dx"), &[("LD_LIBRARY_PATH", "/lib")]);
        assert_eq!(
            env,
            vec![
                ("TMPDIR".to_owned(), "/tmp/dx".to_owned()),
                ("LANG".to_owned(), "C.UTF-8".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
                ("LD_LIBRARY_PATH".to_owned(), "/lib".to_owned()),
            ]
        );
    }

    #[test]
    fn hermetic_env_tmpdir_is_never_shadowed() {
        let env = hermetic_env(
            Path::new("/tmp/dx"),
            &[("TMPDIR", "/evil"), ("LD_LIBRARY_PATH", "/lib")],
        );
        assert_eq!(
            env.iter().filter(|(key, _)| key == "TMPDIR").count(),
            1,
            "exactly one TMPDIR"
        );
        assert_eq!(env[0], ("TMPDIR".to_owned(), "/tmp/dx".to_owned()));
        assert!(env.contains(&("LD_LIBRARY_PATH".to_owned(), "/lib".to_owned())));
    }

    #[test]
    fn hermetic_env_pins_locale_and_time() {
        let env = hermetic_env(
            Path::new("/tmp/dx"),
            &[
                ("LANG", "de_DE.UTF-8"),
                ("TZ", "Europe/Berlin"),
                ("LC_ALL", "de_DE.UTF-8"),
            ],
        );
        assert_eq!(
            env.iter().filter(|(key, _)| key == "LANG").count(),
            1,
            "exactly one LANG"
        );
        assert_eq!(
            env.iter().filter(|(key, _)| key == "TZ").count(),
            1,
            "exactly one TZ"
        );
        assert!(env.contains(&("LANG".to_owned(), "C.UTF-8".to_owned())));
        assert!(env.contains(&("TZ".to_owned(), "UTC".to_owned())));
        assert!(env.contains(&("LC_ALL".to_owned(), "de_DE.UTF-8".to_owned())));
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

    #[test]
    fn spawn_enforces_timeout_and_kills_the_child() {
        let env = hermetic_env(&std::env::temp_dir(), &[]);
        let err = spawn_with_timeout(
            &[
                OsStr::new("/bin/sh"),
                OsStr::new("-c"),
                OsStr::new("exec sleep 30"),
            ],
            Path::new("/"),
            &env,
            Duration::from_millis(50),
        )
        .expect_err("timeout");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn spawn_drains_large_output_without_faking_a_timeout() {
        // A child that writes past the pipe buffer (~64 KiB) must not
        // block on write; concurrent drain keeps it exiting under the
        // wall-time budget.
        let env = hermetic_env(&std::env::temp_dir(), &[]);
        let out = spawn_with_timeout(
            &[
                OsStr::new("/bin/sh"),
                OsStr::new("-c"),
                OsStr::new("head -c 1048576 /dev/zero"),
            ],
            Path::new("/"),
            &env,
            Duration::from_secs(10),
        )
        .expect("large output");
        assert_eq!(out.code, Some(0));
        assert_eq!(out.stdout.len(), 1_048_576);
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn spawn_rejects_oversized_output() {
        // Output past MAX_OUTPUT_BYTES fails closed even when the child
        // itself exits cleanly.
        let env = hermetic_env(&std::env::temp_dir(), &[]);
        let limit = crate::parsers::MAX_OUTPUT_BYTES;
        let bytes = limit + 1;
        let script = format!("head -c {bytes} /dev/zero");
        let err = spawn_with_timeout(
            &[OsStr::new("/bin/sh"), OsStr::new("-c"), OsStr::new(&script)],
            Path::new("/"),
            &env,
            Duration::from_secs(30),
        )
        .expect_err("oversize");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("max size"));
    }
}

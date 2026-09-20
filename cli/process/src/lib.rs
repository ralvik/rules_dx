//! WP1 process boundary for the `dx` CLI.
//!
//! Contract: `docs/cli/cli-contract.md`. This crate owns workspace
//! discovery, launcher selection, safe operation summaries, exact `dx
//! bazel` forwarding with protected-flag enforcement, exit-code mapping,
//! dry-run gating, Unix signal forwarding, and the process-runner seam.
//! It never renders subprocess argument vectors, option values,
//! environment values, or reconstructed shell commands.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Filesystem seam so unit tests use a fake instead of the real tree.
pub trait Fs {
    /// Reports whether `path` is a regular file.
    fn is_file(&self, path: &Path) -> bool;
    /// Reads `path` as UTF-8 text.
    fn read_text(&self, path: &Path) -> io::Result<String>;
}

/// Real filesystem implementation used by the CLI binary.
pub struct RealFs;

impl Fs for RealFs {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn read_text(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
}

/// Workspace discovery failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiscoverError {
    /// No `MODULE.bazel` found walking up from the start directory.
    #[error(
        "workspace_not_found: no MODULE.bazel in {searched_len} directorie(s); pass --workspace <path>",
        searched_len = searched.len()
    )]
    NotFound {
        /// Every ancestor directory considered, nearest first.
        searched: Vec<PathBuf>,
    },
    /// A legacy marker without `MODULE.bazel` was found instead.
    #[error(
        "unsupported_workspace: {dir} has WORKSPACE without MODULE.bazel; migrate to Bzlmod",
        dir = dir.display()
    )]
    UnsupportedLegacy {
        /// Directory holding the legacy marker.
        dir: PathBuf,
    },
    /// An explicit `--workspace` override holds no module marker.
    #[error("workspace_not_found: --workspace {path} has no MODULE.bazel", path = path.display())]
    InvalidOverride {
        /// The rejected override directory.
        path: PathBuf,
    },
}

fn has_module(fs: &dyn Fs, dir: &Path) -> bool {
    fs.is_file(&dir.join("MODULE.bazel"))
}

fn legacy_marker(fs: &dyn Fs, dir: &Path) -> bool {
    fs.is_file(&dir.join("WORKSPACE")) || fs.is_file(&dir.join("WORKSPACE.bazel"))
}

/// Discovers the Bazel workspace root.
///
/// With `override_dir`, only that directory is considered. Otherwise
/// walks ancestors from `start` looking for `MODULE.bazel`. A legacy
/// `WORKSPACE` marker without `MODULE.bazel` is an actionable error,
/// never a second setup path.
pub fn discover(
    start: &Path,
    override_dir: Option<&Path>,
    fs: &dyn Fs,
) -> Result<PathBuf, DiscoverError> {
    if let Some(dir) = override_dir {
        if has_module(fs, dir) {
            return Ok(dir.to_path_buf());
        }
        if legacy_marker(fs, dir) {
            return Err(DiscoverError::UnsupportedLegacy {
                dir: dir.to_path_buf(),
            });
        }
        return Err(DiscoverError::InvalidOverride {
            path: dir.to_path_buf(),
        });
    }
    let mut searched = Vec::new();
    let mut legacy: Option<PathBuf> = None;
    for dir in start.ancestors() {
        searched.push(dir.to_path_buf());
        if has_module(fs, dir) {
            return Ok(dir.to_path_buf());
        }
        if legacy.is_none() && legacy_marker(fs, dir) {
            legacy = Some(dir.to_path_buf());
        }
    }
    if let Some(dir) = legacy {
        return Err(DiscoverError::UnsupportedLegacy { dir });
    }
    Err(DiscoverError::NotFound { searched })
}

/// Discovers the workspace using the real filesystem.
pub fn discover_real(start: &Path, override_dir: Option<&Path>) -> Result<PathBuf, DiscoverError> {
    discover(start, override_dir, &RealFs)
}

/// Workspace start directory for `bazel run`.
///
/// `bazel run` executes with the working directory inside the runfiles tree
/// under `bazel-out`, whose symlinks resolve into the execution root (nested
/// Bazel refuses there). `BUILD_WORKSPACE_DIRECTORY` points back at the
/// source workspace root, so discovery starts there when available and
/// absolute. Otherwise falls back to `cwd`. An explicit `--workspace`
/// override still wins inside `discover`/`discover_real`.
/// Single-sources the `BUILD_WORKSPACE_DIRECTORY || cwd` probe repeated in
/// the `dx` and `env` binaries; shell drivers use `tools/sh/lib.sh`.
pub fn workspace_start(cwd: &Path) -> PathBuf {
    std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .unwrap_or_else(|| cwd.to_path_buf())
}

/// Launcher selection failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LauncherError {
    /// No `.bazelversion` pin in the workspace root.
    #[error(
        "bazel_unavailable: {workspace} has no .bazelversion pin",
        workspace = workspace.display()
    )]
    MissingPin {
        /// Workspace root that was inspected.
        workspace: PathBuf,
    },
    /// The `.bazelversion` pin is blank.
    #[error(
        "bazel_unavailable: {workspace} has an empty .bazelversion pin",
        workspace = workspace.display()
    )]
    EmptyPin {
        /// Workspace root that was inspected.
        workspace: PathBuf,
    },
    /// The `.bazelversion` pin could not be read.
    #[error(
        "bazel_unavailable: cannot read {workspace}/.bazelversion: {reason}",
        workspace = workspace.display()
    )]
    UnreadablePin {
        /// Workspace root that was inspected.
        workspace: PathBuf,
        /// OS-reported reason.
        reason: String,
    },
}

/// Reads the pinned Bazel version from `<workspace>/.bazelversion`.
pub fn pinned_bazel_version(workspace: &Path, fs: &dyn Fs) -> Result<String, LauncherError> {
    let pin = workspace.join(".bazelversion");
    match fs.read_text(&pin) {
        Ok(contents) => {
            let version = contents.trim().to_owned();
            if version.is_empty() {
                return Err(LauncherError::EmptyPin {
                    workspace: workspace.to_path_buf(),
                });
            }
            Ok(version)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(LauncherError::MissingPin {
            workspace: workspace.to_path_buf(),
        }),
        Err(err) => Err(LauncherError::UnreadablePin {
            workspace: workspace.to_path_buf(),
            reason: err.to_string(),
        }),
    }
}

/// Reads the pinned Bazel version from the real filesystem.
pub fn pinned_bazel_version_real(workspace: &Path) -> Result<String, LauncherError> {
    pinned_bazel_version(workspace, &RealFs)
}

/// The only supported launcher executable: a Bazelisk-compatible
/// `bazel` on `PATH`. There is no fallback executable.
pub fn launcher_argv0() -> &'static str {
    "bazel"
}

/// Startup options every workflow command places before the Bazel
/// command: system and home rc files stay off while the discovered
/// workspace's committed `.bazelrc` remains in effect.
pub const WORKFLOW_STARTUP_OPTS: &[&str] = &["--nohome_rc", "--nosystem_rc"];

/// Compact effective Bazel scope for operation summaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Repository-wide scope (`//...`).
    Repository,
    /// Canonical recursive pattern such as `//src/auth/...`.
    Pattern(String),
    /// User-supplied canonical labels or patterns, in order.
    Labels(Vec<String>),
    /// Graph-resolved owner labels, deterministically sorted by the caller.
    ResolvedOwners(Vec<String>),
    /// Resolved target count when labels are not listed.
    Count(usize),
}

/// Renders the compact scope without expanding patterns.
pub fn describe_scope(scope: &Scope) -> String {
    match scope {
        Scope::Repository => "//...".to_owned(),
        Scope::Pattern(pattern) => pattern.clone(),
        Scope::Labels(labels) => labels.join(" "),
        Scope::ResolvedOwners(owners) => owners.join(" "),
        Scope::Count(count) => {
            if *count == 1 {
                "1 target".to_owned()
            } else {
                format!("{count} targets")
            }
        }
    }
}

/// Renders a safe operation summary. The summary never contains
/// executable argv, option values, environment values, or a
/// reconstructed shell command.
pub fn operation_summary(command: &str, phase: &str, scope: &Scope) -> String {
    format!("Running {command} {phase} for {}", describe_scope(scope))
}

/// One workflow-protected Bazel flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedFlag {
    /// Bare flag name without leading dashes, e.g. `keep_going`.
    pub name: String,
    /// Required invocation when repetition is accepted, e.g.
    /// `--keep_going`. `None` rejects every use.
    pub required: Option<String>,
}

/// Forwarding failure. Messages carry the flag name and policy only,
/// never the supplied or generated option value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ForwardError {
    /// A user option conflicts with required workflow policy.
    #[error("conflicting_option: --{flag} conflicts with required workflow policy")]
    ConflictingOption {
        /// Bare flag name.
        flag: String,
    },
    /// A Bazel startup option was passed as a command option.
    #[error("invalid_usage: --{flag} is a startup option; use dx bazel")]
    StartupOption {
        /// Bare flag name.
        flag: String,
    },
    /// Test-binary argument syntax was passed to a workflow command.
    #[error("invalid_usage: --{flag} targets the test binary; use dx bazel")]
    TestBinaryArgs {
        /// Bare flag name.
        flag: String,
    },
    /// A required workflow option is missing from the planned required
    /// set. This signals an internal wiring error, never user input.
    #[error("internal_error: required workflow option --{flag} is missing")]
    InvalidRequiredOption {
        /// Bare flag name.
        flag: String,
    },
    /// A required command setting is malformed. Settings must be
    /// `--name=value` workflow options; anything else is rejected
    /// instead of silently protecting the wrong flag name.
    #[error("internal_error: malformed required setting {option}; expected --name=value")]
    InvalidSetting {
        /// The malformed setting option.
        option: String,
    },
    /// A command does not support the requested plan. This signals an
    /// internal dispatch error, never user input.
    #[error("internal_error: {command} does not support this plan")]
    UnsupportedCommand {
        /// Command name.
        command: String,
    },
}

fn flag_name(arg: &str) -> Option<String> {
    if !arg.starts_with("--") {
        return None;
    }
    let bare = &arg[2..];
    if bare.is_empty() {
        return None;
    }
    Some(bare.split('=').next().unwrap_or("").to_owned()).filter(|name| !name.is_empty())
}

/// Reports whether `arg` is a Bazel startup option, which workflow
/// commands reject as a command option.
pub fn is_startup_option(arg: &str) -> bool {
    const STARTUP: &[&str] = &[
        "bazelrc",
        "home_rc",
        "nohome_rc",
        "system_rc",
        "nosystem_rc",
        "output_base",
        "output_user_root",
        "host_jvm_args",
        "server_jvm_out",
    ];
    match flag_name(arg) {
        Some(name) => STARTUP.contains(&name.as_str()),
        None => false,
    }
}

/// Reports whether `arg` addresses the test binary rather than Bazel.
pub fn is_test_binary_arg(arg: &str) -> bool {
    match flag_name(arg) {
        Some(name) => name == "test_arg",
        None => false,
    }
}

/// Checks user options against protected workflow flags.
///
/// Repeating the exact required value is accepted and canonicalized;
/// any other use of a protected name fails. Unrelated options keep
/// their order. Neither the supplied nor the generated value is
/// echoed in errors.
pub fn check_protected(
    user_args: &[String],
    protected: &[ProtectedFlag],
) -> Result<Vec<String>, ForwardError> {
    let mut kept = Vec::with_capacity(user_args.len());
    for arg in user_args {
        let name = flag_name(arg);
        let guard = match &name {
            Some(name) => protected.iter().find(|flag| flag.name == *name),
            None => None,
        };
        match (guard, name) {
            (Some(guard), _) => match &guard.required {
                Some(required) if arg == required => kept.push(required.clone()),
                Some(_) | None => {
                    return Err(ForwardError::ConflictingOption {
                        flag: guard.name.clone(),
                    });
                }
            },
            (None, _) => kept.push(arg.clone()),
        }
    }
    Ok(kept)
}

/// Builds the exact workflow subprocess argv.
///
/// Layout: `bazel <startup> <command> <required> <user> <labels>`.
/// User options are validated as Bazel command options: startup
/// options and test-binary argument syntax are rejected with guidance
/// to use `dx bazel`, protected conflicts fail, and exact repetitions
/// of required values are canonicalized.
pub fn build_workflow_argv(
    bazel_command: &str,
    user_options: &[String],
    required_options: &[String],
    protected: &[ProtectedFlag],
    labels: &[String],
) -> Result<Vec<String>, ForwardError> {
    for arg in user_options {
        if is_startup_option(arg) {
            let flag = flag_name(arg).unwrap_or_default();
            return Err(ForwardError::StartupOption { flag });
        }
        if is_test_binary_arg(arg) {
            let flag = flag_name(arg).unwrap_or_default();
            return Err(ForwardError::TestBinaryArgs { flag });
        }
    }
    let canonical = check_protected(user_options, protected)?;
    let mut argv =
        Vec::with_capacity(2 + WORKFLOW_STARTUP_OPTS.len() + canonical.len() + labels.len());
    argv.push(launcher_argv0().to_owned());
    for opt in WORKFLOW_STARTUP_OPTS {
        argv.push((*opt).to_owned());
    }
    argv.push(bazel_command.to_owned());
    argv.extend(required_options.iter().cloned());
    argv.extend(canonical);
    argv.extend(labels.iter().cloned());
    Ok(argv)
}

/// Forwards `dx bazel` arguments unchanged: no protected flags exist
/// on the transparent escape hatch.
pub fn build_bazel_passthrough(launcher: &str, args: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(1 + args.len());
    argv.push(launcher.to_owned());
    argv.extend(args.iter().cloned());
    argv
}

/// Success exit code.
pub const EXIT_SUCCESS: i32 = 0;
/// CLI-originated operational failure exit code.
pub const EXIT_OPERATIONAL: i32 = 1;
/// Pre-execution usage, workspace, scope, or owner failure exit code.
pub const EXIT_PRE_EXEC: i32 = 2;

/// Exit code for CLI-detected pre-execution failures.
pub fn pre_exec_code() -> i32 {
    EXIT_PRE_EXEC
}

/// Exit code for quality-policy and operational failures.
pub fn operational_code() -> i32 {
    EXIT_OPERATIONAL
}

/// Preserves a required Bazel subprocess exit code.
pub fn subprocess_code(code: i32) -> i32 {
    code
}

/// Whether the workflow forces Bazel `--keep_going`. Quality
/// workflows always collect across actions; build, test, and coverage
/// preserve the fail-fast default unless the user forwards the flag.
pub fn quality_keeps_going(is_quality: bool) -> bool {
    is_quality
}

/// Dry-run gating failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DryRunError {
    /// The requested work would execute a final workflow or action.
    #[error("dry-run: workflow execution is disabled")]
    WouldExecuteAction,
}

/// Decides whether dry-run may proceed. Read-only resolution queries
/// may run; final workflows never run, and resolution that would
/// require an action fails instead of executing.
pub fn dry_run_allows(is_final_workflow: bool, resolution_requires_action: bool) -> bool {
    !is_final_workflow && !resolution_requires_action
}

/// Guards dry-run execution, failing when any action would run.
pub fn dry_run_guard(
    is_final_workflow: bool,
    resolution_requires_action: bool,
) -> Result<(), DryRunError> {
    if dry_run_allows(is_final_workflow, resolution_requires_action) {
        Ok(())
    } else {
        Err(DryRunError::WouldExecuteAction)
    }
}

/// Unix signals the CLI forwards to the active Bazel process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnixSignal {
    /// Interrupt (`SIGINT`).
    Interrupt,
    /// Terminate (`SIGTERM`).
    Terminate,
}

/// Maps a CLI signal to its OS number.
pub fn signal_number(signal: UnixSignal) -> libc::c_int {
    match signal {
        UnixSignal::Interrupt => libc::SIGINT,
        UnixSignal::Terminate => libc::SIGTERM,
    }
}

/// Forwards `signo` to `pid`. The CLI binary calls this with
/// [`signal_number`] output after mapping its received signal.
pub fn forward_signal_number(pid: u32, signo: libc::c_int) -> io::Result<()> {
    let rc = unsafe { libc::kill(pid as libc::pid_t, signo) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Forwards a CLI signal to a child process id.
pub fn forward_signal(pid: u32, signal: UnixSignal) -> io::Result<()> {
    forward_signal_number(pid, signal_number(signal))
}

/// Re-raises `signo` in this process. The CLI binary calls this with
/// [`signal_number`] output after safe cleanup so shell signal
/// semantics are preserved. Signal zero performs no delivery.
pub fn reraise_number(signo: libc::c_int) -> io::Result<()> {
    let rc = unsafe { libc::raise(signo) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Captured child outcome. `code` is `None` when a signal ended the
/// child; callers treat that as an execution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildStatus {
    /// Process exit code when the child exited normally.
    pub code: Option<i32>,
}

/// Process-runner seam. Unit tests substitute a fake; the CLI binary
/// uses [`SystemRunner`].
pub trait Runner {
    /// Spawns `argv[0]` with the remaining entries as arguments in `cwd`.
    /// `env` carries extra variables (for example per-command dispatch
    /// into the child); the parent environment is always inherited.
    fn run(&self, argv: &[String], cwd: &Path, env: &[(&str, &str)]) -> io::Result<ChildStatus>;
}

/// Real runner that spawns the process directly. Argument vectors are
/// passed through and never rendered.
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, argv: &[String], cwd: &Path, env: &[(&str, &str)]) -> io::Result<ChildStatus> {
        let (binary, args) = argv.split_first().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invocation needs a binary")
        })?;
        let output = Command::new(OsStr::new(binary))
            .args(args)
            .envs(env.iter().copied())
            .current_dir(cwd)
            .output()?;
        Ok(ChildStatus {
            code: output.status.code(),
        })
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;

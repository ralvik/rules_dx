//! M06 WP1 process boundary for the `dx` CLI.
//!
//! Contract: `docs/cli/cli-contract.md`. This crate owns workspace
//! discovery, launcher selection, safe operation summaries, exact `dx
//! bazel` forwarding with protected-flag enforcement, exit-code mapping,
//! dry-run gating, Unix signal forwarding, and the process-runner seam.
//! It never renders subprocess argument vectors, option values,
//! environment values, or reconstructed shell commands.

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverError {
    /// No `MODULE.bazel` found walking up from the start directory.
    NotFound {
        /// Every ancestor directory considered, nearest first.
        searched: Vec<PathBuf>,
    },
    /// A legacy marker without `MODULE.bazel` was found instead.
    UnsupportedLegacy {
        /// Directory holding the legacy marker.
        dir: PathBuf,
    },
    /// An explicit `--workspace` override holds no module marker.
    InvalidOverride {
        /// The rejected override directory.
        path: PathBuf,
    },
}

impl std::fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoverError::NotFound { searched } => {
                write!(
                    f,
                    "workspace_not_found: no MODULE.bazel in {} directorie(s); pass --workspace <path>",
                    searched.len()
                )
            }
            DiscoverError::UnsupportedLegacy { dir } => {
                write!(
                    f,
                    "unsupported_workspace: {} has WORKSPACE without MODULE.bazel; migrate to Bzlmod",
                    dir.display()
                )
            }
            DiscoverError::InvalidOverride { path } => {
                write!(
                    f,
                    "workspace_not_found: --workspace {} has no MODULE.bazel",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DiscoverError {}

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

/// Launcher selection failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherError {
    /// No `.bazelversion` pin in the workspace root.
    MissingPin {
        /// Workspace root that was inspected.
        workspace: PathBuf,
    },
    /// The `.bazelversion` pin is blank.
    EmptyPin {
        /// Workspace root that was inspected.
        workspace: PathBuf,
    },
    /// The `.bazelversion` pin could not be read.
    UnreadablePin {
        /// Workspace root that was inspected.
        workspace: PathBuf,
        /// OS-reported reason.
        reason: String,
    },
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LauncherError::MissingPin { workspace } => {
                write!(
                    f,
                    "bazel_unavailable: {} has no .bazelversion pin",
                    workspace.display()
                )
            }
            LauncherError::EmptyPin { workspace } => {
                write!(
                    f,
                    "bazel_unavailable: {} has an empty .bazelversion pin",
                    workspace.display()
                )
            }
            LauncherError::UnreadablePin { workspace, reason } => {
                write!(
                    f,
                    "bazel_unavailable: cannot read {}/.bazelversion: {reason}",
                    workspace.display()
                )
            }
        }
    }
}

impl std::error::Error for LauncherError {}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardError {
    /// A user option conflicts with required workflow policy.
    ConflictingOption {
        /// Bare flag name.
        flag: String,
    },
    /// A Bazel startup option was passed as a command option.
    StartupOption {
        /// Bare flag name.
        flag: String,
    },
    /// Test-binary argument syntax was passed to a workflow command.
    TestBinaryArgs {
        /// Bare flag name.
        flag: String,
    },
}

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForwardError::ConflictingOption { flag } => {
                write!(
                    f,
                    "conflicting_option: --{flag} conflicts with required workflow policy"
                )
            }
            ForwardError::StartupOption { flag } => {
                write!(
                    f,
                    "invalid_usage: --{flag} is a startup option; use dx bazel"
                )
            }
            ForwardError::TestBinaryArgs { flag } => {
                write!(
                    f,
                    "invalid_usage: --{flag} targets the test binary; use dx bazel"
                )
            }
        }
    }
}

impl std::error::Error for ForwardError {}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DryRunError {
    /// The requested work would execute a final workflow or action.
    WouldExecuteAction,
}

impl std::fmt::Display for DryRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DryRunError::WouldExecuteAction => {
                write!(f, "dry-run: workflow execution is disabled")
            }
        }
    }
}

impl std::error::Error for DryRunError {}

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
    fn run(&self, argv: &[String], cwd: &Path) -> io::Result<ChildStatus>;
}

/// Real runner that spawns the process directly. Argument vectors are
/// passed through and never rendered.
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, argv: &[String], cwd: &Path) -> io::Result<ChildStatus> {
        let (binary, args) = argv.split_first().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invocation needs a binary")
        })?;
        let output = Command::new(OsStr::new(binary))
            .args(args)
            .current_dir(cwd)
            .output()?;
        Ok(ChildStatus {
            code: output.status.code(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    struct FakeFs {
        files: HashSet<PathBuf>,
        texts: HashMap<PathBuf, String>,
        errors: HashMap<PathBuf, io::ErrorKind>,
    }

    impl FakeFs {
        fn with_files(paths: &[&str]) -> FakeFs {
            FakeFs {
                files: paths.iter().map(PathBuf::from).collect(),
                texts: HashMap::new(),
                errors: HashMap::new(),
            }
        }

        fn with_text(path: &str, text: &str) -> FakeFs {
            let mut fs = FakeFs::with_files(&[path]);
            fs.texts.insert(PathBuf::from(path), text.to_owned());
            fs
        }
    }

    impl Fs for FakeFs {
        fn is_file(&self, path: &Path) -> bool {
            self.files.contains(path)
        }

        fn read_text(&self, path: &Path) -> io::Result<String> {
            if let Some(text) = self.texts.get(path) {
                return Ok(text.clone());
            }
            if let Some(kind) = self.errors.get(path) {
                return Err(io::Error::new(*kind, "injected read failure"));
            }
            Err(io::Error::new(io::ErrorKind::NotFound, "no such file"))
        }
    }

    #[test]
    fn discovers_module_at_start() {
        let fs = FakeFs::with_files(&["/repo/MODULE.bazel"]);
        let found = discover(Path::new("/repo/sub/dir"), None, &fs).expect("discover");
        assert_eq!(found, PathBuf::from("/repo"));
    }

    #[test]
    fn discovers_module_at_start_itself() {
        let fs = FakeFs::with_files(&["/repo/MODULE.bazel"]);
        let found = discover(Path::new("/repo"), None, &fs).expect("discover");
        assert_eq!(found, PathBuf::from("/repo"));
    }

    #[test]
    fn override_with_module_wins() {
        let fs = FakeFs::with_files(&["/other/MODULE.bazel"]);
        let found =
            discover(Path::new("/repo/sub"), Some(Path::new("/other")), &fs).expect("override");
        assert_eq!(found, PathBuf::from("/other"));
    }

    #[test]
    fn override_without_marker_fails() {
        let fs = FakeFs::with_files(&[]);
        let err =
            discover(Path::new("/repo"), Some(Path::new("/other")), &fs).expect_err("must fail");
        assert_eq!(
            err,
            DiscoverError::InvalidOverride {
                path: PathBuf::from("/other"),
            }
        );
        assert!(err.to_string().contains("--workspace"));
        assert!(err.to_string().contains("/other"));
    }

    #[test]
    fn override_with_legacy_reports_migration() {
        let fs = FakeFs::with_files(&["/other/WORKSPACE"]);
        let err =
            discover(Path::new("/repo"), Some(Path::new("/other")), &fs).expect_err("must fail");
        assert_eq!(
            err,
            DiscoverError::UnsupportedLegacy {
                dir: PathBuf::from("/other"),
            }
        );
        assert!(err.to_string().contains("Bzlmod"));
    }

    #[test]
    fn legacy_workspace_bazel_reports_migration() {
        let fs = FakeFs::with_files(&["/repo/WORKSPACE.bazel"]);
        let err = discover(Path::new("/repo/sub"), None, &fs).expect_err("must fail");
        assert!(matches!(err, DiscoverError::UnsupportedLegacy { .. }));
        assert!(err.to_string().contains("WORKSPACE"));
    }

    #[test]
    fn missing_module_lists_searched_and_suggests_override() {
        let fs = FakeFs::with_files(&[]);
        let err = discover(Path::new("/repo/sub"), None, &fs).expect_err("must fail");
        assert!(
            matches!(&err, DiscoverError::NotFound { searched } if searched.contains(&PathBuf::from("/repo/sub")))
        );
        assert!(
            matches!(&err, DiscoverError::NotFound { searched } if searched.contains(&PathBuf::from("/repo")))
        );
        assert!(err.to_string().contains("--workspace"));
    }

    #[test]
    fn nearest_legacy_wins_over_distant_module() {
        // A legacy marker below a module still resolves to the module:
        // discovery prefers the nearest MODULE.bazel.
        let fs = FakeFs::with_files(&["/repo/MODULE.bazel", "/repo/sub/WORKSPACE"]);
        let found = discover(Path::new("/repo/sub"), None, &fs).expect("module wins");
        assert_eq!(found, PathBuf::from("/repo"));
    }

    #[test]
    fn real_fs_roundtrip_discovers_workspace() {
        let root = std::env::temp_dir().join(format!("dx-discover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let nested = root.join("a").join("b");
        std::fs::create_dir_all(&nested).expect("dirs");
        std::fs::write(root.join("MODULE.bazel"), "module(name = \"t\")\n").expect("marker");
        let found = discover_real(&nested, None).expect("real discover");
        assert_eq!(found, root);
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn real_fs_missing_module_suggests_override() {
        let root = std::env::temp_dir().join(format!("dx-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dirs");
        let err = discover_real(&root, None).expect_err("must fail");
        assert!(err.to_string().contains("--workspace"));
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn pinned_version_trims_whitespace() {
        let fs = FakeFs::with_text("/repo/.bazelversion", "  9.2.0\n");
        let version = pinned_bazel_version(Path::new("/repo"), &fs).expect("pin");
        assert_eq!(version, "9.2.0");
    }

    #[test]
    fn missing_pin_reports_workspace() {
        let fs = FakeFs::with_files(&[]);
        let err = pinned_bazel_version(Path::new("/repo"), &fs).expect_err("must fail");
        assert_eq!(
            err,
            LauncherError::MissingPin {
                workspace: PathBuf::from("/repo"),
            }
        );
        assert!(err.to_string().contains(".bazelversion"));
    }

    #[test]
    fn empty_pin_fails() {
        let fs = FakeFs::with_text("/repo/.bazelversion", "   \n");
        let err = pinned_bazel_version(Path::new("/repo"), &fs).expect_err("must fail");
        assert_eq!(
            err,
            LauncherError::EmptyPin {
                workspace: PathBuf::from("/repo"),
            }
        );
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn unreadable_pin_reports_reason() {
        let mut fs = FakeFs::with_files(&[]);
        fs.errors.insert(
            PathBuf::from("/repo/.bazelversion"),
            io::ErrorKind::PermissionDenied,
        );
        let err = pinned_bazel_version(Path::new("/repo"), &fs).expect_err("must fail");
        assert!(matches!(err, LauncherError::UnreadablePin { .. }));
        assert!(err.to_string().contains(".bazelversion"));
    }

    #[test]
    fn launcher_has_no_fallback() {
        assert_eq!(launcher_argv0(), "bazel");
        assert_eq!(WORKFLOW_STARTUP_OPTS, &["--nohome_rc", "--nosystem_rc"]);
    }

    #[test]
    fn real_fs_reads_workspace_pin() {
        let version = pinned_bazel_version_real(Path::new("/home/kuntz/workspace/rules_dx"))
            .expect("repo pin");
        assert_eq!(version.trim(), version);
        assert!(!version.is_empty());
    }

    #[test]
    fn scope_rendering_stays_compact() {
        assert_eq!(describe_scope(&Scope::Repository), "//...");
        assert_eq!(
            describe_scope(&Scope::Pattern("//src/auth/...".to_owned())),
            "//src/auth/..."
        );
        assert_eq!(
            describe_scope(&Scope::Labels(vec![
                "//a:a".to_owned(),
                "//b/...".to_owned()
            ])),
            "//a:a //b/..."
        );
        assert_eq!(
            describe_scope(&Scope::ResolvedOwners(vec!["//a:a".to_owned()])),
            "//a:a"
        );
        assert_eq!(describe_scope(&Scope::Count(1)), "1 target");
        assert_eq!(describe_scope(&Scope::Count(3)), "3 targets");
        assert_eq!(describe_scope(&Scope::Count(0)), "0 targets");
    }

    #[test]
    fn summaries_never_render_argv() {
        let summary = operation_summary("lint", "analysis", &Scope::Count(3));
        assert_eq!(summary, "Running lint analysis for 3 targets");
        let summary = operation_summary("lint", "analysis", &Scope::Repository);
        assert_eq!(summary, "Running lint analysis for //...");
        let summary = operation_summary(
            "lint",
            "analysis",
            &Scope::Pattern("//src/auth/...".to_owned()),
        );
        assert_eq!(summary, "Running lint analysis for //src/auth/...");
    }

    #[test]
    fn protected_check_accepts_repeated_required_value() {
        let protected = vec![ProtectedFlag {
            name: "keep_going".to_owned(),
            required: Some("--keep_going".to_owned()),
        }];
        let kept = check_protected(&["--keep_going".to_owned()], &protected).expect("repeat");
        assert_eq!(kept, vec!["--keep_going".to_owned()]);
    }

    #[test]
    fn protected_check_rejects_conflict_without_echoing_values() {
        let protected = vec![ProtectedFlag {
            name: "keep_going".to_owned(),
            required: Some("--keep_going".to_owned()),
        }];
        let err =
            check_protected(&["--keep_going=false".to_owned()], &protected).expect_err("conflict");
        assert_eq!(
            err,
            ForwardError::ConflictingOption {
                flag: "keep_going".to_owned(),
            }
        );
        assert!(!err.to_string().contains("false"));
        assert!(err.to_string().contains("--keep_going"));
    }

    #[test]
    fn protected_check_rejects_bare_name_against_valued_requirement() {
        let protected = vec![ProtectedFlag {
            name: "config".to_owned(),
            required: Some("--config=dx".to_owned()),
        }];
        let err =
            check_protected(&["--config=other".to_owned()], &protected).expect_err("conflict");
        assert_eq!(
            err,
            ForwardError::ConflictingOption {
                flag: "config".to_owned(),
            }
        );
        assert!(!err.to_string().contains("other"));
        assert!(!err.to_string().contains("dx"));
        assert!(err.to_string().contains("--config"));
    }

    #[test]
    fn protected_check_rejects_unconditional_flag() {
        let protected = vec![ProtectedFlag {
            name: "build_event_json_file".to_owned(),
            required: None,
        }];
        let err = check_protected(
            &["--build_event_json_file=/tmp/bep.json".to_owned()],
            &protected,
        )
        .expect_err("conflict");
        assert_eq!(
            err,
            ForwardError::ConflictingOption {
                flag: "build_event_json_file".to_owned(),
            }
        );
        assert!(!err.to_string().contains("/tmp/bep.json"));
    }

    #[test]
    fn protected_check_preserves_unrelated_order() {
        let protected = vec![ProtectedFlag {
            name: "keep_going".to_owned(),
            required: Some("--keep_going".to_owned()),
        }];
        let kept = check_protected(
            &[
                "--jobs=4".to_owned(),
                "plain".to_owned(),
                "--keep_going".to_owned(),
            ],
            &protected,
        )
        .expect("kept");
        assert_eq!(
            kept,
            vec![
                "--jobs=4".to_owned(),
                "plain".to_owned(),
                "--keep_going".to_owned()
            ]
        );
    }

    #[test]
    fn startup_and_binary_args_are_detected() {
        assert!(is_startup_option("--output_base=/tmp/x"));
        assert!(is_startup_option("--bazelrc=/tmp/rc"));
        assert!(is_startup_option("--home_rc"));
        assert!(!is_startup_option("--jobs=4"));
        assert!(!is_startup_option("plain"));
        assert!(!is_startup_option("--"));
        assert!(is_test_binary_arg("--test_arg=fast"));
        assert!(!is_test_binary_arg("--jobs=4"));
        assert!(!is_test_binary_arg("plain"));
        assert!(flag_name("--").is_none());
        assert!(flag_name("plain").is_none());
    }

    #[test]
    fn workflow_argv_orders_startup_command_required_user_labels() {
        let protected = vec![ProtectedFlag {
            name: "keep_going".to_owned(),
            required: Some("--keep_going".to_owned()),
        }];
        let argv = build_workflow_argv(
            "build",
            &["--jobs=4".to_owned(), "--keep_going".to_owned()],
            &["--keep_going".to_owned(), "--config=dx".to_owned()],
            &protected,
            &["//...".to_owned()],
        )
        .expect("argv");
        assert_eq!(
            argv,
            vec![
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "build",
                "--keep_going",
                "--config=dx",
                "--jobs=4",
                "--keep_going",
                "//...",
            ]
        );
    }

    #[test]
    fn workflow_argv_rejects_startup_options() {
        let err = build_workflow_argv(
            "build",
            &["--output_base=/tmp/x".to_owned()],
            &[],
            &[],
            &["//...".to_owned()],
        )
        .expect_err("startup");
        assert_eq!(
            err,
            ForwardError::StartupOption {
                flag: "output_base".to_owned(),
            }
        );
        assert!(err.to_string().contains("dx bazel"));
        assert!(!err.to_string().contains("/tmp/x"));
    }

    #[test]
    fn workflow_argv_rejects_test_binary_args() {
        let err = build_workflow_argv(
            "test",
            &["--test_arg=fast".to_owned()],
            &[],
            &[],
            &["//...".to_owned()],
        )
        .expect_err("binary args");
        assert_eq!(
            err,
            ForwardError::TestBinaryArgs {
                flag: "test_arg".to_owned(),
            }
        );
        assert!(err.to_string().contains("dx bazel"));
    }

    #[test]
    fn quality_workflows_reject_nokeep_going() {
        // Quality callers protect `nokeep_going` unconditionally so a
        // conflicting `--nokeep_going` weakens no result collection.
        let protected = vec![
            ProtectedFlag {
                name: "keep_going".to_owned(),
                required: Some("--keep_going".to_owned()),
            },
            ProtectedFlag {
                name: "nokeep_going".to_owned(),
                required: None,
            },
        ];
        let err = build_workflow_argv(
            "build",
            &["--nokeep_going".to_owned()],
            &["--keep_going".to_owned()],
            &protected,
            &["//...".to_owned()],
        )
        .expect_err("nokeep_going");
        assert_eq!(
            err,
            ForwardError::ConflictingOption {
                flag: "nokeep_going".to_owned(),
            }
        );
    }

    #[test]
    fn workflow_argv_rejects_protected_conflicts() {
        let protected = vec![ProtectedFlag {
            name: "build_event_json_file".to_owned(),
            required: None,
        }];
        let err = build_workflow_argv(
            "build",
            &["--build_event_json_file=/tmp/bep.json".to_owned()],
            &["--build_event_json_file=/tmp/required.json".to_owned()],
            &protected,
            &["//...".to_owned()],
        )
        .expect_err("conflict");
        assert!(matches!(err, ForwardError::ConflictingOption { .. }));
        assert!(!err.to_string().contains("/tmp/bep.json"));
        assert!(!err.to_string().contains("/tmp/required.json"));
    }

    #[test]
    fn bazel_passthrough_forwards_unchanged() {
        let args = vec![
            "--output_base=/tmp/x".to_owned(),
            "build".to_owned(),
            "//...".to_owned(),
        ];
        assert_eq!(
            build_bazel_passthrough("bazel", &args),
            vec![
                "bazel".to_owned(),
                "--output_base=/tmp/x".to_owned(),
                "build".to_owned(),
                "//...".to_owned()
            ]
        );
    }

    #[test]
    fn exit_mapping_preserves_subprocess_codes() {
        assert_eq!(pre_exec_code(), 2);
        assert_eq!(operational_code(), 1);
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(subprocess_code(0), 0);
        assert_eq!(subprocess_code(3), 3);
        assert!(quality_keeps_going(true));
        assert!(!quality_keeps_going(false));
    }

    #[test]
    fn dry_run_never_executes_final_workflows() {
        assert!(dry_run_allows(false, false));
        assert!(!dry_run_allows(false, true));
        assert!(!dry_run_allows(true, false));
        assert!(!dry_run_allows(true, true));
        assert!(dry_run_guard(false, false).is_ok());
        assert_eq!(
            dry_run_guard(false, true),
            Err(DryRunError::WouldExecuteAction)
        );
        assert_eq!(
            dry_run_guard(true, false),
            Err(DryRunError::WouldExecuteAction)
        );
        assert!(dry_run_guard(true, true).is_err());
        assert!(DryRunError::WouldExecuteAction
            .to_string()
            .contains("dry-run"));
    }

    #[test]
    fn signal_numbers_match_os() {
        assert_eq!(signal_number(UnixSignal::Interrupt), libc::SIGINT);
        assert_eq!(signal_number(UnixSignal::Terminate), libc::SIGTERM);
    }

    #[test]
    fn forward_signal_number_checks_existence_safely() {
        // Signal zero performs error checking without delivering.
        forward_signal_number(std::process::id(), 0).expect("self exists");
        assert!(forward_signal_number(1 << 30, 0).is_err());
        assert!(forward_signal_number(std::process::id(), -1).is_err());
    }

    #[test]
    fn forward_signal_propagates_os_errors() {
        // An unallocated pid fails without delivering any signal.
        assert!(forward_signal(1 << 30, UnixSignal::Terminate).is_err());
    }

    #[test]
    fn reraise_zero_is_safe() {
        reraise_number(0).expect("raise zero");
        assert!(reraise_number(-1).is_err());
    }

    struct FakeRunner {
        status: ChildStatus,
    }

    impl Runner for FakeRunner {
        fn run(&self, argv: &[String], cwd: &Path) -> io::Result<ChildStatus> {
            assert!(!argv.is_empty());
            assert!(cwd.is_absolute() || cwd.as_os_str() == ".");
            Ok(self.status.clone())
        }
    }

    #[test]
    fn fake_runner_substitutes_the_boundary() {
        let runner = FakeRunner {
            status: ChildStatus { code: Some(3) },
        };
        let status = runner
            .run(&["bazel".to_owned(), "build".to_owned()], Path::new("."))
            .expect("fake");
        assert_eq!(status.code, Some(3));
    }

    #[test]
    fn system_runner_preserves_exit_codes() {
        let runner = SystemRunner;
        let ok = runner
            .run(&["/bin/true".to_owned()], Path::new("/"))
            .expect("true");
        assert_eq!(ok.code, Some(0));
        let fail = runner
            .run(&["/bin/false".to_owned()], Path::new("/"))
            .expect("false");
        assert_eq!(fail.code, Some(1));
    }

    #[test]
    fn system_runner_rejects_bad_invocations() {
        let runner = SystemRunner;
        assert!(runner.run(&[], Path::new("/")).is_err());
        assert!(runner
            .run(&["/nonexistent-dx-tool".to_owned()], Path::new("/"))
            .is_err());
    }
}

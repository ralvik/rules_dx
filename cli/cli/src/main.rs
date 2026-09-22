//! Thin `dx` process shim over the CLI library.
//!
//! Contract: `docs/cli/README.md`.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::todo
    )
)]

use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use dx_cli::args::parse;
use dx_cli::plan::create_run_temp_dir;
use dx_cli::{execute, Env, ProcessQueryRunner};
use dx_output::{command_finished, error_event, write_event, FinishedCounts, OutputMode};
use dx_process::{
    broken_pipe_code, discover_real, operational_code, pre_exec_code, stdout_io_code, ChildStatus,
    Runner,
};

/// Maps stdout `write_event` failure to `141` on `EPIPE`, else operational.
/// See: `docs/cli/output-protocol.md#exit-codes`.
fn stdout_output_code(error: &dx_output::OutputError) -> i32 {
    if error.is_broken_pipe() {
        broken_pipe_code()
    } else {
        operational_code()
    }
}

/// Checks one stdout `write_event`; returns the exit code on failure.
fn emit_event(out: &mut dyn Write, event: &serde_json::Value) -> Result<(), i32> {
    write_event(out, event).map_err(|error| stdout_output_code(&error))
}

/// Checks `out.flush()`; `141` on `EPIPE`, else operational.
fn flush_out(out: &mut dyn Write) -> Result<(), i32> {
    out.flush().map_err(|error| stdout_io_code(&error))
}

/// Active Bazel child for signal forwarding; see `forward_to_child`.
static CHILD_PID: AtomicU32 = AtomicU32::new(0);

/// Signal-forwarding contract:
///
/// - Forwards SIGINT and SIGTERM only, to the active child if one is
///   registered. Any other signal keeps its default disposition.
/// - Async-signal-safe: the handler performs an atomic load and `kill`
///   only; no allocation, no locking, no I/O.
/// - If no child is registered (pid 0: startup, or teardown after the
///   child was reaped) the signal is swallowed by the handler — the
///   shim itself never dies from a forwarded signal.
/// - Death semantics live in the runner, not the handler: after the
///   child is reaped and the pump thread joined, a signal death resets
///   that signal to `SIG_DFL` and re-raises, so the shell observes the
///   same signal death as a direct Bazel invocation.
/// - Known residual race: a signal landing between `wait` returning and
///   the pid clear forwards to an already-reaped pid. The window is two
///   stores wide and accepted like any supervisor's; the kill target is
///   at worst a recycled pid, never shim state.
extern "C" fn forward_to_child(signo: libc::c_int) {
    let pid = CHILD_PID.load(Ordering::SeqCst);
    if pid != 0 {
        // Handler context: `kill` is async-signal-safe; the return is
        // deliberately unchecked — a dead child means `wait` below
        // already owns the outcome.
        unsafe {
            libc::kill(pid as libc::pid_t, signo);
        }
    }
}

/// Keep raw `libc::signal` over `signal-hook` (spike):
/// `signal-hook` delivers on a spawned thread through a pipe, adding
/// latency to the forward-to-child kill and re-implementation of the
/// pid-0 swallow above, while the `kill` itself stays `unsafe libc`
/// either way — no safety win for a new dependency, lockfile churn,
/// and supply-chain review on the -hardened forwarding path.
fn install_forwarding() {
    unsafe {
        libc::signal(
            libc::SIGINT,
            forward_to_child as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            forward_to_child as *const () as libc::sighandler_t,
        );
    }
}

/// Runner used by the `dx` binary: inherits stderr always, inherits
/// stdout in text mode without a stdout report (subprocess output is
/// preserved passthrough), and otherwise pipes child stdout to `dx`
/// stderr through a pump thread so stdout stays machine-owned. Forwards
/// SIGINT/SIGTERM to the child; when the child dies from a signal, the
/// disposition is reset and the signal re-raised so shell semantics
/// hold.
/// Pump-thread teardown contract: the child is reaped first, then the
/// pid registration is cleared so late signals cannot target a reaped
/// pid, then the pump is joined so all piped stdout reaches stderr
/// before the exit status is inspected. Only after the join may a
/// signal death reset the disposition and re-raise.
struct BinaryRunner {
    inherit_stdout: bool,
}

/// Single streamed-spawn owner for the binary runner: spawns `argv[0]`
/// with piped-or-inherited stdout, stderr always inherited, through the
/// `CHILD_PID` registration plus stdout-to-stderr pump and signal
/// re-raise teardown. `clear_env` selects the hermetic secrets path
/// (only explicit env reaches the child); otherwise the parent
/// environment is inherited.
fn spawn_streamed(
    argv: &[String],
    cwd: &Path,
    env: &[(&str, &str)],
    clear_env: bool,
    inherit_stdout: bool,
) -> io::Result<ChildStatus> {
    let (binary, args) = argv
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invocation needs a binary"))?;
    let mut command = Command::new(binary);
    command.args(args).current_dir(cwd).stderr(Stdio::inherit());
    if clear_env {
        command.env_clear();
    }
    command.envs(env.iter().copied());
    if inherit_stdout {
        command.stdout(Stdio::inherit());
    } else {
        command.stdout(Stdio::piped());
    }
    let mut child = command.spawn()?;
    CHILD_PID.store(child.id(), Ordering::SeqCst);
    let pump = if inherit_stdout {
        None
    } else {
        let stdout = child.stdout.take();
        Some(std::thread::spawn(move || {
            if let Some(mut stdout) = stdout {
                let mut stderr = io::stderr();
                let _ = io::copy(&mut stdout, &mut stderr);
            }
        }))
    };
    let status = child.wait();
    CHILD_PID.store(0, Ordering::SeqCst);
    if let Some(pump) = pump {
        let _ = pump.join();
    }
    let status = status?;
    // Fail-fast policy: signal re-raise exists only on
    // unix (`ExitStatusExt::signal`); Windows reports codes only, so
    // this stays gated instead of a portable fake. Safe cleanup is
    // complete (child reaped, pump joined): restore the default
    // disposition and re-raise so the shell observes the signal death.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signo) = status.signal() {
            unsafe {
                libc::signal(signo, libc::SIG_DFL);
                libc::raise(signo);
            }
        }
    }
    Ok(ChildStatus {
        code: status.code(),
    })
}

impl Runner for BinaryRunner {
    fn run_hermetic(
        &self,
        argv: &[String],
        cwd: &Path,
        env: &[(&str, &str)],
    ) -> io::Result<ChildStatus> {
        // Secrets path clears ambient configuration before spawning:
        // only the explicit hermetic env reaches Gitleaks.
        spawn_streamed(argv, cwd, env, true, self.inherit_stdout)
    }

    fn gitleaks_tool(&self) -> Option<std::path::PathBuf> {
        std::env::var_os("DX_GITLEAKS_BIN")
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_absolute())
    }

    fn git_tool(&self) -> Option<std::path::PathBuf> {
        std::env::var_os("DX_GIT_BIN")
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_absolute())
    }

    fn run(&self, argv: &[String], cwd: &Path, env: &[(&str, &str)]) -> io::Result<ChildStatus> {
        spawn_streamed(argv, cwd, env, false, self.inherit_stdout)
    }
}

fn usage_error(message: &str) -> i32 {
    // Single-sourced fallback registry (See: `docs/cli/commands/README.md`):
    // shares `Command::pipe_list` with `exec/common.rs::pre_exec` and
    // `args/error.rs` so drift fails the registry fixture.
    let commands = dx_cli::args::Command::pipe_list();
    let _ = writeln!(
        io::stderr(),
        "dx: {message}\nusage: dx [--workspace DIR] [--dry-run] [--quiet] [--verbose] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] [--min-coverage 0-100 (coverage only)] <{commands}> [per-command-flags] [scope ...] [-- command-options...]\nper-command flags: clean --bazel (also run `bazel clean`; default never touches Bazel outputs; distinct from `dx bazel` passthrough); owners|deps|why --configured (cquery); coverage --min-coverage; build|run|test|deploy --debug|--release; version --check|--pin|--rollback; docs --check|--serve|--port; completion <bash|zsh|fish|powershell> [--check] (no shell with --check verifies all). --check is per-command only (quality/version/update/docs/completion/check|fix; status rejects --check; see `dx <command> --help`). fix applies without rerun (run `dx check` to validate). no dx doctor; use `dx status` for diagnostics. see `dx help <command>` or `dx <command> --help`."
    );
    pre_exec_code()
}

fn main() {
    install_forwarding();
    let code = run();
    std::process::exit(code);
}

fn run() -> i32 {
    // LCOV_EXCL_START - policy: docs/testing/README.md#coverage
    // `args_os` keeps non-UTF8 bytes opaque so they fail as `InvalidScope`
    // (exit 2) instead of panicking in `args`; `workspace` plus `targets`
    // travel as `OsString` in the grammar for the same reason.
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // Hidden completion-time callback for the generated `dx completion`
    // scripts (See: `docs/cli/commands/completion.md`): answers scope
    // and task candidates without parsing, workspace gates, or Bazel so
    // completion stays fast and never breaks typing. Never a `Command`,
    // never in `--help` or usage.
    if args.first().is_some_and(|first| {
        first.as_os_str() == std::ffi::OsStr::new(dx_cli::args::COMPLETE_SUBCOMMAND)
    }) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let code = dx_cli::args::run_complete(&args[1..], &cwd, &mut out);
        // Completion candidates truncate like any stdout: `EPIPE` is `141`.
        // See: `docs/cli/output-protocol.md#exit-codes`.
        if code != 0 {
            return code;
        }
        if let Err(exit) = flush_out(&mut out) {
            return exit;
        }
        return code;
    }
    let mut invocation = match parse(&args) {
        Ok(invocation) => invocation,
        Err(dx_cli::args::ArgsError::Help { text }) => {
            // `--help`/`-h`: human text on stdout, exit 0,
            // deliberately outside machine-output guarantees (no NDJSON).
            let stdout = io::stdout();
            let mut out = stdout.lock();
            if let Err(error) = write!(out, "{text}") {
                return stdout_io_code(&error);
            }
            if let Err(exit) = flush_out(&mut out) {
                return exit;
            }
            return 0;
        }
        Err(error) => return usage_error(&error.to_string()),
    };
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
    // Structured diagnostics: tracing subscriber init is
    // idempotent and emits nothing by default, keeping runs byte-identical
    // unless `--verbose` (info) or `RUST_LOG` overrides the filter.
    dx_output::init_diagnostics(invocation.verbose);
    tracing::info!(
        command = invocation.command.name(),
        verbose = invocation.verbose,
        quiet = invocation.quiet,
        "dx invocation parsed"
    );
    // Platform gate: unqualified hosts refuse cleanly with a
    // qualification pointer before any Bazel work starts, never partial
    // execution presented as success. Usage errors above still surface so
    // typos stay diagnosable on every host.
    if let Some(message) = dx_cli::platform::refusal(std::env::consts::OS, std::env::consts::ARCH) {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut err = io::stderr();
        let _ = writeln!(err, "dx: {message}");
        if invocation.output == OutputMode::Json {
            if let Ok(event) = error_event("unsupported_platform", &message, None, None, None) {
                if let Err(exit) = emit_event(&mut out, &event) {
                    return exit;
                }
            }
            if let Err(exit) = emit_event(
                &mut out,
                &command_finished(
                    operational_code(),
                    &FinishedCounts {
                        results_complete: Some(false),
                        ..FinishedCounts::default()
                    },
                ),
            ) {
                return exit;
            }
        }
        if let Err(exit) = flush_out(&mut out) {
            return exit;
        }
        return operational_code();
    }
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let _ = writeln!(io::stderr(), "dx: cannot read working directory: {error}");
            return pre_exec_code();
        }
    };
    // Workspace start: single-sourced via
    // `dx_process::workspace_start` (shell: `tools/sh/lib.sh`).
    let start = dx_process::workspace_start(&cwd);
    let workspace = match discover_real(&start, invocation.workspace.as_deref().map(Path::new)) {
        Ok(workspace) => workspace,
        Err(error) => {
            // `dx init` bootstraps a new repository without an existing
            // MODULE.bazel (see `docs/cli/commands/hooks.md`): fall back
            // to the explicit `--workspace` dir, else the start dir, so
            // the absent-only scaffold has a root to write under. Every
            // other command still requires discovery.
            if invocation.command == dx_cli::args::Command::Init {
                invocation
                    .workspace
                    .as_deref()
                    .map(Path::new)
                    .map(|raw| {
                        let display = dx_process::resolve_override_display(raw, &start);
                        dx_process::canonicalize_or_keep(&display)
                    })
                    .unwrap_or(start)
            } else {
                let _ = writeln!(io::stderr(), "dx: cannot resolve workspace: {error}");
                return pre_exec_code();
            }
        }
    };
    // Version-skew gate: a drifted `.dx/version` pin refuses
    // mutating/generating commands before any Bazel work starts, with a
    // diagnostic naming the three versions and the repair. Read-only
    // commands warn and proceed; the diagnose/repair path stays usable.
    // One small file read, no subprocesses.
    // `--here` (`--cwd` alias, See: `docs/cli/target-resolution.md`, issue #699): explicit cwd scope only.
    // Consumed here into a directory scope (`//path/...`; `//...` at the
    // root) so downstream resolution reuses the existing path verbatim.
    // The no-flag default stays `//...`; explicit scopes never combine.
    if invocation.here {
        match dx_cli::args::apply_here(&invocation, &workspace, &cwd) {
            Ok(resolved) => invocation = resolved,
            Err(detail) => {
                let _ = writeln!(io::stderr(), "dx: {detail}");
                return pre_exec_code();
            }
        }
    }
    let pin = dx_cli::skew::read_pin(&workspace);
    match dx_cli::skew::disposition(
        invocation.command,
        invocation.dry_run,
        dx_cli::skew::is_skewed(&pin),
    ) {
        dx_cli::skew::SkewDisposition::Proceed => {}
        dx_cli::skew::SkewDisposition::Warn => {
            let _ = writeln!(
                io::stderr(),
                "dx: warning: {} (proceeding: read-only or --dry-run invocation)",
                dx_cli::skew::diagnostic(&pin)
            );
        }
        dx_cli::skew::SkewDisposition::Refuse => {
            let message = dx_cli::skew::diagnostic(&pin);
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let mut err = io::stderr();
            let _ = writeln!(err, "dx: {message}");
            if invocation.output == OutputMode::Json {
                if let Ok(event) = error_event("version_skew", &message, None, None, None) {
                    if let Err(exit) = emit_event(&mut out, &event) {
                        return exit;
                    }
                }
                if let Err(exit) = emit_event(
                    &mut out,
                    &command_finished(
                        operational_code(),
                        &FinishedCounts {
                            results_complete: Some(false),
                            ..FinishedCounts::default()
                        },
                    ),
                ) {
                    return exit;
                }
            }
            if let Err(exit) = flush_out(&mut out) {
                return exit;
            }
            return operational_code();
        }
    }
    let pid = std::process::id();
    // Unique scratch directory per invocation: `tempfile`
    // mints an exclusive `dx-run-*` directory so recycled PIDs and
    // concurrent runs never share BEP/intended state. The same nonce
    // flows into `Env` so the per-run file names inherit the uniqueness.
    let (temp_dir, nonce) = match create_run_temp_dir(&std::env::temp_dir()) {
        Ok(run) => run,
        Err(error) => {
            let _ = writeln!(
                io::stderr(),
                "dx: cannot create temporary directory: {error}"
            );
            return pre_exec_code();
        }
    };
    let inherit_stdout = matches!(invocation.output, OutputMode::Text { .. })
        && !invocation
            .reports
            .iter()
            .any(|report| report.destination == "-");
    let runner = BinaryRunner { inherit_stdout };
    let query_runner = ProcessQueryRunner;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut err = io::stderr();
    let code = execute(
        &invocation,
        Env {
            workspace: &workspace,
            runner: &runner,
            query_runner: &query_runner,
            temp_dir: temp_dir.path(),
            pid,
            nonce,
            out: &mut out,
            err: &mut err,
            // The local-only `dx run` gate reads the launch
            // environment once here via the shared owner; execution
            // below takes the bit by value so tests stay hermetic.
            // See: `docs/cli/commands/watch.md`.
            ci: dx_process::is_ci(),
        },
    );
    // Truncated stdout overrides the command code: NDJSON consumers
    // distinguish truncation (`141`) from success. See output protocol.
    if let Err(exit) = flush_out(&mut out) {
        let temp_display = temp_dir.path().display().to_string();
        if let Err(error) = temp_dir.close() {
            let _ = writeln!(
                io::stderr(),
                "dx: warning: cannot remove temporary directory {temp_display}: {error}",
            );
        }
        return exit;
    }
    // Cleanup failure is a warning, not silent: a stale `dx-run-*`
    // directory otherwise accumulates with no signal to the operator.
    // `TempDir::close` removes explicitly so the warning survives;
    // dropping without close would clean silently on success.
    let temp_display = temp_dir.path().display().to_string();
    if let Err(error) = temp_dir.close() {
        let _ = writeln!(
            io::stderr(),
            "dx: warning: cannot remove temporary directory {temp_display}: {error}",
        );
    }
    code
}

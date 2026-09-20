//! Thin `dx` process shim over the CLI library.
//!
//! Contract: `docs/cli/README.md`.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - reason: thin binary shim; process wiring, signal forwarding, and stdio routing are operational behaviors verified by build and dogfood execution, not unit coverage.
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use dx_cli::args::parse;
use dx_cli::plan::create_run_temp_dir;
use dx_cli::{execute, Env, ProcessQueryRunner};
use dx_output::{command_finished, error_event, write_event, FinishedCounts, OutputMode};
use dx_process::{discover_real, operational_code, pre_exec_code, ChildStatus, Runner};

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
/// and supply-chain review on the #93-hardened forwarding path.
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

impl Runner for BinaryRunner {
    fn run(&self, argv: &[String], cwd: &Path, env: &[(&str, &str)]) -> io::Result<ChildStatus> {
        let (binary, args) = argv.split_first().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invocation needs a binary")
        })?;
        let mut command = Command::new(binary);
        command
            .args(args)
            .envs(env.iter().copied())
            .current_dir(cwd)
            .stderr(Stdio::inherit());
        if self.inherit_stdout {
            command.stdout(Stdio::inherit());
        } else {
            command.stdout(Stdio::piped());
        }
        let mut child = command.spawn()?;
        CHILD_PID.store(child.id(), Ordering::SeqCst);
        let pump = if self.inherit_stdout {
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
        // Issue #320 fail-fast policy: signal re-raise exists only on
        // unix (`ExitStatusExt::signal`); Windows reports codes only, so
        // this stays gated instead of a portable fake.
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signo) = status.signal() {
                // Safe cleanup is complete (child reaped, pump joined):
                // restore the default disposition and re-raise so the
                // shell observes the signal death.
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
}

fn usage_error(message: &str) -> i32 {
    let _ = writeln!(
        io::stderr(),
        "dx: {message}\nusage: dx [--workspace DIR] [--dry-run] [--quiet] [--verbose] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] [--min-coverage 0-100 (coverage only)] <audit|lint|typecheck|format|generate|build|test|coverage|run|deploy|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel> [--check] [scope ...] [-- command-options...]\nper-command flags: clean --bazel (also run `bazel clean`; distinct from `dx bazel` passthrough); owners|deps|why --configured (cquery); coverage --min-coverage; build|run|test|deploy --debug|--release; version --check|--pin|--rollback. see `dx <command> --help`."
    );
    pre_exec_code()
}

fn main() {
    install_forwarding();
    let code = run();
    std::process::exit(code);
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let invocation = match parse(&args) {
        Ok(invocation) => invocation,
        Err(dx_cli::args::ArgsError::Help { text }) => {
            // `--help`/`-h`: human text on stdout, exit 0,
            // deliberately outside machine-output guarantees (no NDJSON).
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let _ = write!(out, "{text}");
            let _ = out.flush();
            return 0;
        }
        Err(error) => return usage_error(&error.to_string()),
    };
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
    // Platform gate (issues #298/#410/#411/#412/#413/#414): unqualified hosts refuse cleanly with a
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
                let _ = write_event(&mut out, &event);
            }
            let _ = write_event(
                &mut out,
                &command_finished(
                    operational_code(),
                    &FinishedCounts {
                        results_complete: Some(false),
                        ..FinishedCounts::default()
                    },
                ),
            );
        }
        let _ = out.flush();
        return operational_code();
    }
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let _ = writeln!(io::stderr(), "dx: cannot read working directory: {error}");
            return pre_exec_code();
        }
    };
    // Workspace start (issue #319): single-sourced via
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
                    .map(Path::to_path_buf)
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
                    let _ = write_event(&mut out, &event);
                }
                let _ = write_event(
                    &mut out,
                    &command_finished(
                        operational_code(),
                        &FinishedCounts {
                            results_complete: Some(false),
                            ..FinishedCounts::default()
                        },
                    ),
                );
            }
            let _ = out.flush();
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
            // environment once here; execution below takes the bit
            // by value so tests stay hermetic.
            ci: std::env::var("CI").is_ok_and(|value| value == "true"),
        },
    );
    let _ = out.flush();
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
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

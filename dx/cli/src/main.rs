//! `dx` binary: thin CLI shim over the quality command library.
//!
//! All command semantics live in the library and are unit-tested there.
//! This shim owns process concerns only: argument parsing, workspace
//! discovery, temporary-directory lifetime, subprocess stdio routing per
//! the output protocol (text preserves subprocess stdout; every other
//! mode moves it to stderr so stdout stays machine-owned), SIGINT/SIGTERM
//! forwarding to the active Bazel process with shell signal semantics,
//! and exit-code propagation.

// LCOV_EXCL_START - reason: thin binary shim; process wiring, signal forwarding, and stdio routing are operational behaviors verified by build and dogfood execution, not unit coverage.
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use dx_cli::args::parse;
use dx_cli::{execute, Env, ProcessQueryRunner};
use dx_output::OutputMode;
use dx_process::{discover_real, pre_exec_code, ChildStatus, Runner};

/// Pid of the active Bazel child, if any. Written before waiting and
/// cleared after; the signal handler forwards to it.
static CHILD_PID: AtomicU32 = AtomicU32::new(0);

/// Forwards `signo` to the active child, if any. Async-signal-safe:
/// only an atomic load and `kill`.
extern "C" fn forward_to_child(signo: libc::c_int) {
    let pid = CHILD_PID.load(Ordering::Relaxed);
    if pid != 0 {
        unsafe {
            libc::kill(pid as libc::pid_t, signo);
        }
    }
}

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
        CHILD_PID.store(child.id(), Ordering::Relaxed);
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
        CHILD_PID.store(0, Ordering::Relaxed);
        if let Some(pump) = pump {
            let _ = pump.join();
        }
        let status = status?;
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
        "dx: {message}\nusage: dx [--workspace DIR] [--dry-run] [--quiet] [--output text|diff|json] [--report <format>=<destination>]... [--fail-on info|warning|error] <lint|typecheck|format|generate|build|test|coverage|run> [--check] [scope ...] [-- command-options...]"
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
        Err(error) => return usage_error(&error.to_string()),
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let _ = writeln!(io::stderr(), "dx: cannot read working directory: {error}");
            return pre_exec_code();
        }
    };
    // `bazel run` executes with the working directory inside the runfiles
    // tree under bazel-out, whose symlinks resolve into the execution root.
    // Upward MODULE.bazel search from there would find the execution root
    // instead of the source workspace, and nested Bazel would refuse to
    // run. `BUILD_WORKSPACE_DIRECTORY` is set by `bazel run` to the source
    // workspace root, so discovery starts there when available. An explicit
    // `--workspace` override still wins inside discovery.
    let start = std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .unwrap_or(cwd);
    let workspace = match discover_real(&start, invocation.workspace.as_deref().map(Path::new)) {
        Ok(workspace) => workspace,
        Err(error) => {
            let _ = writeln!(io::stderr(), "dx: cannot resolve workspace: {error:?}");
            return pre_exec_code();
        }
    };
    let pid = std::process::id();
    let temp_dir: PathBuf = std::env::temp_dir().join(format!("dx-run-{pid}"));
    if let Err(error) = std::fs::create_dir_all(&temp_dir) {
        let _ = writeln!(
            io::stderr(),
            "dx: cannot create temporary directory {}: {error}",
            temp_dir.display()
        );
        return pre_exec_code();
    }
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
            temp_dir: &temp_dir,
            pid,
            nonce: 0,
            out: &mut out,
            err: &mut err,
        },
    );
    let _ = out.flush();
    let _ = std::fs::remove_dir_all(&temp_dir);
    code
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

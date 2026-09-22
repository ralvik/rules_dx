//! Adoption version execution (`version`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_version`] — pin,
//! rollback, check, and report. Re-exported through `super` so the
//! dispatch path stays `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;
use crate::exec::common::check_stdout_write;
use dx_process::operational_code;

use super::{operational, pre_exec, summaries_suppressed};

/// Runs `dx version`: `--pin` / `--rollback` mutate the pin (dry-run
/// plans are summaries, suppressed under `--quiet`), `--check`
/// validates without mutating, bare reports the binary, module, and
/// pin. Flag combinations that mix check with mutation (or the two
/// mutations with each other) are usage errors.
pub(crate) fn execute_version(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // `--check` validates without mutating and `--pin`/`--rollback`
    // mutate without validating: combining them is a usage error, as
    // is combining the two mutations with each other.
    if invocation.check && (invocation.pin.is_some() || invocation.rollback) {
        return pre_exec(
            err,
            "version --check does not combine with --pin or --rollback",
        );
    }
    if invocation.pin.is_some() && invocation.rollback {
        return pre_exec(err, "version --pin and --rollback are mutually exclusive");
    }
    if invocation.rollback {
        // Rollback re-pins the previous release recorded by the
        // ruleset (`dx_adopt::PREVIOUS_VERSION`); there is no deeper
        // pin history to walk back through. Rolling to the current pin
        // or to an unknown version is rejected by the admissibility
        // gate, not silently re-pinned. A missing or unreadable pin
        // fails closed without forging a default (See:
        // `docs/cli/commands/status-version.md`).
        let previous = dx_adopt::PREVIOUS_VERSION;
        let current = match dx_adopt::read_version_pin(workspace) {
            Ok(pin) => pin,
            Err(error) => return operational(out, err, &error.to_string()),
        };
        if current.is_empty() || !dx_adopt::rollback_re_pins_previous(&current, previous, previous)
        {
            return operational(
                out,
                err,
                &format!(
                    "rollback refused: pin {current:?} is not newer than previous release {previous:?}"
                ),
            );
        }
        if invocation.dry_run {
            if !summaries_suppressed(invocation) {
                if let Err(exit) =
                    check_stdout_write(writeln!(out, "would pin {previous} (rollback)"))
                {
                    return exit;
                }
            }
            return 0;
        }
        return match dx_adopt::write_version_pin(workspace, previous) {
            Ok(()) => {
                if let Err(exit) = check_stdout_write(writeln!(out, "pinned {previous} (rollback)"))
                {
                    return exit;
                }
                0
            }
            Err(error) => operational(out, err, &error.to_string()),
        };
    }
    if let Some(pin) = &invocation.pin {
        if !dx_adopt::version_pin_matches_module(pin, dx_adopt::MODULE_VERSION) {
            return operational(
                out,
                err,
                &format!("version pin must equal module {}", dx_adopt::MODULE_VERSION),
            );
        }
        if invocation.dry_run {
            if !summaries_suppressed(invocation) {
                if let Err(exit) = check_stdout_write(writeln!(out, "would pin {pin}")) {
                    return exit;
                }
            }
            return 0;
        }
        return match dx_adopt::write_version_pin(workspace, pin) {
            Ok(()) => {
                if let Err(exit) = check_stdout_write(writeln!(out, "pinned {pin}")) {
                    return exit;
                }
                0
            }
            Err(error) => operational(out, err, &error.to_string()),
        };
    }
    if invocation.dry_run {
        if !summaries_suppressed(invocation) {
            if invocation.check {
                if let Err(exit) = check_stdout_write(writeln!(out, "would check version pin")) {
                    return exit;
                }
            } else {
                if let Err(exit) = check_stdout_write(writeln!(out, "would report version")) {
                    return exit;
                }
            }
        }
        return 0;
    }
    // A missing or unreadable pin fails closed: propagate the read
    // error instead of forging a default ok (See:
    // `docs/cli/commands/status-version.md`).
    let current = match dx_adopt::read_version_pin(workspace) {
        Ok(pin) => pin,
        Err(error) => return operational(out, err, &error.to_string()),
    };
    if invocation.check {
        if dx_adopt::version_pin_matches_module(&current, dx_adopt::MODULE_VERSION) {
            if let Err(exit) = check_stdout_write(writeln!(out, "version ok: {current}")) {
                return exit;
            }
            0
        } else {
            let _ = writeln!(
                err,
                "dx: version drift: {current} != {}",
                dx_adopt::MODULE_VERSION
            );
            operational_code()
        }
    } else {
        if let Err(exit) = check_stdout_write(writeln!(out, "dx {}", dx_adopt::DX_VERSION)) {
            return exit;
        }
        if let Err(exit) =
            check_stdout_write(writeln!(out, "rules_dx {}", dx_adopt::MODULE_VERSION))
        {
            return exit;
        }
        if let Err(exit) = check_stdout_write(writeln!(out, "pin {current}")) {
            return exit;
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adopt::{execute_adoption, AdoptEnv};
    use crate::args::parse;
    use std::io;

    fn invocation(words: &[&str]) -> Invocation {
        parse(&words.iter().map(ToString::to_string).collect::<Vec<_>>()).expect("parse")
    }

    struct NullQuery;

    impl crate::resolve::QueryRunner for NullQuery {
        fn run_query(
            &self,
            _argv: &[String],
            _cwd: &std::path::Path,
        ) -> io::Result<crate::resolve::QueryResult> {
            Ok(crate::resolve::QueryResult {
                code: Some(0),
                stdout: b"//a:one\n".to_vec(),
                stderr: Vec::new(),
            })
        }
    }

    struct NullRunner;

    impl dx_process::Runner for NullRunner {
        fn run(
            &self,
            _argv: &[String],
            _cwd: &std::path::Path,
            _env: &[(&str, &str)],
        ) -> io::Result<dx_process::ChildStatus> {
            Ok(dx_process::ChildStatus { code: Some(0) })
        }
    }

    #[test]
    fn version_pins_and_reports() {
        let scratch = dx_test_scratch::scratch("dx-adopt-version-pins-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let pin = invocation(&["version", "--pin=0.0.0"]);
        let code = execute_adoption(
            &pin,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(root.join(".dx/version").exists());
    }

    #[test]
    fn version_rollback_pins_previous_release() {
        let scratch = dx_test_scratch::scratch("dx-adopt-version-rollback-");
        let root = scratch.path().to_path_buf();
        let inv = invocation(&["version", "--rollback"]);
        // Pin-less tree refuses without creating a pin: a missing pin
        // fails closed on the read error.
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err)
            .expect("err")
            .contains("read version pin"));
        assert!(!root.join(".dx/version").exists());
        // An empty pin is also no prior pin: refuse without writing.
        std::fs::create_dir_all(root.join(".dx")).expect("dx");
        std::fs::write(root.join(".dx/version"), "\n").expect("empty pin");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err)
            .expect("err")
            .contains("rollback refused"));
        // A drifted pin rolls back to the previous release.
        std::fs::write(root.join(".dx/version"), "9.9.9\n").expect("drifted pin");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).expect("out").contains("rollback"));
        let pinned = std::fs::read_to_string(root.join(".dx/version")).expect("pin");
        assert_eq!(pinned.trim(), dx_adopt::PREVIOUS_VERSION);
        // Rolling back twice is refused: the pin already equals the
        // previous release, so there is nothing to restore.
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err)
            .expect("err")
            .contains("rollback refused"));
    }

    #[test]
    fn version_rejects_combined_mutation_and_check_flags() {
        let scratch = dx_test_scratch::scratch("dx-adopt-version-conflicts-");
        let root = scratch.path().to_path_buf();
        for words in [
            vec!["version", "--pin=0.0.0", "--rollback"],
            vec!["version", "--pin=0.0.0", "--check"],
            vec!["version", "--rollback", "--check"],
        ] {
            let mut out = Vec::new();
            let mut err = Vec::new();
            let inv = invocation(&words);
            let code = execute_adoption(
                &inv,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    runner: &NullRunner,
                    out: &mut out,
                    err: &mut err,
                },
            );
            assert_eq!(code, 2, "words: {words:?}");
        }
    }

    #[test]
    fn version_check_reports_drift() {
        let scratch = dx_test_scratch::scratch("dx-adopt-version-check-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(root.join(".dx")).expect("dx");
        std::fs::write(root.join(".dx/version"), "0.0.0\n").expect("pin");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let inv = invocation(&["version", "--check"]);
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).expect("out").contains("version ok"));
        std::fs::write(root.join(".dx/version"), "0.1.0\n").expect("drift");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err).expect("err").contains("drift"));
    }

    #[test]
    fn version_missing_pin_fails_closed() {
        for words in [vec!["version", "--check"], vec!["version"]] {
            let scratch = dx_test_scratch::scratch("dx-adopt-version-missing-");
            let root = scratch.path().to_path_buf();
            let mut out = Vec::new();
            let mut err = Vec::new();
            let inv = invocation(&words);
            let code = execute_adoption(
                &inv,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    runner: &NullRunner,
                    out: &mut out,
                    err: &mut err,
                },
            );
            assert_eq!(code, 1, "words: {words:?}");
            assert!(
                String::from_utf8(err)
                    .expect("err")
                    .contains("read version pin"),
                "words: {words:?}"
            );
            assert!(
                !String::from_utf8(out).expect("out").contains("version ok"),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn version_bare_and_check_dry_run_plan_without_reading() {
        let scratch = dx_test_scratch::scratch("dx-adopt-version-dry-bare-");
        let root = scratch.path().to_path_buf();
        for (words, want) in [
            (vec!["version", "--dry-run"], "would report version"),
            (
                vec!["version", "--check", "--dry-run"],
                "would check version pin",
            ),
        ] {
            let mut out = Vec::new();
            let mut err = Vec::new();
            let inv = invocation(&words);
            let code = execute_adoption(
                &inv,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    runner: &NullRunner,
                    out: &mut out,
                    err: &mut err,
                },
            );
            assert_eq!(code, 0, "words: {words:?}");
            assert!(
                String::from_utf8(out).expect("out").contains(want),
                "words: {words:?}"
            );
        }
    }
}

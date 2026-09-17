//! Adoption version execution (`version`, issue #236).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_version`] — pin,
//! rollback, check, and report. Re-exported through `super` so the
//! dispatch path stays `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;
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
        // gate, not silently re-pinned.
        let previous = dx_adopt::PREVIOUS_VERSION;
        let current = dx_adopt::read_version_pin(workspace).unwrap_or_default();
        if !dx_adopt::rollback_re_pins_previous(&current, previous, previous) {
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
                let _ = writeln!(out, "would pin {previous} (rollback)");
            }
            return 0;
        }
        return match dx_adopt::write_version_pin(workspace, previous) {
            Ok(()) => {
                let _ = writeln!(out, "pinned {previous} (rollback)");
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
                let _ = writeln!(out, "would pin {pin}");
            }
            return 0;
        }
        return match dx_adopt::write_version_pin(workspace, pin) {
            Ok(()) => {
                let _ = writeln!(out, "pinned {pin}");
                0
            }
            Err(error) => operational(out, err, &error.to_string()),
        };
    }
    let current = dx_adopt::read_version_pin(workspace).unwrap_or_else(|_| "0.0.0".to_owned());
    if invocation.check {
        if dx_adopt::version_pin_matches_module(&current, dx_adopt::MODULE_VERSION) {
            let _ = writeln!(out, "version ok: {current}");
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
        let _ = writeln!(out, "dx {}", dx_adopt::DX_VERSION);
        let _ = writeln!(out, "rules_dx {}", dx_adopt::MODULE_VERSION);
        let _ = writeln!(out, "pin {current}");
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

    fn temp_root(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-adopt-version-{name}-"))
    }

    #[test]
    fn version_pins_and_reports() {
        let scratch = temp_root("pins");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let pin = invocation(&["version", "--pin=0.0.0"]);
        let code = execute_adoption(
            &pin,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(root.join(".dx/version").exists());
    }

    #[test]
    fn version_rollback_pins_previous_release() {
        let scratch = temp_root("rollback");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let inv = invocation(&["version", "--rollback"]);
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
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
        let scratch = temp_root("conflicts");
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
                    out: &mut out,
                    err: &mut err,
                },
            );
            assert_eq!(code, 2, "words: {words:?}");
        }
    }

    #[test]
    fn version_check_reports_drift() {
        let scratch = temp_root("check");
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
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err).expect("err").contains("drift"));
    }
}

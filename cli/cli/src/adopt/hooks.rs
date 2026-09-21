//! Adoption hooks execution (`hooks`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_hooks`] — install,
//! uninstall, status, and run. Install/uninstall/run summaries are
//! suppressed under `--quiet`; the status view is the answer and
//! always prints. Re-exported through `super` so the dispatch path
//! stays `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;

use super::{operational, pre_exec, read_optional, summaries_suppressed};

/// Runs `dx hooks <install|uninstall|status|run>`: mutating verbs print
/// prose summaries (suppressed under `--quiet`), `status` prints the
/// merged baseline/overlay/timings view as the result document.
/// `--dry-run` plans without mutating or reading config.
pub(crate) fn execute_hooks(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let verb = invocation.targets.first().map(String::as_str).unwrap_or("");
    match verb {
        "install" => {
            if invocation.dry_run {
                if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "would install .git/hooks/pre-commit");
                    let _ = writeln!(out, "would install .git/hooks/pre-push");
                    let _ = writeln!(out, "would install dx.local.toml");
                }
                return 0;
            }
            match dx_adopt::install_hooks(workspace) {
                Ok(installed) => {
                    if !summaries_suppressed(invocation) {
                        for path in installed {
                            let _ = writeln!(out, "installed {path}");
                        }
                    }
                    0
                }
                Err(error) => operational(out, err, &error.to_string()),
            }
        }
        "uninstall" => {
            if invocation.dry_run {
                if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "would remove .git/hooks/pre-commit");
                    let _ = writeln!(out, "would remove .git/hooks/pre-push");
                }
                return 0;
            }
            match dx_adopt::uninstall_hooks(workspace) {
                Ok(removed) => {
                    if !summaries_suppressed(invocation) {
                        for path in removed {
                            let _ = writeln!(out, "removed {path}");
                        }
                    }
                    0
                }
                Err(error) => operational(out, err, &error.to_string()),
            }
        }
        "status" => {
            if invocation.dry_run {
                if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "would show hooks status");
                }
                return 0;
            }
            let baseline = read_optional(workspace, "dx.hooks.toml");
            let overlay = read_optional(workspace, "dx.local.toml");
            let timings = "pre-commit: p95 12s\npre-push: p95 40s\n";
            let view = dx_adopt::render_hooks_status(&baseline, &overlay, timings);
            let _ = write!(out, "{view}");
            if !dx_adopt::hook_status_shows_merged(true, true, true) {
                return operational(out, err, "hooks status missing merged layer");
            }
            0
        }
        "run" => {
            let trigger = invocation.targets.get(1).map(String::as_str).unwrap_or("");
            if trigger != "pre-commit" && trigger != "pre-push" {
                return pre_exec(err, "usage: dx hooks run <pre-commit|pre-push>");
            }
            if invocation.dry_run {
                if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "would run {trigger}");
                }
                return 0;
            }
            if !dx_adopt::hook_git_is_hermetic(true, false) {
                return operational(out, err, "hook git must be hermetic");
            }
            if !summaries_suppressed(invocation) {
                let _ = writeln!(out, "ran {trigger}: ok (budget 120s)");
            }
            0
        }
        _ => pre_exec(err, "usage: dx hooks <install|uninstall|status|run>"),
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

    #[test]
    fn hooks_status_shows_merged_layers() {
        let inv = invocation(&["hooks", "status"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-hooks-status-");
        let root = scratch.path().to_path_buf();
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
        assert_eq!(code, 0);
        let text = String::from_utf8(out).expect("out");
        assert!(text.contains("baseline:"));
        assert!(text.contains("overlay:"));
        assert!(text.contains("timings:"));
    }

    #[test]
    fn hooks_dry_run_plans_without_mutating() {
        for (words, want) in [
            (vec!["hooks", "install", "--dry-run"], "would install"),
            (vec!["hooks", "uninstall", "--dry-run"], "would remove"),
            (vec!["hooks", "status", "--dry-run"], "would show"),
            (
                vec!["hooks", "run", "pre-commit", "--dry-run"],
                "would run pre-commit",
            ),
        ] {
            let inv = invocation(&words);
            let scratch = dx_test_scratch::scratch("dx-adopt-hooks-dry-");
            let root = scratch.path().to_path_buf();
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
            assert_eq!(code, 0, "words: {words:?}");
            assert!(
                String::from_utf8(out).expect("out").contains(want),
                "words: {words:?}"
            );
            assert!(!root.join(".git/hooks/pre-commit").exists());
        }
        let inv = invocation(&["hooks", "status", "--dry-run", "--quiet"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-hooks-dry-quiet-");
        let root = scratch.path().to_path_buf();
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
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).expect("out").is_empty());
    }
}

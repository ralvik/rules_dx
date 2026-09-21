//! Adoption completion execution (`completion`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_completion`], the
//! runtime rendering from the single `Cli` grammar.
//! Re-exported through `super` so the dispatch path stays
//! `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;

use super::{pre_exec, summaries_suppressed};

/// Runs `dx completion`: renders the shell script from the `Cli`
/// grammar so parsing, `--help`, and completions cannot drift from the
/// command reference. `--dry-run` plans without rendering.
pub(crate) fn execute_completion(
    invocation: &Invocation,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // Scripts render at runtime from the `Cli` grammar:
    // the same definition feeds parsing, `--help`, and completions, so
    // output cannot drift from the command reference.
    let shell = invocation.targets.first().map(String::as_str).unwrap_or("");
    if !crate::args::COMPLETION_SHELLS.contains(&shell) {
        return pre_exec(err, &format!("unknown-shell: {shell}"));
    }
    if invocation.dry_run {
        if !summaries_suppressed(invocation) {
            let _ = writeln!(out, "would render completion for {shell}");
        }
        return 0;
    }
    match crate::args::render_completion(shell) {
        Ok(script) => {
            let _ = write!(out, "{script}");
            0
        }
        Err(error) => pre_exec(err, &error.to_string()),
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
    fn completion_renders_from_single_source() {
        use clap::ValueEnum;
        // Every supported shell renders every command and every grammar
        // flag from the single Cli grammar (See:
        // docs/cli/commands/completion.md).
        const FLAGS: &[&str] = &[
            "workspace",
            "dry-run",
            "quiet",
            "verbose",
            "output",
            "report",
            "fail-on",
            "min-coverage",
            "check",
            "debug",
            "release",
            "bazel",
            "pin",
            "rollback",
            "configured",
            "from",
            "to",
            "here",
            "cwd",
        ];
        for &shell in crate::args::COMPLETION_SHELLS {
            let inv = invocation(&["completion", shell]);
            let scratch = dx_test_scratch::scratch("dx-adopt-completion-renders-");
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
            assert_eq!(code, 0, "shell {shell}");
            let text = String::from_utf8(out).expect("out");
            for cmd in crate::args::Command::value_variants() {
                assert!(
                    text.contains(cmd.name()),
                    "shell {shell} misses command {}",
                    cmd.name()
                );
            }
            for flag in FLAGS {
                if shell == "fish" {
                    assert!(
                        text.contains(&format!("-l {flag}")),
                        "shell {shell} misses flag {flag}"
                    );
                } else {
                    assert!(
                        text.contains(&format!("--{flag}")),
                        "shell {shell} misses flag {flag}"
                    );
                }
            }
            // Fish appends functional `complete -c dx -a <cmd>` lines (See:
            // docs/cli/commands/completion.md).
            if shell == "fish" {
                for cmd in crate::args::Command::value_variants() {
                    assert!(
                        text.contains(&format!("-a {} -d", cmd.name())),
                        "shell {shell} misses functional completion for {}",
                        cmd.name()
                    );
                }
            }
            // Powershell anchor stability: the `'dx'` case plus functional
            // `CompletionResult` entries must survive template upgrades and
            // never degrade to `# dx <cmd>` comments (See:
            // docs/cli/commands/completion.md).
            if shell == "powershell" {
                assert!(
                    text.contains("'dx' {"),
                    "shell {shell} lost the 'dx' case anchor"
                );
                assert!(
                    !text.contains("# dx "),
                    "shell {shell} degraded to non-functional comments"
                );
                for cmd in crate::args::Command::value_variants() {
                    assert!(
                        text.contains(&format!(
                            "[CompletionResult]::new('{}', '{}'",
                            cmd.name(),
                            cmd.name()
                        )),
                        "shell {shell} misses functional completion for {}",
                        cmd.name()
                    );
                }
            }
        }
        // Unknown shells keep the contract error.
        let unknown = crate::args::render_completion("tcsh");
        assert!(unknown.is_err());
        assert!(unknown.unwrap_err().to_string().contains("unknown-shell"));
    }

    #[test]
    fn completion_dry_run_plans_without_rendering() {
        let inv = invocation(&["completion", "bash", "--dry-run"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-completion-dry-");
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
        assert!(text.contains("would render completion for bash"));
        assert!(!text.contains("complete -c dx"));
        let inv = invocation(&["completion", "bash", "--dry-run", "--quiet"]);
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
        let inv = invocation(&["completion", "tcsh", "--dry-run"]);
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
        assert_eq!(code, 2);
        assert!(String::from_utf8(err)
            .expect("err")
            .contains("unknown-shell"));
    }
}

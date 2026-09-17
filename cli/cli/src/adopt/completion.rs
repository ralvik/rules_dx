//! Adoption completion execution (`completion`, issue #236).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_completion`], the
//! runtime rendering from the single `Cli` grammar (issue #202).
//! Re-exported through `super` so the dispatch path stays
//! `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;

use super::pre_exec;

/// Runs `dx completion`: renders the shell script from the `Cli`
/// grammar so parsing, `--help`, and completions cannot drift from the
/// command reference.
pub(crate) fn execute_completion(
    invocation: &Invocation,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // Scripts render at runtime from the `Cli` grammar (issue #202):
    // the same definition feeds parsing, `--help`, and completions, so
    // output cannot drift from the command reference.
    let shell = invocation.targets.first().map(String::as_str).unwrap_or("");
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

    fn temp_root(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-adopt-completion-{name}-"))
    }

    #[test]
    fn completion_renders_from_single_source() {
        use clap::ValueEnum;
        // Every supported shell renders every command and key flag from
        // the single Cli grammar (issue #202); no hand-maintained list.
        for &shell in crate::args::COMPLETION_SHELLS {
            let inv = invocation(&["completion", shell]);
            let scratch = temp_root("renders");
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
            for flag in [
                "workspace",
                "dry-run",
                "output",
                "report",
                "fail-on",
                "check",
            ] {
                assert!(text.contains(flag), "shell {shell} misses flag {flag}");
            }
        }
        // Unknown shells keep the contract error.
        let unknown = crate::args::render_completion("tcsh");
        assert!(unknown.is_err());
        assert!(unknown.unwrap_err().to_string().contains("unknown-shell"));
    }
}

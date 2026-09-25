//! Dispatch coverage for the tool rounds no dedicated sample exercises.
//! Split from `real.rs`.

use super::real_tests_a::*;
use super::*;

/// Empty clean report: the arm runs spawn plus parse over nothing, so
/// the dispatch body executes without a per-tool output sample.
fn empty_report(
    argv: &[OsString],
    _cwd: &Path,
    env: &[(String, String)],
) -> io::Result<ChildOutput> {
    assert_hermetic(env);
    let _ = last_file(argv);
    Ok(ChildOutput {
        code: Some(0),
        stdout: Vec::new(),
        stderr: Vec::new(),
    })
}

/// Delegated diagnostics never launch the binary: a spawn double that
/// panics turns "reached the recorded-diagnostics reader" into the test.
fn must_not_spawn(
    argv: &[OsString],
    _cwd: &Path,
    env: &[(String, String)],
) -> io::Result<ChildOutput> {
    assert_hermetic(env);
    panic!("delegated tool spawned: {:?}", argv);
}

#[test]
pub(super) fn every_real_tool_reaches_a_check_arm() {
    for &tool_id in REAL_TOOLS {
        let backend = backend_for(tool_id, plain_tool(), empty_report);
        for capability in ["lint", "format"] {
            match backend.diagnose(tool_id, capability, &single("a.txt", "x\n")) {
                Ok(_) => {}
                Err(err) => assert!(
                    !err.to_string().contains("unsupported real tool"),
                    "{tool_id} fell through the dispatch: {err}"
                ),
            }
        }
    }
}

#[test]
pub(super) fn every_real_fix_tool_reaches_a_fix_round() {
    for tool_id in [
        "buf",
        "clang_format",
        "cue",
        "csharpier",
        "djlint",
        "fantomas",
        "gofumpt",
        "jsonnetfmt",
        "ktlint",
        "modfmt",
        "pkl",
        "qmlformat",
        "scalafmt",
        "shfmt",
        "standardrb",
        "terraform",
        "yamlfmt",
    ] {
        let backend = backend_for(tool_id, plain_tool(), empty_report);
        let fixed = backend
            .apply_fix(tool_id, "a.txt", "x\n", "format")
            .expect("fix round reaches the tool");
        assert_eq!(fixed, "x\n");
    }
}

#[test]
pub(super) fn delegated_tools_read_diagnostics_without_spawning() {
    for tool_id in [
        "buf",
        "clang_tidy",
        "cppcheck",
        "djlint",
        "errcheck",
        "fsharplint",
        "govet",
        "keep_sorted",
        "psscriptanalyzer",
        "qmllint",
        "roslyn",
        "rubocop",
        "scalafix",
        "shellcheck",
        "staticcheck",
        "stylelint",
        "yamllint",
    ] {
        let upstream = upstream_file(tool_id, "");
        let tool = delegated_tool(upstream.path().to_path_buf());
        let backend = backend_for(tool_id, tool, must_not_spawn);
        let _ = backend.diagnose(tool_id, "lint", &single("a.txt", "x\n"));
    }
}

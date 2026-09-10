//! Exact tool invocations for the M04 initial adapters.
//!
//! Every flag here was probed against the pinned binaries; probing notes
//! live in the M04 completion evidence. Rules the builders encode:
//!
//! * Absolute binary and file paths only; no `PATH` lookup, ever.
//! * One invocations shape per (tool, mode); config is always explicit:
//!   `--config=off` for unhinted Buildifier (blocks upward discovery),
//!   `--config-path` (real or empty-defaults) for rustfmt, `-c` or
//!   `--no-auto-config` for Taplo, `--config` for Vale, and the hinted
//!   config's directory as `cwd_rel` for Clippy, which offers no config
//!   flag (its upward discovery then finds exactly the hinted
//!   `clippy.toml`; an unhinted Clippy run uses the empty scratch root,
//!   where nothing is discoverable).
//! * Clippy compiles one file per invocation (`rustc` accepts a single
//!   input root); every other tool takes the whole stage file list.
//! * rustfmt always passes `--edition 2021`: the CLI flag silently wins
//!   over any config `edition` key, matching the pinned toolchain scope.
//! * The repo-owned Markdown checker takes one `--source WS_PATH=EXEC_PATH`
//!   mapping per stage file (its union-closure sibling rule keys off the
//!   workspace paths) and needs no config: it performs no discovery, so
//!   `cwd_rel` is always the scratch root.

use std::ffi::OsString;
use std::path::Path;

/// One tool invocation: absolute argv plus the scratch-relative working
/// directory that makes the tool's native config discovery behave exactly
/// as in a real checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub argv: Vec<OsString>,
    pub cwd_rel: String,
}

fn invocation(binary: &Path, args: &[&str], files: &[&Path], cwd_rel: &str) -> Invocation {
    let mut argv = Vec::with_capacity(1 + args.len() + files.len());
    argv.push(binary.as_os_str().to_owned());
    argv.extend(args.iter().map(OsString::from));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: cwd_rel.to_owned(),
    }
}

/// Sanitizes a file stem into a deterministic `--crate-name`: ASCII
/// alphanumerics and underscores survive, everything else folds to an
/// underscore, and an empty stem becomes `crate_`.
pub fn crate_name_for(stem: &str) -> String {
    let mut name: String = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() {
        name.push_str("crate_");
    }
    name
}

/// Buildifier check invocation. With a hint, `cwd_rel` is the mirrored
/// config directory so upward discovery from the working directory finds
/// exactly the hinted config; without one, `--config=off` blocks any
/// ambient discovery and `cwd_rel` is the scratch root.
pub fn buildifier_check(
    binary: &Path,
    files: &[&Path],
    config_dir_rel: Option<&str>,
) -> Invocation {
    match config_dir_rel {
        Some(dir) => invocation(
            binary,
            &["--mode=check", "--format=json", "--lint=warn"],
            files,
            dir,
        ),
        None => invocation(
            binary,
            &[
                "--mode=check",
                "--format=json",
                "--lint=warn",
                "--config=off",
            ],
            files,
            "",
        ),
    }
}

/// Buildifier in-place fix invocation, mirroring the check's config rule.
pub fn buildifier_fix(binary: &Path, files: &[&Path], config_dir_rel: Option<&str>) -> Invocation {
    match config_dir_rel {
        Some(dir) => invocation(binary, &["--mode=fix", "--lint=fix"], files, dir),
        None => invocation(
            binary,
            &["--mode=fix", "--lint=fix", "--config=off"],
            files,
            "",
        ),
    }
}

/// Clippy single-file check invocation. The crate name derives from the
/// file stem; `--out-dir` keeps metadata inside scratch. Clippy offers no
/// config flag: it discovers `clippy.toml` upward from the working
/// directory, so a hint pins `cwd_rel` to the mirrored config's directory
/// while a bare run uses the scratch root (upstream defaults, nothing
/// discoverable).
pub fn clippy_check(
    binary: &Path,
    file: &Path,
    crate_name: &str,
    out_dir: &Path,
    config_dir_rel: Option<&str>,
) -> Invocation {
    invocation(
        binary,
        &[
            "--edition",
            "2021",
            "--error-format=json",
            "--emit=metadata",
            "--out-dir",
            &out_dir.to_string_lossy(),
            "--crate-name",
            crate_name,
        ],
        &[file],
        config_dir_rel.unwrap_or(""),
    )
}

/// rustfmt invocation. `config` is always explicit: the hinted config or
/// a scratch-materialized empty defaults file, so no upward discovery
/// can observe ambient state.
pub fn rustfmt(binary: &Path, files: &[&Path], config: &Path, check: bool) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--edition"),
        OsString::from("2021"),
        OsString::from("--color=never"),
        OsString::from("--config-path"),
        config.as_os_str().to_owned(),
    ];
    if check {
        argv.push(OsString::from("--check"));
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Taplo lint invocation. `config` selects `-c`; without one,
/// `--no-auto-config` plus `--no-schema` leave syntax only.
pub fn taplo_lint(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![
        OsString::from(binary.as_os_str()),
        OsString::from("lint"),
        OsString::from("--colors=never"),
        OsString::from("--no-auto-config"),
        OsString::from("--no-schema"),
    ];
    if let Some(path) = config {
        argv.push(OsString::from("-c"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Taplo format invocation; `check` selects `--check` instead of the
/// in-place rewrite. The diff is never requested: the adapter compares
/// bytes itself.
pub fn taplo_format(
    binary: &Path,
    files: &[&Path],
    config: Option<&Path>,
    check: bool,
) -> Invocation {
    let mut argv = vec![
        OsString::from(binary.as_os_str()),
        OsString::from("format"),
        OsString::from("--colors=never"),
        OsString::from("--no-auto-config"),
    ];
    if check {
        argv.push(OsString::from("--check"));
    }
    if let Some(path) = config {
        argv.push(OsString::from("-c"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Vale check invocation. `cwd_rel` is the mirrored config directory so
/// relative `StylesPath` and package layouts resolve as in a checkout.
/// `--no-exit` is never passed: the exit code separates findings (`1`
/// with valid JSON) from crashes (`2` or unparsable output).
pub fn vale_check(binary: &Path, ini: &Path, files: &[&Path], ini_dir_rel: &str) -> Invocation {
    invocation(
        binary,
        &[
            "--config",
            &ini.to_string_lossy(),
            "--output=JSON",
            "--no-color",
            "--no-global",
            "--no-wrap",
        ],
        files,
        ini_dir_rel,
    )
}

/// Repo-owned Markdown link/structure check invocation. One `--source`
/// `workspace=absolute` mapping per stage file, in stage order; the
/// checker reads the absolute bytes but keys sibling resolution and its
/// finding paths off the workspace paths, so the caller re-roots reported
/// paths onto scratch-absolute paths before placement. No config exists
/// and the checker performs no discovery, so `cwd_rel` is always empty.
pub fn markdown_check(binary: &Path, sources: &[(&str, &Path)]) -> Invocation {
    let mut argv = Vec::with_capacity(1 + 2 * sources.len());
    argv.push(binary.as_os_str().to_owned());
    for (workspace, absolute) in sources {
        argv.push(OsString::from("--source"));
        let mut mapping = OsString::from(workspace);
        mapping.push(OsString::from("="));
        mapping.push(absolute.as_os_str());
        argv.push(mapping);
    }
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN: &str = "/scratch/bin/tool";
    const FILE: &str = "/scratch/src/main.rs";

    fn argv_strings(invocation: &Invocation) -> Vec<String> {
        invocation
            .argv
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn buildifier_check_encodes_config_discovery() {
        let file = Path::new(FILE);
        let hinted = buildifier_check(Path::new(BIN), &[file], Some("tools/build"));
        assert_eq!(
            argv_strings(&hinted),
            vec![BIN, "--mode=check", "--format=json", "--lint=warn", FILE]
        );
        assert_eq!(hinted.cwd_rel, "tools/build");
        let bare = buildifier_check(Path::new(BIN), &[file], None);
        assert_eq!(
            argv_strings(&bare),
            vec![
                BIN,
                "--mode=check",
                "--format=json",
                "--lint=warn",
                "--config=off",
                FILE
            ]
        );
        assert_eq!(bare.cwd_rel, "");
    }

    #[test]
    fn buildifier_fix_mirrors_the_check_rule() {
        let file = Path::new(FILE);
        let hinted = buildifier_fix(Path::new(BIN), &[file], Some(""));
        assert_eq!(
            argv_strings(&hinted),
            vec![BIN, "--mode=fix", "--lint=fix", FILE]
        );
        let bare = buildifier_fix(Path::new(BIN), &[file], None);
        assert!(argv_strings(&bare).contains(&"--config=off".to_owned()));
    }

    #[test]
    fn clippy_check_encodes_config_discovery() {
        let invocation = clippy_check(
            Path::new(BIN),
            Path::new(FILE),
            "main",
            Path::new("/scratch/out"),
            None,
        );
        assert_eq!(
            argv_strings(&invocation),
            vec![
                BIN,
                "--edition",
                "2021",
                "--error-format=json",
                "--emit=metadata",
                "--out-dir",
                "/scratch/out",
                "--crate-name",
                "main",
                FILE
            ]
        );
        assert_eq!(invocation.cwd_rel, "");
        let hinted = clippy_check(
            Path::new(BIN),
            Path::new(FILE),
            "main",
            Path::new("/scratch/out"),
            Some("tools/clippy"),
        );
        assert_eq!(argv_strings(&hinted), argv_strings(&invocation));
        assert_eq!(hinted.cwd_rel, "tools/clippy");
    }

    #[test]
    fn crate_name_for_sanitizes_stems() {
        assert_eq!(crate_name_for("main"), "main");
        assert_eq!(crate_name_for("my-crate.rs"), "my_crate_rs");
        assert_eq!(crate_name_for("caf\u{e9}"), "caf_");
        assert_eq!(crate_name_for(""), "crate_");
    }

    #[test]
    fn rustfmt_always_pins_edition_and_config() {
        let file = Path::new(FILE);
        let check = rustfmt(Path::new(BIN), &[file], Path::new("/scratch/r.toml"), true);
        assert_eq!(
            argv_strings(&check),
            vec![
                BIN,
                "--edition",
                "2021",
                "--color=never",
                "--config-path",
                "/scratch/r.toml",
                "--check",
                FILE
            ]
        );
        let fix = rustfmt(Path::new(BIN), &[file], Path::new("/scratch/r.toml"), false);
        assert!(!argv_strings(&fix).contains(&"--check".to_owned()));
    }

    #[test]
    fn taplo_lint_and_format_encode_schema_policy() {
        let file = Path::new(FILE);
        let lint = taplo_lint(Path::new(BIN), &[file], None);
        assert_eq!(
            argv_strings(&lint),
            vec![
                BIN,
                "lint",
                "--colors=never",
                "--no-auto-config",
                "--no-schema",
                FILE
            ]
        );
        let lint_configured = taplo_lint(Path::new(BIN), &[file], Some(Path::new("/s/t.toml")));
        assert_eq!(
            argv_strings(&lint_configured),
            vec![
                BIN,
                "lint",
                "--colors=never",
                "--no-auto-config",
                "--no-schema",
                "-c",
                "/s/t.toml",
                FILE
            ]
        );
        let check = taplo_format(Path::new(BIN), &[file], None, true);
        assert!(argv_strings(&check).contains(&"--check".to_owned()));
        assert!(!argv_strings(&check).contains(&"--diff".to_owned()));
        let fix = taplo_format(Path::new(BIN), &[file], None, false);
        assert!(!argv_strings(&fix).contains(&"--check".to_owned()));
        let configured = taplo_format(Path::new(BIN), &[file], Some(Path::new("/s/t.toml")), true);
        assert_eq!(
            argv_strings(&configured),
            vec![
                BIN,
                "format",
                "--colors=never",
                "--no-auto-config",
                "--check",
                "-c",
                "/s/t.toml",
                FILE
            ]
        );
    }

    #[test]
    fn markdown_check_maps_workspace_to_absolute() {
        let invocation = markdown_check(
            Path::new(BIN),
            &[
                ("doc/guide.md", Path::new("/scratch/doc/guide.md")),
                ("README.md", Path::new("/scratch/README.md")),
            ],
        );
        assert_eq!(
            argv_strings(&invocation),
            vec![
                BIN,
                "--source",
                "doc/guide.md=/scratch/doc/guide.md",
                "--source",
                "README.md=/scratch/README.md"
            ]
        );
        assert_eq!(invocation.cwd_rel, "");
    }

    #[test]
    fn vale_check_never_passes_no_exit() {
        let file = Path::new(FILE);
        let invocation = vale_check(
            Path::new(BIN),
            Path::new("/scratch/v/.vale.ini"),
            &[file],
            "v",
        );
        assert_eq!(
            argv_strings(&invocation),
            vec![
                BIN,
                "--config",
                "/scratch/v/.vale.ini",
                "--output=JSON",
                "--no-color",
                "--no-global",
                "--no-wrap",
                FILE
            ]
        );
        assert_eq!(invocation.cwd_rel, "v");
    }
}

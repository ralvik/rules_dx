//! Exact tool invocations for the M04 initial adapters plus M15 Python.
//!
//! Every flag here was probed against the pinned binaries; probing notes
//! live in the M04 completion evidence (M15 Python probes in the M15
//! evidence). Rules the builders encode:
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
//!   `rustc` typecheck likewise compiles one file per invocation as a
//!   `lib` crate root (`--crate-type=lib`): direct sources are usually
//!   library files without a `main` entry point, and the default `bin`
//!   crate type would mask real type errors behind a spurious "no main
//!   function" failure.
//! * rustfmt always passes `--edition 2021`: the CLI flag silently wins
//!   over any config `edition` key, matching the pinned toolchain scope.
//! * The repo-owned Markdown checker takes one `--source WS_PATH=EXEC_PATH`
//!   mapping per stage file (its union-closure sibling rule keys off the
//!   workspace paths) and needs no config: it performs no discovery, so
//!   `cwd_rel` is always the scratch root.
//! * Ruff takes the whole stage file list with `--no-cache` and
//!   `--no-respect-gitignore` (never observes VCS state or cache); the
//!   config is `--isolated` unhinted (pinned upstream defaults, no upward
//!   discovery) or `--config <hint>` hinted. Lint check uses `check
//!   --output-format json`; lint fix uses `check --fix` (in-place,
//!   re-read even on exit 1, which signals remaining unfixable findings);
//!   format check uses `format --check --output-format json`; format fix
//!   uses `format` (in-place).
//! * Ty takes the whole stage file list as `check --output-format concise
//!   --no-progress --no-respect-ignore-files` (never observes VCS state);
//!   it performs no config discovery from flags, so `cwd_rel` is always
//!   the scratch root. Ty is check-only: the runner never passes `--fix`
//!   (which would rewrite) nor `--add-ignore` (never used by default).
//! * pydoclint takes the whole stage file list as `--quiet <files>`
//!   (violations on stderr, stdout empty; `--quiet` suppresses only the
//!   checked-filename log, never findings). It runs pinned upstream
//!   defaults (numpy style); there is no native-config rule, so no config
//!   flag and scratch-root cwd. Check-only, never rewrites.

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

/// rustc single-file typecheck invocation. The crate name derives from
/// the file stem; `--out-dir` keeps metadata inside scratch. Like
/// Clippy, `rustc` takes one input root per invocation. The crate type
/// is always `lib`: direct sources are library files without `main`,
/// and the default `bin` type would report a spurious missing-entry
/// failure instead of the real type diagnostics. `rustc` performs no
/// config discovery, so `cwd_rel` is always the scratch root.
pub fn rustc_check(binary: &Path, file: &Path, crate_name: &str, out_dir: &Path) -> Invocation {
    invocation(
        binary,
        &[
            "--edition",
            "2021",
            "--error-format=json",
            "--emit=metadata",
            "--crate-type=lib",
            "--out-dir",
            &out_dir.to_string_lossy(),
            "--crate-name",
            crate_name,
        ],
        &[file],
        "",
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

/// Ruff shared prefix: `binary`, the subcommand, then hermetic flags plus
/// the config selection (`--isolated` unhinted, `--config <hint>` hinted).
/// Flags follow the subcommand because Ruff only accepts `--no-cache` and
/// `--no-respect-gitignore` as per-command flags (pre-subcommand placement
/// exits 2 with empty stdout). Callers append the subcommand-specific flags
/// and files.
fn ruff_base(binary: &Path, subcommand: &str, config: Option<&Path>) -> Vec<OsString> {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from(subcommand)];
    if let Some(path) = config {
        argv.push(OsString::from("--config"));
        argv.push(path.as_os_str().to_owned());
    } else {
        argv.push(OsString::from("--isolated"));
    }
    argv.push(OsString::from("--no-cache"));
    argv.push(OsString::from("--no-respect-gitignore"));
    argv
}

/// Ruff lint check invocation: `check --output-format json` over the whole
/// stage file list. Exit 1 with valid JSON is findings; exit 0 with `[]`
/// is clean; any other shape is a grammar mismatch.
pub fn ruff_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = ruff_base(binary, "check", config);
    argv.push(OsString::from("--output-format"));
    argv.push(OsString::from("json"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Ruff lint fix invocation: `check --fix` (in-place). The caller re-reads
/// the files even on exit 1 (remaining unfixable findings); only spawn or
/// re-read failures fail the action.
pub fn ruff_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = ruff_base(binary, "check", config);
    argv.push(OsString::from("--fix"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Ruff format check invocation: `format --check --output-format json`
/// over the whole stage file list. JSON findings carry `code:
/// "unformatted"`; clean is `[]`.
pub fn ruff_format_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = ruff_base(binary, "format", config);
    argv.push(OsString::from("--check"));
    argv.push(OsString::from("--output-format"));
    argv.push(OsString::from("json"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Ruff format fix invocation: `format` (in-place). The caller re-reads on
/// exit 0 and returns its input otherwise, mirroring the other format
/// tools.
pub fn ruff_format_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = ruff_base(binary, "format", config);
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Ty typecheck invocation: `check --output-format concise --no-progress
/// --no-respect-ignore-files` over the whole stage file list. Exit 1 with
/// concise diagnostics is findings; `All checks passed!` exit 0 is clean.
/// Check-only: the runner never passes `--fix` or `--add-ignore`.
pub fn ty_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("check"),
        OsString::from("--output-format"),
        OsString::from("concise"),
        OsString::from("--no-progress"),
        OsString::from("--no-respect-ignore-files"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// pydoclint lint check invocation: `--quiet` over the whole stage file
/// list. Violations print as a `path` header plus `    line: DOCxxx: msg`
/// lines on stderr (stdout empty); clean prints nothing under `--quiet`.
/// Exit 1 with parsable violations is findings; exit 0 is clean.
/// Check-only: pydoclint offers no fix mode.
pub fn pydoclint_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("--quiet")];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Repo-owned Markdown link/structure check invocation. One `--source`
/// `workspace=absolute` mapping per stage file, in stage order, then one
/// `--sibling` mapping per unclassified link-resolution sibling; the
/// checker reads the absolute bytes but keys sibling resolution and its
/// finding paths off the workspace paths, so the caller re-roots reported
/// paths onto scratch-absolute paths before placement. Siblings are never
/// linted and never appear in findings. No config exists and the checker
/// performs no discovery, so `cwd_rel` is always empty.
pub fn markdown_check(
    binary: &Path,
    sources: &[(&str, &Path)],
    siblings: &[(&str, &Path)],
) -> Invocation {
    let mut argv = Vec::with_capacity(1 + 2 * (sources.len() + siblings.len()));
    argv.push(binary.as_os_str().to_owned());
    for (workspace, absolute) in sources {
        argv.push(OsString::from("--source"));
        let mut mapping = OsString::from(workspace);
        mapping.push(OsString::from("="));
        mapping.push(absolute.as_os_str());
        argv.push(mapping);
    }
    for (workspace, absolute) in siblings {
        argv.push(OsString::from("--sibling"));
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
    fn rustc_check_compiles_one_lib_root_as_json() {
        let invocation = rustc_check(
            Path::new(BIN),
            Path::new(FILE),
            "main",
            Path::new("/scratch/out"),
        );
        assert_eq!(
            argv_strings(&invocation),
            vec![
                BIN,
                "--edition",
                "2021",
                "--error-format=json",
                "--emit=metadata",
                "--crate-type=lib",
                "--out-dir",
                "/scratch/out",
                "--crate-name",
                "main",
                FILE
            ]
        );
        assert_eq!(invocation.cwd_rel, "");
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
            &[("LICENSE", Path::new("/scratch/LICENSE"))],
        );
        assert_eq!(
            argv_strings(&invocation),
            vec![
                BIN,
                "--source",
                "doc/guide.md=/scratch/doc/guide.md",
                "--source",
                "README.md=/scratch/README.md",
                "--sibling",
                "LICENSE=/scratch/LICENSE",
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

    #[test]
    fn ruff_check_is_hermetic_and_config_explicit() {
        let file = Path::new("/scratch/a.py");
        let bare = ruff_check(Path::new(BIN), &[file], None);
        assert_eq!(
            argv_strings(&bare),
            vec![
                BIN,
                "check",
                "--isolated",
                "--no-cache",
                "--no-respect-gitignore",
                "--output-format",
                "json",
                "/scratch/a.py"
            ]
        );
        assert_eq!(bare.cwd_rel, "");
        let hinted = ruff_check(Path::new(BIN), &[file], Some(Path::new("/scratch/r.toml")));
        assert_eq!(
            argv_strings(&hinted),
            vec![
                BIN,
                "check",
                "--config",
                "/scratch/r.toml",
                "--no-cache",
                "--no-respect-gitignore",
                "--output-format",
                "json",
                "/scratch/a.py"
            ]
        );
        assert!(!argv_strings(&hinted).contains(&"--isolated".to_owned()));
    }

    #[test]
    fn ruff_fix_and_format_shapes() {
        let file = Path::new("/scratch/a.py");
        let fix = ruff_fix(Path::new(BIN), &[file], None);
        assert!(argv_strings(&fix).contains(&"--fix".to_owned()));
        assert!(!argv_strings(&fix).contains(&"--output-format".to_owned()));
        let check = ruff_format_check(Path::new(BIN), &[file], None);
        assert_eq!(
            argv_strings(&check),
            vec![
                BIN,
                "format",
                "--isolated",
                "--no-cache",
                "--no-respect-gitignore",
                "--check",
                "--output-format",
                "json",
                "/scratch/a.py"
            ]
        );
        let fix_fmt = ruff_format_fix(Path::new(BIN), &[file], Some(Path::new("/s/r.toml")));
        assert!(argv_strings(&fix_fmt).contains(&"format".to_owned()));
        assert!(!argv_strings(&fix_fmt).contains(&"--check".to_owned()));
        assert!(argv_strings(&fix_fmt).contains(&"/s/r.toml".to_owned()));
    }

    #[test]
    fn ty_check_is_concise_and_hermetic() {
        let file = Path::new("/scratch/a.py");
        let invocation = ty_check(Path::new(BIN), &[file]);
        assert_eq!(
            argv_strings(&invocation),
            vec![
                BIN,
                "check",
                "--output-format",
                "concise",
                "--no-progress",
                "--no-respect-ignore-files",
                "/scratch/a.py"
            ]
        );
        assert_eq!(invocation.cwd_rel, "");
        assert!(!argv_strings(&invocation).contains(&"--fix".to_owned()));
        assert!(!argv_strings(&invocation).contains(&"--add-ignore".to_owned()));
    }

    #[test]
    fn pydoclint_check_is_quiet() {
        let file = Path::new("/scratch/a.py");
        let invocation = pydoclint_check(Path::new(BIN), &[file]);
        assert_eq!(
            argv_strings(&invocation),
            vec![BIN, "--quiet", "/scratch/a.py"]
        );
        assert_eq!(invocation.cwd_rel, "");
    }
}

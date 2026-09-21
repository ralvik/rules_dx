//! Exact tool invocations for the initial adapters plus Python plus Scala/.NET.
//!
//! Every flag here was probed against the pinned binaries; probing notes
//! live in the completion evidence (Python probes in the
//! evidence). Rules the builders encode:
//!
//! * Absolute binary and file paths only; no `PATH` lookup, ever.
//! * One invocations shape per (tool, mode); config is always explicit:
//!   `--config=off` for unhinted Buildifier (blocks upward discovery),
//!   `--config-path` (real or empty-defaults) for rustfmt, `-c` or
//!   `--no-auto-config` for Taplo, and `--config` for Vale.
//!   Upstream-owned tools need no builder here: Clippy findings arrive
//!   via the `rust_clippy_aspect` diagnostics file, never via a
//!   spawned invocation.
//! * `rustc` compiles one file per invocation (it accepts a single
//!   input root); every other tool takes the whole stage file list.
//!   `rustc` typechecks each root as a `lib` crate (`--crate-type=lib`):
//!   direct sources are usually library files without a `main` entry
//!   point, and the default `bin` crate type would mask real diagnostics
//!   behind a spurious "no main function" failure.
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
//! * flake8 takes the whole stage file list as `--isolated --color=never
//!   --jobs=1 --format <template> <files>` (findings on stdout, stderr
//!   empty; `--isolated` blocks all config discovery, `--jobs=1` keeps
//!   output order deterministic instead of the default auto parallel fan,
//!   `--color=never` blocks ANSI). Pinned upstream defaults, no config
//!   flag, scratch-root cwd. Check-only, never rewrites.
//! * pylint takes the whole stage file list as `--persistent=n --reports=n
//!   --score=n --output-format=json --jobs=1 <files>` (findings as a JSON
//!   array on stdout; `--persistent=n` disables the cache, `--reports=n`
//!   and `--score=n` suppress the human report/score). The cleared child
//!   environment (no `HOME`) plus scratch-root cwd leaves no discoverable
//!   `pylintrc`/`pyproject.toml`, so pinned upstream defaults apply.
//!   Check-only, never rewrites.
//! * Biome lint takes the whole stage file list as `lint --reporter=json
//!   --colors=off --error-on-warnings --vcs-enabled=false --config-path
//!   <dir> <files>` over scratch-root-relative paths. The config dir holds
//!   exactly one `biome.json` (hinted config or materialized `{}` defaults),
//!   so `--config-path` disables default resolution and no upward discovery
//!   can observe ambient state. Biome reports paths relative to its working
//!   directory, so backends re-anchor to the workspace-relative mirror paths
//!   (exit codes per probing: clean 0, findings 1). Biome lint is
//!   check-only: safe `--write` does not fix the fixable rules (needs
//!   `--unsafe`), so the runner never passes it and converges on format.
//! * Biome format takes the whole stage file list as `format
//!   --reporter=json --colors=off --config-path <dir> <files>` (check);
//!   format fix is `format --config-path <dir> --write <files>` (in-place,
//!   re-read on exit 0).
//! * ESLint takes the whole stage file list as `-c <config> -f json
//!   <files>` (check); fix is `-c <config> -f json --fix <files>`
//!   (in-place, re-read on exit 0 or 1, which signals remaining unfixable
//!   findings after the fixable ones were applied). The config is always
//!   explicit (no defaults exist); `cwd_rel` is the scratch root so the
//!   flat-config base path contains the mirrored sources.
//! * Prettier takes the whole stage file list as `--no-config
//!   --no-editorconfig --check <files>` (check); fix is `--no-config
//!   --no-editorconfig --write <files>` (in-place, re-read on exit 0).
//!   `--no-editorconfig` stays mandatory because the `editorconfig`
//!   package is absent from the runfiles forest, so `.editorconfig` files
//!   are currently inert; the flag freezes that behavior against future
//!   dependency additions.
//! * Error Prone check takes the whole stage file list as `javac
//!   -Xplugin:ErrorProne <files>` (diagnostics on stderr in the pinned
//!   javac shape, stdout empty; default severities with no `-Werror`,
//!   no `-verbose`, and no patch flags). Check-only: no files written,
//!   scratch-root cwd.
//! * Error Prone patch takes the same compile plus
//!   `-XepPatchChecks:<checks> -XepPatchLocation:<declared-dir>` over
//!   the whole stage file list. The declared dir is the per-target Bazel
//!   output directory holding `error-prone.patch` (unified diff relative
//!   to the source root, applied with `patch -p0 -u`); the runner
//!   validates the declared file and normalizes hunks to the edit
//!   contract. The `IN_PLACE` location is rejected: it mutates inputs in
//!   place, breaking sandbox immutability, action caching, and remote
//!   execution (and is experimental upstream).

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

/// rustfmt invocation. `config` is always explicit: the hinted config or
/// a scratch-materialized empty defaults file, so no upward discovery
/// can observe ambient state. `edition` is always explicit too: the
/// caller passes the crate's real edition (read from `CrateInfo` by the
/// quality aspect, never guessed or defaulted here), because the CLI
/// flag silently wins over any config `edition` key. `check` selects
/// `--check` instead of the in-place rewrite.
pub fn rustfmt(
    binary: &Path,
    files: &[&Path],
    config: &Path,
    edition: &str,
    check: bool,
) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--edition"),
        OsString::from(edition),
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
/// --no-respect-ignore-files` plus one `--extra-search-path DIR` per import
/// search dir (ty dep context,) over the whole stage file list.
/// Exit 1 with concise diagnostics is findings; `All checks passed!`
/// exit 0 is clean. Check-only: the runner never passes `--fix` or
/// `--add-ignore`.
pub fn ty_check(binary: &Path, files: &[&Path], search_paths: &[&Path]) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("check"),
        OsString::from("--output-format"),
        OsString::from("concise"),
        OsString::from("--no-progress"),
        OsString::from("--no-respect-ignore-files"),
    ];
    for dir in search_paths {
        argv.push(OsString::from("--extra-search-path"));
        argv.push(dir.as_os_str().to_owned());
    }
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

/// flake8 lint check invocation: `--isolated --color=never --jobs=1
/// --format <template>` over the whole stage file list. One
/// `path:row:col:code:text` line per finding on stdout (stderr empty);
/// clean prints nothing. Exit 1 with parsable lines is findings; exit 0
/// is clean. Check-only: no fix flag exists.
pub fn flake8_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--isolated"),
        OsString::from("--color=never"),
        OsString::from("--jobs=1"),
        OsString::from("--format"),
        OsString::from("%(path)s:%(row)s:%(col)s:%(code)s:%(text)s"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// pylint lint check invocation: `--persistent=n --reports=n --score=n
/// --output-format=json --jobs=1` over the whole stage file list. A JSON
/// array on stdout (one object per message); clean prints `[]`. Exit code
/// is a bit-encoded message-class mask (1 fatal, 2 error, 4 warning, 8
/// refactor, 16 convention, 32 usage error), so nonzero with parsable JSON
/// is findings and exit 0 is clean. Check-only: the runner never passes a
/// fix flag.
pub fn pylint_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--persistent=n"),
        OsString::from("--reports=n"),
        OsString::from("--score=n"),
        OsString::from("--output-format=json"),
        OsString::from("--jobs=1"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Biome shared prefix: `binary`, the subcommand, then machine flags plus
/// the explicit config selection (`--config-path <dir>`).
fn biome_base(binary: &Path, subcommand: &str, config_dir: &Path) -> Vec<OsString> {
    vec![
        binary.as_os_str().to_owned(),
        OsString::from(subcommand),
        OsString::from("--reporter=json"),
        OsString::from("--colors=off"),
        OsString::from("--config-path"),
        config_dir.as_os_str().to_owned(),
    ]
}

/// Biome lint check invocation: `lint --error-on-warnings
/// --vcs-enabled=false` over the whole stage file list. Exit 1 with valid
/// JSON is findings; exit 0 is clean. Check-only: the runner never passes
/// `--write` (safe write does not fix the fixable lint rules; only
/// `--unsafe` would, and it is never used).
pub fn biome_lint_check(binary: &Path, files: &[&Path], config_dir: &Path) -> Invocation {
    let mut argv = biome_base(binary, "lint", config_dir);
    argv.push(OsString::from("--error-on-warnings"));
    argv.push(OsString::from("--vcs-enabled=false"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Biome format check invocation: `format` over the whole stage file list.
/// Exit 1 with valid JSON is findings; exit 0 is clean.
pub fn biome_format_check(binary: &Path, files: &[&Path], config_dir: &Path) -> Invocation {
    let mut argv = biome_base(binary, "format", config_dir);
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Biome format fix invocation: `format --write` (in-place). The caller
/// re-reads on exit 0 and returns its input otherwise.
pub fn biome_format_fix(binary: &Path, files: &[&Path], config_dir: &Path) -> Invocation {
    let mut argv = biome_base(binary, "format", config_dir);
    argv.push(OsString::from("--write"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// ESLint lint check invocation: `-c <config> -f json` over the whole
/// stage file list. Exit 1 with valid JSON is findings; exit 0 is clean.
/// The config is always explicit: without one ESLint hard-fails
/// (`couldn't find an eslint.config.* file`), so the caller resolves it
/// first and fails the action when absent.
pub fn eslint_check(binary: &Path, files: &[&Path], config: &Path) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("-c"),
        config.as_os_str().to_owned(),
        OsString::from("-f"),
        OsString::from("json"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// ESLint lint fix invocation: `-c <config> -f json --fix` (in-place).
/// The caller re-reads on exit 0 or 1 (exit 1 signals remaining unfixable
/// findings after the fixable ones were applied) and keeps its input on
/// any other exit, mirroring the Ruff lint-fix contract.
pub fn eslint_fix(binary: &Path, files: &[&Path], config: &Path) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("-c"),
        config.as_os_str().to_owned(),
        OsString::from("-f"),
        OsString::from("json"),
        OsString::from("--fix"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Prettier shared prefix: `binary` plus the frozen hermetic flags
/// (`--no-config --no-editorconfig`, never observing config files).
/// Callers append the mode flag and files.
fn prettier_base(binary: &Path) -> Vec<OsString> {
    vec![
        binary.as_os_str().to_owned(),
        OsString::from("--no-config"),
        OsString::from("--no-editorconfig"),
    ]
}

/// Prettier format check invocation: `--check` over the whole stage file
/// list. Exit 1 with `[warn] <file>` stderr lines is findings; exit 0 is
/// clean.
pub fn prettier_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = prettier_base(binary);
    argv.push(OsString::from("--check"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Prettier format fix invocation: `--write` (in-place). The caller
/// re-reads on exit 0 and returns its input otherwise.
pub fn prettier_fix(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = prettier_base(binary);
    argv.push(OsString::from("--write"));
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

/// Scalafmt check invocation: `scalafmt --check` over the whole stage
/// file list plus `--config <hint>` when hinted. Exit 0 clean, exit 1
/// with unified diff on stdout when dirty. Scratch-root cwd blocks
/// ambient `.scalafmt.conf` discovery; the hinted config directory is
/// passed explicitly, never discovered.
pub fn scalafmt_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("--check")];
    if let Some(path) = config {
        argv.push(OsString::from("--config"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Scalafmt fix invocation: in-place rewrite over the whole stage file
/// list. The caller re-reads on exit 0 and returns its input otherwise.
pub fn scalafmt_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned()];
    if let Some(path) = config {
        argv.push(OsString::from("--config"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Scalafix check invocation: custom Java entrypoint binding
/// `scalafix.interfaces.ScalafixMainCallback` over semantic-rule
/// artifacts. Target-coupled `--classpath` plus `--sourceroot` plus
/// `--semanticdb-targetroots` come from the authoritative target;
/// syntactic-only runs pass none. Console-parse rejected: diagnostics
/// arrive as callback NDJSON on stdout, never console text.
/// Check-only with sandbox-apply-and-diff fix flow and declared outputs.
pub fn scalafix_check(
    binary: &Path,
    files: &[&Path],
    sourceroot: Option<&Path>,
    classpath: Option<&str>,
    semanticdb_targetroots: Option<&Path>,
) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned()];
    if let Some(root) = sourceroot {
        argv.push(OsString::from("--sourceroot"));
        argv.push(root.as_os_str().to_owned());
    }
    if let Some(cp) = classpath {
        argv.push(OsString::from("--classpath"));
        argv.push(OsString::from(cp));
    }
    if let Some(roots) = semanticdb_targetroots {
        argv.push(OsString::from("--semanticdb-targetroots"));
        argv.push(roots.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// CSharpier check invocation: `check` over the whole stage file list
/// plus `--config-path <hint>` when hinted. Exit 0 clean, exit 1 with
/// unformatted paths on stdout when dirty. Scratch-root cwd blocks
/// ambient config discovery. Declared DLLs over the managed .NET
/// runtime, never `dotnet tool install`.
pub fn csharpier_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("check")];
    if let Some(path) = config {
        argv.push(OsString::from("--config-path"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// CSharpier fix invocation: `format` (in-place). The caller re-reads
/// on exit 0 and returns its input otherwise.
pub fn csharpier_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("format")];
    if let Some(path) = config {
        argv.push(OsString::from("--config-path"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Fantomas check invocation: `check --json` over the whole stage file
/// list. Exit 0 all unchanged, exit 99 with `needs-formatting` files,
/// exit 1 operational failure. JSON on stdout carries per-file status;
/// the caller re-anchors workspace-relative paths. Declared DLLs over
/// the managed .NET runtime, never `dotnet tool install`.
pub fn fantomas_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["check", "--json"], files, "")
}

/// Fantomas fix invocation: in-place format over the whole stage file
/// list. The caller re-reads on exit 0 and returns its input otherwise.
pub fn fantomas_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &[], files, "")
}

/// Roslyn check invocation shape: `csc /errorlog:<sarif>` per pivot.
/// The adapter never spawns this directly in the runner: per-pivot
/// SARIF files are declared action inputs from the authoritative
/// target, concatenated into one log in deterministic pivot order and
/// parsed via `parsers::parse_roslyn`. Single-SARIF and merged-run
/// rejected. Check-only with sandbox-apply-and-diff fix flow.
pub fn roslyn_errorlog(sarif: &Path) -> OsString {
    OsString::from(format!("/errorlog:{}", sarif.to_string_lossy()))
}

/// FSharpLint check invocation: custom .NET entrypoint binding
/// `FSharpLint.Application.Lint` with `ReceivedWarning` over exact
/// package artifacts. Target-coupled `.fsproj`/`.sln` plus
/// `fsharplint.json` come from the authoritative target. Console-parse
/// rejected: diagnostics arrive as library NDJSON on stdout, never
/// console text. Check-only with sandbox-apply-and-diff fix flow.
pub fn fsharplint_check(
    binary: &Path,
    files: &[&Path],
    project: Option<&Path>,
    config: Option<&Path>,
) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned()];
    if let Some(proj) = project {
        argv.push(OsString::from("--project"));
        argv.push(proj.as_os_str().to_owned());
    }
    if let Some(cfg) = config {
        argv.push(OsString::from("--config"));
        argv.push(cfg.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

/// Declared Error Prone patch-file name: the patch invocation writes
/// exactly this file into the declared output directory as a unified
/// diff relative to the source root.
pub const ERROR_PRONE_PATCH_FILE: &str = "error-prone.patch";

/// Error Prone check invocation: `javac -Xplugin:ErrorProne` over the
/// whole stage file list. Diagnostics print to stderr in the pinned
/// javac shape (`parsers::parse_error_prone`); stdout stays empty.
/// Default severities with no `-Werror`, no `-verbose`, and no patch
/// flags: check-only, no files written, scratch-root cwd.
pub fn error_prone_check(javac: &Path, files: &[&Path]) -> Invocation {
    invocation(javac, &["-Xplugin:ErrorProne"], files, "")
}

/// Error Prone patch invocation: the check compile plus
/// `-XepPatchChecks:<checks>` with `-XepPatchLocation:<patch_dir>`.
/// `patch_dir` is the per-target declared output directory that will
/// hold `error-prone.patch`; the runner validates the declared file
/// and normalizes hunks to the edit contract. The `IN_PLACE` location
/// is rejected: it mutates inputs in place, breaking sandbox
/// immutability, action caching, and remote execution (and is
/// experimental upstream). `checks` is the comma-separated Error Prone
/// check list (for example `MissingOverride,DefaultCharset`);
/// `patch_dir` renders verbatim, so callers pass the declared dir,
/// never the `IN_PLACE` literal.
pub fn error_prone_patch(
    javac: &Path,
    files: &[&Path],
    patch_dir: &Path,
    checks: &str,
) -> Invocation {
    let mut argv = vec![
        javac.as_os_str().to_owned(),
        OsString::from("-Xplugin:ErrorProne"),
        OsString::from(format!("-XepPatchChecks:{checks}")),
        OsString::from(format!("-XepPatchLocation:{}", patch_dir.to_string_lossy())),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

#[path = "commands_tests.rs"]
#[cfg(test)]
mod commands_tests;

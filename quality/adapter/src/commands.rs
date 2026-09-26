use std::ffi::OsString;
use std::path::Path;

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

pub fn ruff_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = ruff_base(binary, "check", config);
    argv.push(OsString::from("--fix"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

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

pub fn ruff_format_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = ruff_base(binary, "format", config);
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

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

pub fn pydoclint_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("--quiet")];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

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

pub fn biome_format_check(binary: &Path, files: &[&Path], config_dir: &Path) -> Invocation {
    let mut argv = biome_base(binary, "format", config_dir);
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn biome_format_fix(binary: &Path, files: &[&Path], config_dir: &Path) -> Invocation {
    let mut argv = biome_base(binary, "format", config_dir);
    argv.push(OsString::from("--write"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

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

fn prettier_base(binary: &Path) -> Vec<OsString> {
    vec![
        binary.as_os_str().to_owned(),
        OsString::from("--no-config"),
        OsString::from("--no-editorconfig"),
    ]
}

pub fn prettier_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = prettier_base(binary);
    argv.push(OsString::from("--check"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn prettier_fix(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = prettier_base(binary);
    argv.push(OsString::from("--write"));
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

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

pub fn fantomas_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["check", "--json"], files, "")
}

pub fn fantomas_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &[], files, "")
}

pub fn roslyn_errorlog(sarif: &Path) -> OsString {
    OsString::from(format!("/errorlog:{}", sarif.to_string_lossy()))
}

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

pub fn clang_format_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--dry-run"),
        OsString::from("--Werror"),
    ];
    if let Some(path) = config {
        argv.push(OsString::from(format!(
            "--style=file:{}",
            path.to_string_lossy()
        )));
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn clang_format_fix(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("-i")];
    if let Some(path) = config {
        argv.push(OsString::from(format!(
            "--style=file:{}",
            path.to_string_lossy()
        )));
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn gofumpt_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-d"], files, "")
}

pub fn gofumpt_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-w"], files, "")
}

pub fn clang_tidy_check(
    binary: &Path,
    files: &[&Path],
    config: Option<&Path>,
    compile_commands_dir: Option<&Path>,
) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("--quiet")];
    if let Some(cfg) = config {
        argv.push(OsString::from("--config-file"));
        argv.push(cfg.as_os_str().to_owned());
    }
    if let Some(dir) = compile_commands_dir {
        argv.push(OsString::from("-p"));
        argv.push(dir.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn cppcheck_check(binary: &Path, files: &[&Path], suppressions: Option<&Path>) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--xml"),
        OsString::from("--xml-version=2"),
    ];
    if let Some(path) = suppressions {
        argv.push(OsString::from(format!(
            "--suppressions-list={}",
            path.to_string_lossy()
        )));
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn staticcheck_check(
    binary: &Path,
    files: &[&Path],
    config_dir_rel: Option<&str>,
) -> Invocation {
    match config_dir_rel {
        Some(dir) => invocation(binary, &["-f", "json"], files, dir),
        None => invocation(binary, &["-f", "json"], files, ""),
    }
}

pub fn govet_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &[], files, "")
}

pub fn errcheck_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &[], files, "")
}

pub const ERROR_PRONE_PATCH_FILE: &str = "error-prone.patch";

pub fn error_prone_check(javac: &Path, files: &[&Path]) -> Invocation {
    invocation(javac, &["-Xplugin:ErrorProne"], files, "")
}

pub fn google_java_format_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--dry-run", "--set-exit-if-changed"], files, "")
}

pub fn google_java_format_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--replace"], files, "")
}

pub fn ktfmt_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(
        binary,
        &["--kotlinlang-style", "--dry-run", "--set-exit-if-changed"],
        files,
        "",
    )
}

pub fn ktfmt_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--kotlinlang-style"], files, "")
}

pub fn checkstyle_check(binary: &Path, files: &[&Path], config: &Path) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("-c"),
        config.as_os_str().to_owned(),
        OsString::from("-f"),
        OsString::from("sarif"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn pmd_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("check")];
    for file in files {
        argv.push(OsString::from("--dir"));
        argv.push(file.as_os_str().to_owned());
    }
    argv.push(OsString::from("--format"));
    argv.push(OsString::from("sarif"));
    argv.push(OsString::from("--rulesets"));
    match config {
        Some(path) => argv.push(path.as_os_str().to_owned()),
        None => argv.push(OsString::from("rulesets/java/quickstart.xml")),
    }
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn spotbugs_check(binary: &Path, files: &[&Path]) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("-textui"),
        OsString::from("-effort:default"),
        OsString::from("-sarif"),
    ];
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn ktlint_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(
        binary,
        &["--relative", "--log-level=none", "--reporter=sarif"],
        files,
        "",
    )
}

pub fn ktlint_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--relative", "--format"], files, "")
}

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

pub fn buf_lint_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["lint", "--error-format=json"], files, "")
}

pub fn buf_format_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["format", "--diff", "--exit-code"], files, "")
}

pub fn buf_format_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["format", "--write"], files, "")
}

pub fn qmlformat_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--check"], files, "")
}

pub fn qmlformat_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-i"], files, "")
}

pub fn qmllint_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--json", "-"], files, "")
}

pub fn cue_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["fmt", "--check", "--diff"], files, "")
}

pub fn cue_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["fmt", "--write"], files, "")
}

pub fn jsonnetfmt_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--test"], files, "")
}

pub fn jsonnetfmt_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-i"], files, "")
}

pub fn pkl_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--check"], files, "")
}

pub fn pkl_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--write"], files, "")
}

pub fn modfmt_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-d"], files, "")
}

pub fn modfmt_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-w"], files, "")
}

pub fn terraform_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["fmt", "-check", "-diff"], files, "")
}

pub fn terraform_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["fmt", "-write"], files, "")
}

pub fn yamlfmt_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-lint"], files, "")
}

pub fn yamlfmt_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-write"], files, "")
}

pub fn shfmt_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-d"], files, "")
}

pub fn shfmt_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["-w"], files, "")
}

pub fn standardrb_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--check"], files, "")
}

pub fn standardrb_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--fix"], files, "")
}

pub fn djlint_format_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--reformat", "--check"], files, "")
}

pub fn djlint_format_fix(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--reformat"], files, "")
}

pub fn djlint_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![binary.as_os_str().to_owned(), OsString::from("--lint")];
    if let Some(path) = config {
        argv.push(OsString::from("--configuration"));
        argv.push(path.as_os_str().to_owned());
    }
    argv.extend(files.iter().map(|path| path.as_os_str().to_owned()));
    Invocation {
        argv,
        cwd_rel: String::new(),
    }
}

pub fn stylelint_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("--formatter"),
        OsString::from("json"),
    ];
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

pub fn rubocop_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--format", "json"], files, "")
}

pub fn psscriptanalyzer_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &[], files, "")
}

pub fn yamllint_check(binary: &Path, files: &[&Path], config: Option<&Path>) -> Invocation {
    let mut argv = vec![
        binary.as_os_str().to_owned(),
        OsString::from("-f"),
        OsString::from("parsable"),
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

pub fn shellcheck_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &["--format=gcc"], files, "")
}

pub fn keep_sorted_check(binary: &Path, files: &[&Path]) -> Invocation {
    invocation(binary, &[], files, "")
}

#[path = "commands_tests.rs"]
#[cfg(test)]
mod commands_tests;

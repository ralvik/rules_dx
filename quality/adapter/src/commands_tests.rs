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
fn rustfmt_passes_caller_edition_and_config() {
    let file = Path::new(FILE);
    let check = rustfmt(
        Path::new(BIN),
        &[file],
        Path::new("/scratch/r.toml"),
        "2021",
        true,
    );
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
    // The edition is never defaulted: a different crate edition
    // flows straight through to the flag.
    let older = rustfmt(
        Path::new(BIN),
        &[file],
        Path::new("/scratch/r.toml"),
        "2018",
        true,
    );
    assert_eq!(argv_strings(&older)[2], "2018");
    let fix = rustfmt(
        Path::new(BIN),
        &[file],
        Path::new("/scratch/r.toml"),
        "2021",
        false,
    );
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
    let invocation = ty_check(Path::new(BIN), &[file], &[]);
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
    let search = Path::new("/scratch/pkg");
    let with_search = ty_check(Path::new(BIN), &[file], &[search]);
    let argv = argv_strings(&with_search);
    assert!(argv.contains(&"--extra-search-path".to_owned()));
    assert!(argv.contains(&"/scratch/pkg".to_owned()));
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

#[test]
fn flake8_check_is_isolated_single_job() {
    let file = Path::new("/scratch/a.py");
    let invocation = flake8_check(Path::new(BIN), &[file]);
    assert_eq!(
        argv_strings(&invocation),
        vec![
            BIN,
            "--isolated",
            "--color=never",
            "--jobs=1",
            "--format",
            "%(path)s:%(row)s:%(col)s:%(code)s:%(text)s",
            "/scratch/a.py"
        ]
    );
    assert_eq!(invocation.cwd_rel, "");
}

#[test]
fn pylint_check_is_json_without_cache_or_report() {
    let file = Path::new("/scratch/a.py");
    let invocation = pylint_check(Path::new(BIN), &[file]);
    assert_eq!(
        argv_strings(&invocation),
        vec![
            BIN,
            "--persistent=n",
            "--reports=n",
            "--score=n",
            "--output-format=json",
            "--jobs=1",
            "/scratch/a.py"
        ]
    );
    assert_eq!(invocation.cwd_rel, "");
}

#[test]
fn biome_lint_check_is_json_hermetic() {
    let file = Path::new("/scratch/src/a.js");
    let config = Path::new("/scratch/cfg");
    let invocation = biome_lint_check(Path::new(BIN), &[file], config);
    assert_eq!(
        argv_strings(&invocation),
        vec![
            BIN,
            "lint",
            "--reporter=json",
            "--colors=off",
            "--config-path",
            "/scratch/cfg",
            "--error-on-warnings",
            "--vcs-enabled=false",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(invocation.cwd_rel, "");
}

#[test]
fn biome_format_check_and_fix_share_config() {
    let file = Path::new("/scratch/src/a.js");
    let config = Path::new("/scratch/cfg");
    let check = biome_format_check(Path::new(BIN), &[file], config);
    assert_eq!(
        argv_strings(&check),
        vec![
            BIN,
            "format",
            "--reporter=json",
            "--colors=off",
            "--config-path",
            "/scratch/cfg",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(check.cwd_rel, "");
    let fix = biome_format_fix(Path::new(BIN), &[file], config);
    assert_eq!(
        argv_strings(&fix),
        vec![
            BIN,
            "format",
            "--reporter=json",
            "--colors=off",
            "--config-path",
            "/scratch/cfg",
            "--write",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(fix.cwd_rel, "");
}

#[test]
fn eslint_check_and_fix_share_config() {
    let file = Path::new("/scratch/src/a.js");
    let config = Path::new("/scratch/cfg/eslint.config.js");
    let check = eslint_check(Path::new(BIN), &[file], config);
    assert_eq!(
        argv_strings(&check),
        vec![
            BIN,
            "-c",
            "/scratch/cfg/eslint.config.js",
            "-f",
            "json",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(check.cwd_rel, "");
    let fix = eslint_fix(Path::new(BIN), &[file], config);
    assert_eq!(
        argv_strings(&fix),
        vec![
            BIN,
            "-c",
            "/scratch/cfg/eslint.config.js",
            "-f",
            "json",
            "--fix",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(fix.cwd_rel, "");
}

#[test]
fn prettier_check_and_fix_are_hermetic() {
    let file = Path::new("/scratch/src/a.js");
    let check = prettier_check(Path::new(BIN), &[file]);
    assert_eq!(
        argv_strings(&check),
        vec![
            BIN,
            "--no-config",
            "--no-editorconfig",
            "--check",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(check.cwd_rel, "");
    let fix = prettier_fix(Path::new(BIN), &[file]);
    assert_eq!(
        argv_strings(&fix),
        vec![
            BIN,
            "--no-config",
            "--no-editorconfig",
            "--write",
            "/scratch/src/a.js"
        ]
    );
    assert_eq!(fix.cwd_rel, "");
}

#[test]
fn error_prone_check_is_plugin_only() {
    let file = Path::new("/scratch/src/Hello.java");
    let invocation = error_prone_check(Path::new(BIN), &[file]);
    assert_eq!(
        argv_strings(&invocation),
        vec![BIN, "-Xplugin:ErrorProne", "/scratch/src/Hello.java"]
    );
    assert_eq!(invocation.cwd_rel, "");
}

#[test]
fn error_prone_patch_declares_a_directory_never_in_place() {
    let file = Path::new("/scratch/src/Hello.java");
    let dir = Path::new("/scratch/patch-out");
    let invocation = error_prone_patch(
        Path::new(BIN),
        &[file],
        dir,
        "MissingOverride,DefaultCharset",
    );
    let argv = argv_strings(&invocation);
    assert_eq!(
        argv,
        vec![
            BIN,
            "-Xplugin:ErrorProne",
            "-XepPatchChecks:MissingOverride,DefaultCharset",
            "-XepPatchLocation:/scratch/patch-out",
            "/scratch/src/Hello.java"
        ]
    );
    assert_eq!(invocation.cwd_rel, "");
    assert!(
        !argv.iter().any(|arg| arg.contains("IN_PLACE")),
        "patch location is a declared dir, never IN_PLACE",
    );
    assert_eq!(ERROR_PRONE_PATCH_FILE, "error-prone.patch");
}

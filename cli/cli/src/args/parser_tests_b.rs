//! Invocation-parser tests (part 2/2) — split from `args/parser.rs` with no behavior change.
//! Originally the inline `mod tests` of `parser.rs`.
#![allow(unused_imports)]

use super::*;

use super::super::{ArgsError, Command, ReportRequest};
use super::parse;
use dx_output::{OutputMode, Threshold};

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(ToString::to_string).collect()
}

#[test]
fn bump_needs_exactly_one_selector_plus_version() {
    // `dx bump <selector> <version>`: one requirement,
    // never batch, mutating without confirmation.
    let bump = parse(&args(&["bump", "cargo:anyhow", "1.2.3"])).expect("parse bump");
    assert_eq!(bump.command, Command::Bump);
    assert_eq!(bump.command.name(), "bump");
    assert!(bump.command.is_audit_update());
    assert!(!bump.command.is_workflow());
    assert!(!bump.command.is_adoption());
    assert!(!bump.command.is_managed());
    assert_eq!(
        bump.targets,
        vec!["cargo:anyhow".to_owned(), "1.2.3".to_owned()]
    );
    assert_eq!(
        parse(&args(&["bump"])),
        Err(ArgsError::MissingValue {
            option: "<selector> <version>".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["bump", "cargo:anyhow"])),
        Err(ArgsError::MissingValue {
            option: "<selector> <version>".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["bump", "cargo:anyhow", "1.2.3", "npm:react"])),
        Err(ArgsError::MissingValue {
            option: "<selector> <version>".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["bump", "cargo:anyhow", "1.2.3", "--check"])),
        Err(ArgsError::UnsupportedOption {
            command: "bump",
            option: "--check".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["bump", "cargo:anyhow", "1.2.3", "--fail-on=error"])),
        Err(ArgsError::UnsupportedOption {
            command: "bump",
            option: "--fail-on".to_owned(),
        })
    );
    let got = parse(&args(&["bump", "cargo:anyhow", "1.2.3", "--output=json"])).expect("bump json");
    assert_eq!(got.command, Command::Bump);
    assert_eq!(got.output, OutputMode::Json);
    assert_eq!(
        parse(&args(&["bump", "cargo:anyhow", "1.2.3", "--output=diff"])),
        Err(ArgsError::UnsupportedOption {
            command: "bump",
            option: "--output=diff".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&[
            "bump",
            "cargo:anyhow",
            "1.2.3",
            "--report=sarif=out.sarif"
        ])),
        Err(ArgsError::UnsupportedOption {
            command: "bump",
            option: "--report=sarif=out.sarif".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["bump", "cargo:anyhow", "1.2.3", "--", "--jobs=4"])),
        Err(ArgsError::UnsupportedOption {
            command: "bump",
            option: "--".to_owned(),
        })
    );
}

#[test]
fn migrate_needs_from_and_to_versions() {
    // `dx migrate --from <version> --to <version>`:
    // both Cargo semver, upgrade-only gate, one manifest per
    // major hop plus one per full version pair for minor/patch,
    // mutating by default with fail-closed execution.
    let migrate = parse(&args(&["migrate", "--from=1.2.3", "--to=2.0.0"])).expect("parse migrate");
    assert_eq!(migrate.command, Command::Migrate);
    assert_eq!(migrate.command.name(), "migrate");
    assert!(!migrate.command.is_audit_update());
    assert!(!migrate.command.is_workflow());
    assert!(!migrate.command.is_adoption());
    assert!(!migrate.command.is_managed());
    assert!(migrate.command.is_mutating_by_default());
    assert_eq!(migrate.from, Some("1.2.3".to_owned()));
    assert_eq!(migrate.to, Some("2.0.0".to_owned()));
    let spaced =
        parse(&args(&["migrate", "--from", "1.2.3", "--to", "2.0.0"])).expect("spaced parse");
    assert_eq!(spaced.from, Some("1.2.3".to_owned()));
    assert_eq!(spaced.to, Some("2.0.0".to_owned()));
    let scoped =
        parse(&args(&["migrate", "--from=1.2.3", "--to=2.0.0", "//a:one"])).expect("scoped parse");
    assert_eq!(scoped.targets, vec!["//a:one".to_owned()]);
    assert_eq!(
        parse(&args(&["migrate", "--from=1.2.3"])),
        Err(ArgsError::MissingValue {
            option: "--from <version> --to <version>".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["migrate"])),
        Err(ArgsError::MissingValue {
            option: "--from <version> --to <version>".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["migrate", "--from", "--to=2.0.0"])),
        Err(ArgsError::MissingValue {
            option: "--from".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["migrate", "--from=1.2.3", "--to=2.0.0", "--check"])),
        Err(ArgsError::UnsupportedOption {
            command: "migrate",
            option: "--check".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&[
            "migrate",
            "--from=1.2.3",
            "--to=2.0.0",
            "--fail-on=error"
        ])),
        Err(ArgsError::UnsupportedOption {
            command: "migrate",
            option: "--fail-on".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&[
            "migrate",
            "--from=1.2.3",
            "--to=2.0.0",
            "--report=sarif=out.sarif"
        ])),
        Err(ArgsError::UnsupportedOption {
            command: "migrate",
            option: "--report=sarif=out.sarif".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&[
            "migrate",
            "--from=1.2.3",
            "--to=2.0.0",
            "--",
            "--jobs=4"
        ])),
        Err(ArgsError::UnsupportedOption {
            command: "migrate",
            option: "--".to_owned(),
        })
    );
    // `--from`/`--to` belong to migrate only.
    assert_eq!(
        parse(&args(&["lint", "--from=1.0.0"])),
        Err(ArgsError::UnsupportedOption {
            command: "lint",
            option: "--from".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["build", "//a:one", "--to=2.0.0"])),
        Err(ArgsError::UnsupportedOption {
            command: "build",
            option: "--to".to_owned(),
        })
    );
}

#[test]
fn run_rejects_machine_output_and_reports() {
    assert_eq!(
        parse(&args(&["run", "//app:bin", "--output=json"])),
        Err(ArgsError::UnsupportedOption {
            command: "run",
            option: "--output=json".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["run", "//app:bin", "--output=diff"])),
        Err(ArgsError::UnsupportedOption {
            command: "run",
            option: "--output=diff".to_owned(),
        })
    );
    assert_eq!(
        parse(&args(&["run", "--report=junit=out.xml"])),
        Err(ArgsError::UnsupportedOption {
            command: "run",
            option: "--report=junit=out.xml".to_owned(),
        })
    );
    let got = parse(&args(&["run", "//app:bin", "--", "--port=8080"])).expect("parse run");
    assert_eq!(got.command, Command::Run);
    assert_eq!(got.targets, vec!["//app:bin".to_owned()]);
    assert_eq!(got.bazel_options, vec!["--port=8080".to_owned()]);
}

#[test]
fn output_contract_has_no_silent_ignore() {
    // Every command either supports a machine-output mode or
    // rejects it pre-exec with `UnsupportedOption`. Silent ignore (accept
    // the flag, print text anyway) is never allowed.
    //
    // JSON-capable: quality, generate, workflow build/test/coverage,
    // umbrellas, audit, update, status.
    for command in [
        "lint",
        "typecheck",
        "format",
        "generate",
        "build",
        "test",
        "coverage",
        "check",
        "fix",
        "audit",
        "update",
    ] {
        let got = parse(&args(&[command, "--output=json"])).expect("json capable");
        assert_eq!(got.output, OutputMode::Json, "command: {command}");
        assert!(got.command.supports_json(), "command: {command}");
    }
    // Bump is JSON-capable with its required positionals
    // (`dx bump <selector> <version>` never runs bare).
    let got = parse(&args(&["bump", "cargo:anyhow", "1.2.3", "--output=json"])).expect("bump json");
    assert_eq!(got.output, OutputMode::Json);
    assert!(got.command.supports_json());
    // Migrate is JSON-capable with its required versions
    // (`dx migrate --from/--to` never runs bare).
    let got = parse(&args(&[
        "migrate",
        "--from=1.2.3",
        "--to=2.0.0",
        "--output=json",
    ]))
    .expect("migrate json");
    assert_eq!(got.output, OutputMode::Json);
    assert!(got.command.supports_json());
    let got = parse(&args(&["status", "--output=json"])).expect("status json");
    assert_eq!(got.output, OutputMode::Json);
    assert!(Command::Status.supports_json());
    // Diff-capable: patch producers only.
    for command in ["lint", "typecheck", "format", "generate", "check", "fix"] {
        let got = parse(&args(&[command, "--output=diff"])).expect("diff capable");
        assert_eq!(got.output, OutputMode::Diff, "command: {command}");
        assert!(got.command.supports_diff(), "command: {command}");
    }
    // Text-only exemptions fail fast on both machine modes.
    // (`bazel` takes dx flags before the command word; tokens after it
    // forward verbatim to the launcher.)
    for words in [
        vec!["clean", "--output=json"],
        vec!["clean", "--output=diff"],
        vec!["codegen", "--output=json"],
        vec!["env", "--output=diff"],
        vec!["setup", "--output=json"],
        vec!["--output=json", "bazel", "version"],
        vec!["init", "--output=json"],
        vec!["init", "proj", "--output=diff"],
        vec!["hooks", "status", "--output=json"],
        vec!["version", "--output=json"],
        vec!["watch", "test", "--output=json"],
        vec!["owners", "//a:one", "--output=json"],
        vec!["deps", "//a:one", "--output=diff"],
        vec!["why", "src/main.rs", "//a:one", "--output=json"],
        vec!["completion", "bash", "--output=json"],
    ] {
        assert!(
            matches!(
                parse(&args(&words)),
                Err(ArgsError::UnsupportedOption { .. })
            ),
            "words: {words:?}"
        );
    }
    // No patch to emit: JSON-capable but diff-rejecting commands fail
    // fast on `--output=diff` instead of printing empty stdout.
    for words in [
        vec!["build", "//a:one", "--output=diff"],
        vec!["test", "//a:one", "--output=diff"],
        vec!["coverage", "--output=diff"],
        vec!["audit", "--output=diff"],
        vec!["update", "--output=diff"],
        vec!["status", "--output=diff"],
        vec!["bump", "cargo:anyhow", "1.2.3", "--output=diff"],
        vec!["migrate", "--from=1.2.3", "--to=2.0.0", "--output=diff"],
    ] {
        assert_eq!(
            parse(&args(&words)),
            Err(ArgsError::UnsupportedOption {
                command: words[0],
                option: "--output=diff".to_owned(),
            }),
            "words: {words:?}"
        );
    }
}

#[test]
fn bazel_forwards_verbatim_and_rejects_dx_options() {
    let got = parse(&args(&["bazel", "build", "//...", "--", "--jobs=4"])).expect("parse");
    assert_eq!(got.command, Command::Bazel);
    assert_eq!(got.command.name(), "bazel");
    assert!(!got.command.is_workflow());
    assert!(!got.command.is_adoption());
    assert!(got.targets.is_empty());
    assert_eq!(
        got.bazel_options,
        vec![
            "build".to_owned(),
            "//...".to_owned(),
            "--jobs=4".to_owned()
        ]
    );
    // Tokens after the command word forward verbatim even when
    // they look like dx options: dx globals must precede `bazel`.
    for words in [
        vec!["bazel", "--jobs=4"],
        vec!["bazel", "--check"],
        vec!["bazel", "--pin=0.1.0"],
        vec!["bazel", "version", "--configured"],
        vec!["bazel", "--", "--fail-on=error"],
    ] {
        let got = parse(&args(&words)).expect("verbatim");
        assert_eq!(got.command, Command::Bazel, "words: {words:?}");
        assert!(got.targets.is_empty(), "words: {words:?}");
    }
    let got = parse(&args(&["bazel", "build", "--jobs", "4"])).expect("verbatim");
    assert_eq!(
        got.bazel_options,
        vec!["build".to_owned(), "--jobs".to_owned(), "4".to_owned()]
    );
    // dx-owned options before the command word still fail fast so
    // launcher flags can never be misread as dx flags.
    for words in [
        vec!["--output=json", "bazel", "version"],
        vec!["--check", "bazel", "version"],
        vec!["--pin=0.1.0", "bazel", "version"],
        vec!["--configured", "bazel", "version"],
        vec!["--fail-on=error", "bazel", "build", "//..."],
        vec!["--report=sarif=x.sarif", "bazel", "build"],
    ] {
        assert!(
            matches!(
                parse(&args(&words)),
                Err(ArgsError::UnsupportedOption { .. })
            ),
            "words: {words:?}"
        );
    }
}

#[test]
fn version_rollback_check_and_configured_parse() {
    let got = parse(&args(&["version", "--rollback"])).expect("parse");
    assert_eq!(got.command, Command::Version);
    assert!(got.rollback);
    let got = parse(&args(&["version", "--check"])).expect("parse");
    assert!(got.check);
    let got = parse(&args(&["deps", "--configured", "//a:one"])).expect("parse");
    assert_eq!(got.command, Command::Deps);
    assert!(got.configured);
    for words in [
        vec!["status", "--rollback"],
        vec!["status", "--check"],
        vec!["owners", "//a:one", "--pin=0.1.0"],
        vec!["build", "//a:one", "--configured"],
        vec!["lint", "--rollback"],
        vec!["check", "//...", "--configured"],
        vec!["status", "--pin=0.1.0"],
    ] {
        assert!(
            matches!(
                parse(&args(&words)),
                Err(ArgsError::UnsupportedOption { .. })
            ),
            "words: {words:?}"
        );
    }
}

#[test]
fn managed_commands_parse_repo_and_exact_scopes() {
    for command in ["codegen", "env", "setup"] {
        let got = parse(&args(&[command])).expect("parse");
        assert!(got.command.is_managed());
        assert!(got.targets.is_empty());
        assert!(got.bazel_options.is_empty());
        let got = parse(&args(&[command, "//a:one", "--", "--jobs=4"])).expect("scoped parse");
        assert_eq!(got.targets, args(&["//a:one"]));
        assert_eq!(got.bazel_options, args(&["--jobs=4"]));
        let got = parse(&args(&[command, "@repo//pkg:lib"])).expect("external label");
        assert_eq!(got.targets, args(&["@repo//pkg:lib"]));
    }
    // Scope rules are the shared setup scope rules: multiple
    // positionals, patterns, and non-labels fail before execution.
    assert_eq!(
        parse(&args(&["codegen", "//a:one", "//b:two"])),
        Err(ArgsError::UnsupportedOption {
            command: "codegen",
            option: "//b:two".to_owned(),
        })
    );
    for words in [
        vec!["env", "//a/..."],
        vec!["setup", "src/main.rs"],
        vec!["codegen", ":target"],
        vec!["env", ""],
        vec!["setup", "--config=release"],
    ] {
        assert!(
            matches!(
                parse(&args(&words)),
                Err(ArgsError::EmptyScope)
                    | Err(ArgsError::RelativeLabel { .. })
                    | Err(ArgsError::InvalidScope { .. })
                    | Err(ArgsError::UnknownOption { .. })
            ),
            "words: {words:?}"
        );
    }
    // Quality-only, version-only, and clean-only options
    // fail fast on managed commands.
    for words in [
        vec!["codegen", "--check"],
        vec!["env", "--fail-on=error"],
        vec!["setup", "--output=json"],
        vec!["codegen", "--report=sarif=out.sarif"],
        vec!["env", "--pin=0.1.0"],
        vec!["setup", "--rollback"],
        vec!["codegen", "--configured"],
        vec!["env", "--configured"],
        vec!["setup", "--pin=0.2.0"],
        vec!["codegen", "--bazel"],
    ] {
        assert!(
            matches!(
                parse(&args(&words)),
                Err(ArgsError::UnsupportedOption { .. })
            ),
            "words: {words:?}"
        );
    }
}

#[test]
fn why_requires_file_and_label() {
    let got = parse(&args(&["why", "src/a.rs", "//a:one"])).expect("parse");
    assert_eq!(got.command, Command::Why);
    assert_eq!(
        got.targets,
        vec!["src/a.rs".to_owned(), "//a:one".to_owned()]
    );
    for words in [
        vec!["why", "src/a.rs"],
        vec!["why", "src/a.rs", "//a:one", "//b:two"],
    ] {
        assert_eq!(
            parse(&args(&words)),
            Err(ArgsError::MissingValue {
                option: "<file> <label>".to_owned(),
            }),
            "words: {words:?}"
        );
    }
}

#[test]
fn init_and_hooks_have_no_force_flag() {
    // Issue #700: absent-only writes plus unmanaged refusal ship with no
    // overwrite flag, so `--force` must fail as an unknown option rather
    // than read as a no-op.
    for words in [
        vec!["init", "--force"],
        vec!["init", "--force", "demo"],
        vec!["hooks", "install", "--force"],
    ] {
        assert!(
            matches!(
                parse(&args(&words)),
                Err(ArgsError::UnknownOption { option, .. }) if option == "--force"
            ),
            "words: {words:?} must reject --force as unknown, got {:?}",
            parse(&args(&words))
        );
    }
}

#[test]
fn here_selects_cwd_scope_only_via_explicit_flag() {
    // Issue #699: `--here` (`--cwd` alias) selects the current directory
    // tree on cwd-scope commands only, never implicitly, and never with
    // explicit scopes. The no-flag default stays `//...`.
    for command in [
        "audit",
        "lint",
        "typecheck",
        "format",
        "generate",
        "build",
        "test",
        "coverage",
        "check",
        "fix",
    ] {
        let got = parse(&args(&[command, "--here"])).expect("here parses");
        assert!(got.here, "command: {command}");
        assert!(got.targets.is_empty(), "command: {command}");
        assert!(got.command.supports_here(), "command: {command}");
        let alias = parse(&args(&[command, "--cwd"])).expect("cwd alias parses");
        assert!(alias.here, "command: {command}");
        // Flags may appear before or after the command word.
        let before = parse(&args(&["--here", command])).expect("before parses");
        assert!(before.here, "command: {command}");
        // Bare default stays repository-wide without the flag.
        let bare = parse(&args(&[command])).expect("bare parses");
        assert!(!bare.here, "command: {command}");
        assert!(bare.targets.is_empty(), "command: {command}");
    }
    // Audit allows one family selector plus `--here`.
    let family = parse(&args(&["audit", "security", "--here"])).expect("family plus here");
    assert!(family.here);
    assert_eq!(family.targets, vec!["security".to_owned()]);
    let family_alias = parse(&args(&["audit", "--cwd", "license"])).expect("family plus cwd");
    assert!(family_alias.here);
    // Explicit scopes never combine with `--here`.
    for words in [
        vec!["build", "--here", "//a:one"],
        vec!["lint", "src/a.py", "--here"],
        vec!["test", "--cwd", "//..."],
        vec!["audit", "--here", "//a:one"],
        vec!["audit", "security", "//a:one", "--here"],
        vec!["audit", "bogus", "--here"],
    ] {
        assert_eq!(
            parse(&args(&words)),
            Err(ArgsError::ConflictingHere),
            "words: {words:?}"
        );
    }
    // Every other command rejects `--here` instead of silently ignoring it.
    for words in [
        vec!["clean", "--here"],
        vec!["update", "--here"],
        vec!["bump", "cargo:anyhow", "1.2.3", "--here"],
        vec!["migrate", "--from=1.2.3", "--to=2.0.0", "--here"],
        vec!["codegen", "--here"],
        vec!["env", "--here"],
        vec!["setup", "--here"],
        vec!["run", "//app:bin", "--here"],
        vec!["deploy", "//app:bin", "--here"],
        vec!["status", "--here"],
        vec!["version", "--here"],
        vec!["owners", "//a:one", "--here"],
        vec!["completion", "bash", "--here"],
    ] {
        assert_eq!(
            parse(&args(&words)),
            Err(ArgsError::UnsupportedOption {
                command: words[0],
                option: "--here".to_owned(),
            }),
            "words: {words:?}"
        );
    }
    // `dx bazel` owns its tail verbatim: `--here` after it forwards to
    // Bazel, while dx-owned `--here` before it is rejected.
    let verbatim = parse(&args(&["bazel", "build", "--here"])).expect("verbatim");
    assert_eq!(verbatim.command, Command::Bazel);
    assert!(!verbatim.here);
    assert_eq!(
        parse(&args(&["--here", "bazel", "version"])),
        Err(ArgsError::UnsupportedOption {
            command: "bazel",
            option: "--here".to_owned(),
        })
    );
}

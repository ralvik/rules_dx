//! Process boundary tests (split from `lib.rs`).
//! Originally the inline `mod tests` of `lib.rs`.
#![allow(unused_imports)]

use super::*;

use std::collections::{HashMap, HashSet};

struct FakeFs {
    files: HashSet<PathBuf>,
    texts: HashMap<PathBuf, String>,
    errors: HashMap<PathBuf, io::ErrorKind>,
}

impl FakeFs {
    fn with_files(paths: &[&str]) -> FakeFs {
        FakeFs {
            files: paths.iter().map(PathBuf::from).collect(),
            texts: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    fn with_text(path: &str, text: &str) -> FakeFs {
        let mut fs = FakeFs::with_files(&[path]);
        fs.texts.insert(PathBuf::from(path), text.to_owned());
        fs
    }
}

impl Fs for FakeFs {
    fn is_file(&self, path: &Path) -> bool {
        self.files.contains(path)
    }

    fn read_text(&self, path: &Path) -> io::Result<String> {
        if let Some(text) = self.texts.get(path) {
            return Ok(text.clone());
        }
        if let Some(kind) = self.errors.get(path) {
            return Err(io::Error::new(*kind, "injected read failure"));
        }
        Err(io::Error::new(io::ErrorKind::NotFound, "no such file"))
    }
}

#[test]
fn discovers_module_at_start() {
    let fs = FakeFs::with_files(&["/repo/MODULE.bazel"]);
    let found = discover(Path::new("/repo/sub/dir"), None, &fs).expect("discover");
    assert_eq!(found, PathBuf::from("/repo"));
}

#[test]
fn discovers_module_at_start_itself() {
    let fs = FakeFs::with_files(&["/repo/MODULE.bazel"]);
    let found = discover(Path::new("/repo"), None, &fs).expect("discover");
    assert_eq!(found, PathBuf::from("/repo"));
}

#[test]
fn override_with_module_wins() {
    let fs = FakeFs::with_files(&["/other/MODULE.bazel"]);
    let found = discover(Path::new("/repo/sub"), Some(Path::new("/other")), &fs).expect("override");
    assert_eq!(found, PathBuf::from("/other"));
}

#[test]
fn override_without_marker_fails() {
    let fs = FakeFs::with_files(&[]);
    let err = discover(Path::new("/repo"), Some(Path::new("/other")), &fs).expect_err("must fail");
    assert_eq!(
        err,
        DiscoverError::InvalidOverride {
            path: PathBuf::from("/other"),
        }
    );
    assert!(err.to_string().contains("--workspace"));
    assert!(err.to_string().contains("/other"));
}

#[test]
fn override_with_legacy_reports_migration() {
    let fs = FakeFs::with_files(&["/other/WORKSPACE"]);
    let err = discover(Path::new("/repo"), Some(Path::new("/other")), &fs).expect_err("must fail");
    assert_eq!(
        err,
        DiscoverError::UnsupportedLegacy {
            dir: PathBuf::from("/other"),
        }
    );
    assert!(err.to_string().contains("Bzlmod"));
}

#[test]
fn legacy_workspace_bazel_reports_migration() {
    let fs = FakeFs::with_files(&["/repo/WORKSPACE.bazel"]);
    let err = discover(Path::new("/repo/sub"), None, &fs).expect_err("must fail");
    assert!(matches!(err, DiscoverError::UnsupportedLegacy { .. }));
    assert!(err.to_string().contains("WORKSPACE"));
}

#[test]
fn missing_module_lists_searched_and_suggests_override() {
    let fs = FakeFs::with_files(&[]);
    let err = discover(Path::new("/repo/sub"), None, &fs).expect_err("must fail");
    assert!(
        matches!(&err, DiscoverError::NotFound { searched } if searched.contains(&PathBuf::from("/repo/sub")))
    );
    assert!(
        matches!(&err, DiscoverError::NotFound { searched } if searched.contains(&PathBuf::from("/repo")))
    );
    assert!(err.to_string().contains("--workspace"));
}

#[test]
fn nearest_legacy_wins_over_distant_module() {
    // A legacy marker below a module still resolves to the module:
    // discovery prefers the nearest MODULE.bazel.
    let fs = FakeFs::with_files(&["/repo/MODULE.bazel", "/repo/sub/WORKSPACE"]);
    let found = discover(Path::new("/repo/sub"), None, &fs).expect("module wins");
    assert_eq!(found, PathBuf::from("/repo"));
}

#[test]
fn real_fs_roundtrip_discovers_workspace() {
    let scratch = dx_test_scratch::scratch("dx-discover-");
    let root = scratch.path().to_path_buf();
    let nested = root.join("a").join("b");
    std::fs::create_dir_all(&nested).expect("dirs");
    std::fs::write(root.join("MODULE.bazel"), "module(name = \"t\")\n").expect("marker");
    let found = discover_real(&nested, None).expect("real discover");
    assert_eq!(found, root);
    scratch.close().expect("cleanup");
}

#[test]
fn real_fs_missing_module_suggests_override() {
    let scratch = dx_test_scratch::scratch("dx-missing-");
    let root = scratch.path().to_path_buf();
    std::fs::create_dir_all(&root).expect("dirs");
    let err = discover_real(&root, None).expect_err("must fail");
    assert!(err.to_string().contains("--workspace"));
    scratch.close().expect("cleanup");
}

#[test]
fn pinned_version_trims_whitespace() {
    let fs = FakeFs::with_text("/repo/.bazelversion", "  9.2.0\n");
    let version = pinned_bazel_version(Path::new("/repo"), &fs).expect("pin");
    assert_eq!(version, "9.2.0");
}

#[test]
fn missing_pin_reports_workspace() {
    let fs = FakeFs::with_files(&[]);
    let err = pinned_bazel_version(Path::new("/repo"), &fs).expect_err("must fail");
    assert_eq!(
        err,
        LauncherError::MissingPin {
            workspace: PathBuf::from("/repo"),
        }
    );
    assert!(err.to_string().contains(".bazelversion"));
}

#[test]
fn empty_pin_fails() {
    let fs = FakeFs::with_text("/repo/.bazelversion", "   \n");
    let err = pinned_bazel_version(Path::new("/repo"), &fs).expect_err("must fail");
    assert_eq!(
        err,
        LauncherError::EmptyPin {
            workspace: PathBuf::from("/repo"),
        }
    );
    assert!(err.to_string().contains("empty"));
}

#[test]
fn unreadable_pin_reports_reason() {
    let mut fs = FakeFs::with_files(&[]);
    fs.errors.insert(
        PathBuf::from("/repo/.bazelversion"),
        io::ErrorKind::PermissionDenied,
    );
    let err = pinned_bazel_version(Path::new("/repo"), &fs).expect_err("must fail");
    assert!(matches!(err, LauncherError::UnreadablePin { .. }));
    assert!(err.to_string().contains(".bazelversion"));
}

#[test]
fn launcher_has_no_fallback() {
    assert_eq!(launcher_argv0(), "bazel");
    assert_eq!(WORKFLOW_STARTUP_OPTS, &["--nohome_rc", "--nosystem_rc"]);
}

#[test]
fn real_fs_reads_workspace_pin() {
    let scratch = dx_test_scratch::scratch("dx-pin-");
    let root = scratch.path().to_path_buf();
    std::fs::create_dir_all(&root).expect("dirs");
    std::fs::write(root.join(".bazelversion"), "9.2.0\n").expect("pin");
    let version = pinned_bazel_version_real(&root).expect("repo pin");
    assert_eq!(version, "9.2.0");
    scratch.close().expect("cleanup");
}

#[test]
fn scope_rendering_stays_compact() {
    assert_eq!(describe_scope(&Scope::Repository), "//...");
    assert_eq!(
        describe_scope(&Scope::Pattern("//src/auth/...".to_owned())),
        "//src/auth/..."
    );
    assert_eq!(
        describe_scope(&Scope::Labels(vec![
            "//a:a".to_owned(),
            "//b/...".to_owned()
        ])),
        "//a:a //b/..."
    );
    assert_eq!(
        describe_scope(&Scope::ResolvedOwners(vec!["//a:a".to_owned()])),
        "//a:a"
    );
    assert_eq!(describe_scope(&Scope::Count(1)), "1 target");
    assert_eq!(describe_scope(&Scope::Count(3)), "3 targets");
    assert_eq!(describe_scope(&Scope::Count(0)), "0 targets");
}

#[test]
fn summaries_never_render_argv() {
    let summary = operation_summary("lint", "analysis", &Scope::Count(3));
    assert_eq!(summary, "Running lint analysis for 3 targets");
    let summary = operation_summary("lint", "analysis", &Scope::Repository);
    assert_eq!(summary, "Running lint analysis for //...");
    let summary = operation_summary(
        "lint",
        "analysis",
        &Scope::Pattern("//src/auth/...".to_owned()),
    );
    assert_eq!(summary, "Running lint analysis for //src/auth/...");
}

#[test]
fn protected_check_accepts_repeated_required_value() {
    let protected = vec![ProtectedFlag {
        name: "keep_going".to_owned(),
        required: Some("--keep_going".to_owned()),
    }];
    let kept = check_protected(&["--keep_going".to_owned()], &protected).expect("repeat");
    assert_eq!(kept, vec!["--keep_going".to_owned()]);
}

#[test]
fn protected_check_rejects_conflict_without_echoing_values() {
    let protected = vec![ProtectedFlag {
        name: "keep_going".to_owned(),
        required: Some("--keep_going".to_owned()),
    }];
    let err =
        check_protected(&["--keep_going=false".to_owned()], &protected).expect_err("conflict");
    assert_eq!(
        err,
        ForwardError::ConflictingOption {
            flag: "keep_going".to_owned(),
        }
    );
    assert!(!err.to_string().contains("false"));
    assert!(err.to_string().contains("--keep_going"));
}

#[test]
fn protected_check_rejects_bare_name_against_valued_requirement() {
    let protected = vec![ProtectedFlag {
        name: "config".to_owned(),
        required: Some("--config=dx".to_owned()),
    }];
    let err = check_protected(&["--config=other".to_owned()], &protected).expect_err("conflict");
    assert_eq!(
        err,
        ForwardError::ConflictingOption {
            flag: "config".to_owned(),
        }
    );
    assert!(!err.to_string().contains("other"));
    assert!(!err.to_string().contains("dx"));
    assert!(err.to_string().contains("--config"));
}

#[test]
fn protected_check_rejects_unconditional_flag() {
    let protected = vec![ProtectedFlag {
        name: "build_event_json_file".to_owned(),
        required: None,
    }];
    let err = check_protected(
        &["--build_event_json_file=/tmp/bep.json".to_owned()],
        &protected,
    )
    .expect_err("conflict");
    assert_eq!(
        err,
        ForwardError::ConflictingOption {
            flag: "build_event_json_file".to_owned(),
        }
    );
    assert!(!err.to_string().contains("/tmp/bep.json"));
}

#[test]
fn protected_check_preserves_unrelated_order() {
    let protected = vec![ProtectedFlag {
        name: "keep_going".to_owned(),
        required: Some("--keep_going".to_owned()),
    }];
    let kept = check_protected(
        &[
            "--jobs=4".to_owned(),
            "plain".to_owned(),
            "--keep_going".to_owned(),
        ],
        &protected,
    )
    .expect("kept");
    assert_eq!(
        kept,
        vec![
            "--jobs=4".to_owned(),
            "plain".to_owned(),
            "--keep_going".to_owned()
        ]
    );
}

#[test]
fn startup_and_binary_args_are_detected() {
    assert!(is_startup_option("--output_base=/tmp/x"));
    assert!(is_startup_option("--bazelrc=/tmp/rc"));
    assert!(is_startup_option("--home_rc"));
    assert!(!is_startup_option("--jobs=4"));
    assert!(!is_startup_option("plain"));
    assert!(!is_startup_option("--"));
    assert!(is_test_binary_arg("--test_arg=fast"));
    assert!(!is_test_binary_arg("--jobs=4"));
    assert!(!is_test_binary_arg("plain"));
    assert!(flag_name("--").is_none());
    assert!(flag_name("plain").is_none());
}

#[test]
fn internal_errors_render_without_echoing_values() {
    let err = ForwardError::InvalidRequiredOption {
        flag: "keep_going".to_owned(),
    };
    assert!(err.to_string().contains("--keep_going"));
    let err = ForwardError::InvalidSetting {
        option: "clippy_output_diagnostics=true".to_owned(),
    };
    assert!(err.to_string().contains("malformed required setting"));
    let err = ForwardError::UnsupportedCommand {
        command: "lint".to_owned(),
    };
    assert!(err.to_string().contains("lint"));
}

#[test]
fn workflow_argv_orders_startup_command_required_user_labels() {
    let protected = vec![ProtectedFlag {
        name: "keep_going".to_owned(),
        required: Some("--keep_going".to_owned()),
    }];
    let argv = build_workflow_argv(
        "build",
        &["--jobs=4".to_owned(), "--keep_going".to_owned()],
        &["--keep_going".to_owned(), "--config=dx".to_owned()],
        &protected,
        &["//...".to_owned()],
    )
    .expect("argv");
    assert_eq!(
        argv,
        vec![
            "bazel",
            "--nohome_rc",
            "--nosystem_rc",
            "build",
            "--keep_going",
            "--config=dx",
            "--jobs=4",
            "--keep_going",
            "//...",
        ]
    );
}

#[test]
fn workflow_argv_rejects_startup_options() {
    let err = build_workflow_argv(
        "build",
        &["--output_base=/tmp/x".to_owned()],
        &[],
        &[],
        &["//...".to_owned()],
    )
    .expect_err("startup");
    assert_eq!(
        err,
        ForwardError::StartupOption {
            flag: "output_base".to_owned(),
        }
    );
    assert!(err.to_string().contains("dx bazel"));
    assert!(!err.to_string().contains("/tmp/x"));
}

#[test]
fn workflow_argv_rejects_test_binary_args() {
    let err = build_workflow_argv(
        "test",
        &["--test_arg=fast".to_owned()],
        &[],
        &[],
        &["//...".to_owned()],
    )
    .expect_err("binary args");
    assert_eq!(
        err,
        ForwardError::TestBinaryArgs {
            flag: "test_arg".to_owned(),
        }
    );
    assert!(err.to_string().contains("dx bazel"));
}

#[test]
fn execution_gaps_forwarding_matrix_is_wont_fix() {
    // Every Bazel startup option plus test-binary args
    // stays rejected on workflow commands with `dx bazel` guidance;
    // only `dx bazel` forwards unchanged. Pinned with fixtures in
    // `cli/cli/tests/fixtures/cli_execution_gaps/`.
    for startup in [
        "--bazelrc=/tmp/rc",
        "--home_rc",
        "--nohome_rc",
        "--system_rc",
        "--nosystem_rc",
        "--output_base=/tmp/x",
        "--output_user_root=/tmp/y",
        "--host_jvm_args=-Xmx1g",
        "--server_jvm_out=/tmp/jvm.out",
    ] {
        assert!(
            is_startup_option(startup),
            "{startup} must count as startup"
        );
        let err = build_workflow_argv(
            "build",
            &[startup.to_owned()],
            &[],
            &[],
            &["//...".to_owned()],
        )
        .expect_err("startup must fail");
        assert!(
            matches!(err, ForwardError::StartupOption { .. }),
            "{startup} produced {err:?}"
        );
        assert!(err.to_string().contains("dx bazel"), "{startup}: {err}");
    }
    for binary in ["--test_arg=fast", "--test_arg"] {
        assert!(
            is_test_binary_arg(binary),
            "{binary} must count as test-binary"
        );
        let err = build_workflow_argv(
            "test",
            &[binary.to_owned()],
            &[],
            &[],
            &["//...".to_owned()],
        )
        .expect_err("test-binary must fail");
        assert!(
            matches!(err, ForwardError::TestBinaryArgs { .. }),
            "{binary} produced {err:?}"
        );
    }
    // The transparent escape hatch forwards even startup options unchanged.
    let passthrough = build_bazel_passthrough(
        "bazel",
        &[
            "--output_base=/tmp/x".to_owned(),
            "build".to_owned(),
            "//...".to_owned(),
        ],
    );
    assert_eq!(
        passthrough,
        vec![
            "bazel".to_owned(),
            "--output_base=/tmp/x".to_owned(),
            "build".to_owned(),
            "//...".to_owned()
        ]
    );
}

#[test]
fn quality_workflows_reject_nokeep_going() {
    // Quality callers protect `nokeep_going` unconditionally so a
    // conflicting `--nokeep_going` weakens no result collection.
    let protected = vec![
        ProtectedFlag {
            name: "keep_going".to_owned(),
            required: Some("--keep_going".to_owned()),
        },
        ProtectedFlag {
            name: "nokeep_going".to_owned(),
            required: None,
        },
    ];
    let err = build_workflow_argv(
        "build",
        &["--nokeep_going".to_owned()],
        &["--keep_going".to_owned()],
        &protected,
        &["//...".to_owned()],
    )
    .expect_err("nokeep_going");
    assert_eq!(
        err,
        ForwardError::ConflictingOption {
            flag: "nokeep_going".to_owned(),
        }
    );
}

#[test]
fn workflow_argv_rejects_protected_conflicts() {
    let protected = vec![ProtectedFlag {
        name: "build_event_json_file".to_owned(),
        required: None,
    }];
    let err = build_workflow_argv(
        "build",
        &["--build_event_json_file=/tmp/bep.json".to_owned()],
        &["--build_event_json_file=/tmp/required.json".to_owned()],
        &protected,
        &["//...".to_owned()],
    )
    .expect_err("conflict");
    assert!(matches!(err, ForwardError::ConflictingOption { .. }));
    assert!(!err.to_string().contains("/tmp/bep.json"));
    assert!(!err.to_string().contains("/tmp/required.json"));
}

#[test]
fn bazel_passthrough_forwards_unchanged() {
    let args = vec![
        "--output_base=/tmp/x".to_owned(),
        "build".to_owned(),
        "//...".to_owned(),
    ];
    assert_eq!(
        build_bazel_passthrough("bazel", &args),
        vec![
            "bazel".to_owned(),
            "--output_base=/tmp/x".to_owned(),
            "build".to_owned(),
            "//...".to_owned()
        ]
    );
}

#[test]
fn exit_mapping_preserves_subprocess_codes() {
    assert_eq!(pre_exec_code(), 2);
    assert_eq!(operational_code(), 1);
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(subprocess_code(0), 0);
    assert_eq!(subprocess_code(3), 3);
    assert!(quality_keeps_going(true));
    assert!(!quality_keeps_going(false));
}

#[test]
fn dry_run_never_executes_final_workflows() {
    assert!(dry_run_allows(false, false));
    assert!(!dry_run_allows(false, true));
    assert!(!dry_run_allows(true, false));
    assert!(!dry_run_allows(true, true));
    assert!(dry_run_guard(false, false).is_ok());
    assert_eq!(
        dry_run_guard(false, true),
        Err(DryRunError::WouldExecuteAction)
    );
    assert_eq!(
        dry_run_guard(true, false),
        Err(DryRunError::WouldExecuteAction)
    );
    assert!(dry_run_guard(true, true).is_err());
    assert!(DryRunError::WouldExecuteAction
        .to_string()
        .contains("dry-run"));
}

#[test]
fn signal_numbers_match_os() {
    assert_eq!(signal_number(UnixSignal::Interrupt), libc::SIGINT);
    assert_eq!(signal_number(UnixSignal::Terminate), libc::SIGTERM);
}

#[test]
fn forward_signal_number_checks_existence_safely() {
    // Signal zero performs error checking without delivering.
    forward_signal_number(std::process::id(), 0).expect("self exists");
    assert!(forward_signal_number(1 << 30, 0).is_err());
    assert!(forward_signal_number(std::process::id(), -1).is_err());
}

#[test]
fn forward_signal_propagates_os_errors() {
    // An unallocated pid fails without delivering any signal.
    assert!(forward_signal(1 << 30, UnixSignal::Terminate).is_err());
}

#[test]
fn reraise_zero_is_safe() {
    reraise_number(0).expect("raise zero");
    assert!(reraise_number(-1).is_err());
}

struct FakeRunner {
    status: ChildStatus,
}

impl Runner for FakeRunner {
    fn run(&self, argv: &[String], cwd: &Path, _env: &[(&str, &str)]) -> io::Result<ChildStatus> {
        assert!(!argv.is_empty());
        assert!(cwd.is_absolute() || cwd.as_os_str() == ".");
        Ok(self.status.clone())
    }
}

#[test]
fn fake_runner_substitutes_the_boundary() {
    let runner = FakeRunner {
        status: ChildStatus { code: Some(3) },
    };
    let status = runner
        .run(
            &["bazel".to_owned(), "build".to_owned()],
            Path::new("."),
            &[],
        )
        .expect("fake");
    assert_eq!(status.code, Some(3));
}

#[test]
fn system_runner_preserves_exit_codes() {
    let runner = SystemRunner;
    let ok = runner
        .run(&["/bin/true".to_owned()], Path::new("/"), &[])
        .expect("true");
    assert_eq!(ok.code, Some(0));
    let fail = runner
        .run(&["/bin/false".to_owned()], Path::new("/"), &[])
        .expect("false");
    assert_eq!(fail.code, Some(1));
}

#[test]
fn system_runner_forwards_extra_environment() {
    let runner = SystemRunner;
    let probed = runner
        .run(
            &[
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "test \"$DX_RUNNER_PROBE\" = forwarded".to_owned(),
            ],
            Path::new("/"),
            &[("DX_RUNNER_PROBE", "forwarded")],
        )
        .expect("probe");
    assert_eq!(probed.code, Some(0));
}

#[test]
fn system_runner_rejects_bad_invocations() {
    let runner = SystemRunner;
    assert!(runner.run(&[], Path::new("/"), &[]).is_err());
    assert!(runner
        .run(&["/nonexistent-dx-tool".to_owned()], Path::new("/"), &[])
        .is_err());
}

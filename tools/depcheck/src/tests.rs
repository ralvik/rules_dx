use super::*;

fn testdata_root() -> PathBuf {
    for candidate in [
        std::env::var("TEST_SRCDIR")
            .ok()
            .zip(std::env::var("TEST_WORKSPACE").ok())
            .map(|(root, ws)| PathBuf::from(root).join(ws).join("tools/depcheck/testdata")),
        std::env::var("RUNFILES_DIR")
            .ok()
            .map(|root| PathBuf::from(root).join("_main/tools/depcheck/testdata")),
        std::env::var("CARGO_MANIFEST_DIR")
            .ok()
            .map(|root| PathBuf::from(root).join("testdata")),
        Some(PathBuf::from("tools/depcheck/testdata")),
    ]
    .into_iter()
    .flatten()
    {
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("tools/depcheck/testdata")
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdirs");
    }
    std::fs::write(path, text).expect("write");
}

#[test]
fn normalizes_match_python() {
    assert_eq!(normalize_py("Foo-Bar.Baz"), "foo_bar_baz");
    assert_eq!(normalize_js("Foo-Bar"), "foo-bar");
    assert_eq!(normalize_rs("Foo-Bar"), "foo_bar");
    assert_eq!(normalize_go("Example.COM/Foo"), "example.com/foo");
    assert_eq!(normalize_jvm("Group:Artifact"), "group:artifact");
    assert_eq!(normalize_dotnet("Newtonsoft.Json"), "newtonsoft.json");
    assert_eq!(normalize_cc("Foo-Bar"), "foo_bar");
}

#[test]
fn satisfies_caret_and_exact() {
    assert!(satisfies("1", "1.0.0"));
    assert!(!satisfies("^1.2.3", "1.9.0"));
    assert!(satisfies("^1", "1.9.0"));
    assert!(!satisfies("1", "0.9.0"));
    assert!(satisfies("*", "9.9.9"));
    assert!(satisfies("", "1.0.0"));
    assert!(satisfies("==1.0.0", "1.0.0"));
    assert!(!satisfies("==1.0.0", "1.0.1"));
    assert!(satisfies("=1.0.0", "1.0.0"));
    assert!(satisfies("30.2.0", "30.2.0"));
    assert!(!satisfies("30.2.0", "30.3.0"));
}

#[test]
fn satisfies_tilde_and_range() {
    assert!(satisfies("~1.2.3", "1.2.9"));
    assert!(!satisfies("~1.2.3", "1.3.0"));
    assert!(satisfies(">=7", "8.0.0"));
    assert!(!satisfies(">=7", "6.0.0"));
    assert!(satisfies("v1.2.3", "1.2.3"));
    assert!(satisfies("1.2.3", "v1.2.3"));
}

#[test]
fn rust_manifest_covers_all_categories() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("Cargo.toml");
    write_file(
        &man,
        r#"
[dependencies]
anyhow = "1"
opt-feat = { version = "1", optional = true }

[dev-dependencies]
helper = "2"

[build-dependencies]
codegen = "3"

[target.'cfg(target_os = "windows")'.dependencies]
win-only = "1"
"#,
    );
    let deps = parse_rust_manifest(&man).expect("parse");
    assert_eq!(deps["anyhow"].category, "prod");
    assert!(deps["opt-feat"].optional);
    assert_eq!(deps["helper"].category, "dev");
    assert_eq!(deps["codegen"].category, "build");
    assert!(deps["win-only"].platform);
}

#[test]
fn rust_lock_parses_packages() {
    let dir = tempfile::tempdir().expect("scratch");
    let lock = dir.path().join("Cargo.lock");
    write_file(
        &lock,
        r#"
[[package]]
name = "anyhow"
version = "1.0.0"

[[package]]
name = "hello"
version = "0.0.0"
"#,
    );
    let pkgs = parse_rust_lock(&lock).expect("parse");
    assert_eq!(pkgs["anyhow"], "1.0.0");
}

#[test]
fn python_manifest_covers_markers_and_groups() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("pyproject.toml");
    write_file(
        &man,
        r#"
[project]
name = "hello"
dependencies = ["pytest>=7", "win-only; sys_platform=='win32'", "plain"]

[project.optional-dependencies]
extra = ["opt-dep==1.0"]

[dependency-groups]
dev = ["helper==2.0"]
"#,
    );
    let deps = parse_python_manifest(&man).expect("parse");
    assert_eq!(deps["pytest"].category, "prod");
    assert!(deps["win_only"].platform);
    assert!(deps["opt_dep"].optional);
    assert_eq!(deps["helper"].category, "dev");
}

#[test]
fn python_lock_parses_and_falls_back() {
    let dir = tempfile::tempdir().expect("scratch");
    let lock = dir.path().join("uv.lock");
    write_file(
        &lock,
        r#"
[[package]]
name = "pytest"
version = "8.0.0"
"#,
    );
    let pkgs = parse_python_lock(&lock).expect("parse");
    assert_eq!(pkgs["pytest"], "8.0.0");
}

#[test]
fn js_manifest_marks_peer_and_optional() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("package.json");
    write_file(
        &man,
        r#"{"dependencies": {"jest": "30.2.0"}, "devDependencies": {"dev": "1"}, "optionalDependencies": {"opt": "1"}, "peerDependencies": {"peer": "1"}}"#,
    );
    let deps = parse_js_manifest(&man).expect("parse");
    assert_eq!(deps["jest"].category, "prod");
    assert_eq!(deps["dev"].category, "dev");
    assert!(deps["opt"].optional);
    assert!(deps["peer"].peer);
}

#[test]
fn pnpm_lock_parses_scoped() {
    let dir = tempfile::tempdir().expect("scratch");
    let lock = dir.path().join("pnpm-lock.yaml");
    write_file(
        &lock,
        "packages:\n  'jest@30.2.0':\n    resolution: {integrity: sha512-x}\n  '@scope/name@1.2.3':\n    resolution: {integrity: sha512-y}\n",
    );
    let pkgs = parse_pnpm_lock(&lock).expect("parse");
    assert_eq!(pkgs["jest"], "30.2.0");
    assert_eq!(pkgs["@scope/name"], "1.2.3");
}

#[test]
fn go_manifest_covers_markers() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("go.mod");
    write_file(
        &man,
        "module example.com/hello\n\ngo 1.21\n\nrequire (\n\texample.com/greet v1.0.0\n\texample.com/dev v0.1.0 // depcheck:test\n\texample.com/opt v0.2.0 // optional\n)\n",
    );
    let deps = parse_go_manifest(&man).expect("parse");
    assert_eq!(deps["example.com/greet"].category, "prod");
    assert_eq!(deps["example.com/dev"].category, "dev");
    assert!(deps["example.com/opt"].optional);
}

#[test]
fn go_lock_strips_go_mod_suffix() {
    let dir = tempfile::tempdir().expect("scratch");
    let lock = dir.path().join("go.sum");
    write_file(
        &lock,
        "example.com/greet v1.0.0 h1:abc\nexample.com/greet v1.0.0/go.mod h1:def\n",
    );
    let pkgs = parse_go_lock(&lock).expect("parse");
    assert_eq!(pkgs["example.com/greet"], "v1.0.0");
}

#[test]
fn jvm_manifest_and_lock() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("jvm_deps.toml");
    write_file(
        &man,
        "[[dep]]\ngroup = \"example\"\nartifact = \"greet\"\nversion = \"1.0.0\"\nscope = \"compile\"\n",
    );
    let deps = parse_jvm_manifest(&man).expect("parse");
    assert_eq!(deps["example:greet"].category, "prod");
    let lock = dir.path().join("maven_install.json");
    write_file(
        &lock,
        r#"{"artifacts": {"example:greet": {"version": "1.0.0"}}}"#,
    );
    let pkgs = parse_jvm_lock(&lock).expect("parse");
    assert_eq!(pkgs["example:greet"], "1.0.0");
}

#[test]
fn dotnet_manifest_and_lock() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("paket.dependencies");
    write_file(&man, "source https://api.nuget.org/v3/index.json\nnuget Foo 1.0.0\ngroup Test\nnuget Bar 2.0.0\n");
    let deps = parse_dotnet_manifest(&man).expect("parse");
    assert_eq!(deps["foo"].category, "prod");
    assert_eq!(deps["bar"].category, "dev");
    let lock = dir.path().join("paket.lock");
    write_file(&lock, "NUGET\n  remote: x\n    Foo (1.0.0)\n");
    let pkgs = parse_dotnet_lock(&lock).expect("parse");
    assert_eq!(pkgs["foo"], "1.0.0");
}

#[test]
fn cc_manifest_and_lock() {
    let dir = tempfile::tempdir().expect("scratch");
    let man = dir.path().join("cc_deps.toml");
    write_file(
        &man,
        "[[dep]]\nname = \"greet\"\nversion = \"1.0.0\"\nsha256 = \"abc\"\nscope = \"prod\"\n",
    );
    let deps = parse_cc_manifest(&man).expect("parse");
    assert_eq!(deps["greet"].spec, "1.0.0");
    assert_eq!(deps["greet"].sha256, "abc");
    let lock = dir.path().join("cc_lock.json");
    write_file(
        &lock,
        r#"{"packages": {"greet": {"version": "1.0.0", "sha256": "abc"}}}"#,
    );
    let pkgs = parse_cc_lock(&lock).expect("parse");
    assert_eq!(pkgs["greet"], "1.0.0");
}

#[test]
fn exceptions_require_reason_shape() {
    let dir = tempfile::tempdir().expect("scratch");
    let exc = dir.path().join("depcheck_exceptions.toml");
    write_file(
        &exc,
        "[[exception]]\ndependency = \"build-plugin\"\nreason = \"codegen\"\n",
    );
    let parsed = parse_exceptions(Some(&exc)).expect("parse");
    assert_eq!(parsed["build_plugin"].reason, "codegen");
    assert!(parse_exceptions(None).expect("none").is_empty());
}

#[test]
fn test_file_detection_per_eco() {
    assert!(is_test_file(Ecosystem::Rust, Path::new("a/tests/foo.rs")));
    assert!(is_test_file(Ecosystem::Python, Path::new("a/test_foo.py")));
    assert!(!is_test_file(Ecosystem::Python, Path::new("a/hello.py")));
    assert!(is_test_file(Ecosystem::Go, Path::new("a/foo_test.go")));
    assert!(is_test_file(Ecosystem::Js, Path::new("a/foo.test.js")));
}

#[test]
fn find_usages_rust_and_python() {
    let dir = tempfile::tempdir().expect("scratch");
    write_file(&dir.path().join("src/lib.rs"), "use anyhow::Result;\n");
    write_file(&dir.path().join("src/main.py"), "import pytest\n");
    let rust_hits =
        find_usages(Ecosystem::Rust, dir.path(), &["anyhow".to_owned()]).expect("usages");
    assert!(rust_hits["anyhow"].src);
    let py_hits =
        find_usages(Ecosystem::Python, dir.path(), &["pytest".to_owned()]).expect("usages");
    assert!(py_hits["pytest"].src);
}

fn manifest_name(eco: Ecosystem) -> &'static str {
    match eco {
        Ecosystem::Rust => "Cargo.toml",
        Ecosystem::Python => "pyproject.toml",
        Ecosystem::Js | Ecosystem::Ts => "package.json",
        Ecosystem::Go => "go.mod",
        Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => "jvm_deps.toml",
        Ecosystem::Csharp | Ecosystem::Fsharp => "paket.dependencies",
        Ecosystem::Cc => "cc_deps.toml",
    }
}

fn lock_name(eco: Ecosystem) -> &'static str {
    match eco {
        Ecosystem::Rust => "Cargo.lock",
        Ecosystem::Python => "uv.lock",
        Ecosystem::Js | Ecosystem::Ts => "pnpm-lock.yaml",
        Ecosystem::Go => "go.sum",
        Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => "maven_install.json",
        Ecosystem::Csharp | Ecosystem::Fsharp => "paket.lock",
        Ecosystem::Cc => "cc_lock.json",
    }
}

fn eco_dir(eco: Ecosystem) -> &'static str {
    match eco {
        Ecosystem::Rust => "rust",
        Ecosystem::Python => "python",
        Ecosystem::Js => "js",
        Ecosystem::Ts => "ts",
        Ecosystem::Go => "go",
        Ecosystem::Java => "java",
        Ecosystem::Kotlin => "kotlin",
        Ecosystem::Scala => "scala",
        Ecosystem::Csharp => "csharp",
        Ecosystem::Fsharp => "fsharp",
        Ecosystem::Cc => "cc",
    }
}

fn consistency(eco: Ecosystem, case: &str) -> i32 {
    let root = testdata_root();
    let man = root.join(eco_dir(eco)).join(case).join(manifest_name(eco));
    let lock = root.join(eco_dir(eco)).join(case).join(lock_name(eco));
    let mut out = String::new();
    let mut err = String::new();
    cmd_consistency(eco, &man, &lock, &mut out, &mut err)
}

fn usage(eco: Ecosystem, case: &str, with_exc: bool) -> (i32, String) {
    let root = testdata_root();
    let base = root.join(eco_dir(eco)).join(case);
    let man = base.join(manifest_name(eco));
    let exc = base.join("depcheck_exceptions.toml");
    let exc_opt = if with_exc && exc.exists() {
        Some(exc)
    } else {
        None
    };
    let mut out = String::new();
    let mut err = String::new();
    let code = cmd_usage(eco, &man, &base, exc_opt.as_deref(), &mut out, &mut err);
    (code, err)
}

#[test]
fn parity_consistency_truth_table() {
    for eco in [
        Ecosystem::Rust,
        Ecosystem::Python,
        Ecosystem::Js,
        Ecosystem::Ts,
        Ecosystem::Go,
        Ecosystem::Java,
        Ecosystem::Kotlin,
        Ecosystem::Scala,
        Ecosystem::Csharp,
        Ecosystem::Fsharp,
        Ecosystem::Cc,
    ] {
        assert_eq!(consistency(eco, "ok_used"), 0, "{:?} ok_used", eco);
        assert_eq!(consistency(eco, "stale"), 1, "{:?} stale", eco);
        assert_eq!(consistency(eco, "unused"), 0, "{:?} unused", eco);
        assert_eq!(
            consistency(eco, "transitive_shared"),
            0,
            "{:?} transitive",
            eco
        );
        assert_eq!(consistency(eco, "exception"), 0, "{:?} exception", eco);
        assert_eq!(consistency(eco, "category"), 0, "{:?} category", eco);
        assert_eq!(consistency(eco, "category_ok"), 0, "{:?} category_ok", eco);
        assert_eq!(
            consistency(eco, "platform_optional"),
            0,
            "{:?} platform",
            eco
        );
    }
}

#[test]
fn parity_consistency_missing_lock_is_actionable() {
    let root = testdata_root();
    let man = root.join("rust/ok_used/Cargo.toml");
    let lock = root.join("rust/ok_used/MISSING.lock");
    let mut out = String::new();
    let mut err = String::new();
    let code = cmd_consistency(Ecosystem::Rust, &man, &lock, &mut out, &mut err);
    assert_eq!(code, 2);
}

#[test]
fn parity_usage_truth_table() {
    for eco in [
        Ecosystem::Rust,
        Ecosystem::Python,
        Ecosystem::Js,
        Ecosystem::Ts,
        Ecosystem::Go,
        Ecosystem::Java,
        Ecosystem::Kotlin,
        Ecosystem::Scala,
        Ecosystem::Csharp,
        Ecosystem::Fsharp,
        Ecosystem::Cc,
    ] {
        let (ok_code, _) = usage(eco, "ok_used", false);
        assert_eq!(ok_code, 0, "{:?} ok_used usage", eco);
        let (stale_code, _) = usage(eco, "stale", false);
        assert_eq!(stale_code, 0, "{:?} stale usage", eco);
        let (unused_code, _) = usage(eco, "unused", false);
        assert_eq!(unused_code, 1, "{:?} unused usage", eco);
        let (trans_code, _) = usage(eco, "transitive_shared", false);
        assert_eq!(trans_code, 0, "{:?} transitive usage", eco);
        let (exc_code, _) = usage(eco, "exception", true);
        assert_eq!(exc_code, 0, "{:?} exception usage", eco);
        let (obs_code, _) = usage(eco, "obsolete", true);
        assert_eq!(obs_code, 1, "{:?} obsolete usage", eco);
        let (plat_code, _) = usage(eco, "platform_optional", false);
        assert_eq!(plat_code, 0, "{:?} platform usage", eco);
        let (plat_unused_code, _) = usage(eco, "platform_optional_unused", false);
        assert_eq!(plat_unused_code, 1, "{:?} platform unused", eco);
        let (cat_code, cat_err) = usage(eco, "category", false);
        assert_eq!(cat_code, 1, "{:?} category usage", eco);
        assert!(
            cat_err.contains("category error"),
            "{:?} category error text: {cat_err}",
            eco
        );
        let (cat_ok_code, _) = usage(eco, "category_ok", false);
        assert_eq!(cat_ok_code, 0, "{:?} category_ok usage", eco);
    }
}

#[test]
fn parity_usage_missing_reason_and_obsolete() {
    let root = testdata_root();
    let base = root.join("rust/exception");
    let man = base.join("Cargo.toml");
    let dir = tempfile::tempdir().expect("scratch");
    let exc = dir.path().join("depcheck_exceptions.toml");
    write_file(
        &exc,
        "[[exception]]\ndependency = \"build-plugin\"\nreason = \"\"\n",
    );
    let mut out = String::new();
    let mut err = String::new();
    let code = cmd_usage(Ecosystem::Rust, &man, &base, Some(&exc), &mut out, &mut err);
    assert_eq!(code, 1);
}

#[test]
fn cc_missing_sha256_fails() {
    let dir = tempfile::tempdir().expect("scratch");
    write_file(
        &dir.path().join("cc_deps.toml"),
        "[[dep]]\nname = \"greet\"\nversion = \"1.0.0\"\nscope = \"prod\"\n",
    );
    write_file(
        &dir.path().join("cc_lock.json"),
        r#"{"packages": {"greet": {"version": "1.0.0", "sha256": "abc"}}}"#,
    );
    let mut out = String::new();
    let mut err = String::new();
    let code = cmd_consistency(
        Ecosystem::Cc,
        &dir.path().join("cc_deps.toml"),
        &dir.path().join("cc_lock.json"),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 1);
    assert!(err.contains("missing sha256"));
}

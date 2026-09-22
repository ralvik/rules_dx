//! Bump widen planning tests (split from `request.rs`).
//! Originally the inline `mod tests` of `request.rs`.

use super::*;

#[test]
fn parses_single_requirement_shapes() {
    let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
    assert_eq!(bump.set, BumpSet::Cargo);
    assert_eq!(bump.package, "anyhow");
    assert_eq!(
        bump.target_manifest(),
        "rust/tests/fixtures/hello/Cargo.toml"
    );
    assert!(bump.needs_update_refresh());

    let bump = BumpRequest::parse("npm:react", "1.2.3").expect("npm");
    assert_eq!(bump.target_manifest(), "package.json");

    let bump = BumpRequest::parse("npm:@astrojs/compiler", "1.2.3").expect("scoped");
    assert_eq!(bump.package, "@astrojs/compiler");

    let bump = BumpRequest::parse("bazel:rules_rust", "0.74.0").expect("bazel module");
    assert_eq!(bump.target_manifest(), "MODULE.bazel");
    assert!(!bump.needs_update_refresh());

    let bump = BumpRequest::parse("bazel:.bazelversion", "9.2.0").expect("bazelversion");
    assert_eq!(bump.target_manifest(), ".bazelversion");

    let bump = BumpRequest::parse("github-actions:actions/checkout", "v4").expect("gha");
    assert_eq!(bump.set, BumpSet::GithubActions);
    assert!(!bump.needs_update_refresh());

    let bump = BumpRequest::parse("go:example.com/mod", "1.2.3").expect("go");
    assert_eq!(bump.target_manifest(), "third_party/go/go.mod");

    let bump = BumpRequest::parse("maven:junit:junit", "4.13.2").expect("maven");
    assert_eq!(bump.set, BumpSet::Maven);
    assert_eq!(bump.package, "junit:junit");
    assert_eq!(bump.target_manifest(), "MODULE.bazel");
    assert!(bump.needs_update_refresh());

    let bump = BumpRequest::parse("maven:org.junit.jupiter:junit-jupiter-api", "6.1.3")
        .expect("maven jupiter");
    assert_eq!(bump.package, "org.junit.jupiter:junit-jupiter-api");

    let bump = BumpRequest::parse("nuget:FSharp.Core", "10.1.201").expect("nuget");
    assert_eq!(bump.set, BumpSet::NuGet);
    assert_eq!(
        bump.target_manifest(),
        "third_party/dotnet/paket.dependencies"
    );
    assert!(bump.needs_update_refresh());
}

#[test]
fn aliases_resolve_to_canonical_sets() {
    let bump = BumpRequest::parse("gomod:example.com/mod", "1.2.3").expect("gomod");
    assert_eq!(bump.set, BumpSet::Go);
    let bump = BumpRequest::parse("gha:actions/checkout", "v4").expect("gha");
    assert_eq!(bump.set, BumpSet::GithubActions);
}

#[test]
fn widen_owns_rewrite_while_update_never_does() {
    assert!(BumpRequest::may_be_rewritten());
}

#[test]
fn bare_sets_targets_and_unknown_fail_closed() {
    assert!(matches!(
        BumpRequest::parse("cargo", "1.2.3"),
        Err(BumpError::BareSet { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("//rust/tests/fixtures/hello:hello", "1.2.3"),
        Err(BumpError::NotAPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("rust/tests/fixtures/hello/Cargo.toml", "1.2.3"),
        Err(BumpError::NotAPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("crates", "1.2.3"),
        Err(BumpError::UnknownSelector { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("cargo:", "1.2.3"),
        Err(BumpError::InvalidPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("", "1.2.3"),
        Err(BumpError::Empty)
    ));
    assert!(matches!(
        BumpRequest::parse("cargo:anyhow", ""),
        Err(BumpError::Empty)
    ));
    assert!(matches!(
        BumpRequest::parse("cargo:bad name", "1.2.3"),
        Err(BumpError::InvalidPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("maven", "1.2.3"),
        Err(BumpError::BareSet { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("nuget", "10.1.201"),
        Err(BumpError::BareSet { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("maven:junit", "1.2.3"),
        Err(BumpError::InvalidPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("maven::artifact", "1.2.3"),
        Err(BumpError::InvalidPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("maven:junit:junit:extra", "1.2.3"),
        Err(BumpError::InvalidPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("nuget:", "10.1.201"),
        Err(BumpError::InvalidPackage { .. })
    ));
    assert!(matches!(
        BumpRequest::parse("cargo:anyhow:extra", "1.2.3"),
        Err(BumpError::InvalidPackage { .. })
    ));
}

#[test]
fn invalid_versions_fail_without_widen() {
    assert!(matches!(
        BumpRequest::parse("cargo:anyhow", "not-a-version!!!"),
        Err(BumpError::Version(_))
    ));
    assert!(matches!(
        BumpRequest::parse("github-actions:actions/checkout", "bad tag"),
        Err(BumpError::Version(_))
    ));
    assert!(matches!(
        BumpRequest::parse("maven:junit:junit", "not-a-version!!!"),
        Err(BumpError::Version(_))
    ));
    assert!(matches!(
        BumpRequest::parse("nuget:FSharp.Core", "not-a-version!!!"),
        Err(BumpError::Version(_))
    ));
}

#[test]
fn summary_names_selector_version_and_next_step() {
    let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
    let summary = bump.summary();
    assert!(summary.contains("cargo:anyhow"), "{summary}");
    assert!(summary.contains("1.2.3"), "{summary}");
    assert!(summary.contains("dx update cargo"), "{summary}");
    assert!(summary.contains("automatically"), "{summary}");
    let bump = BumpRequest::parse("bazel:rules_rust", "0.74.0").expect("bazel");
    assert!(bump.summary().contains("flag-diff"), "{summary}");
    let bump = BumpRequest::parse("maven:junit:junit", "4.13.3").expect("maven");
    assert!(bump.summary().contains("dx update maven"), "{summary}");
    assert!(bump.summary().contains("automatically"), "{summary}");
    let bump = BumpRequest::parse("nuget:FSharp.Core", "10.1.202").expect("nuget");
    assert!(bump.summary().contains("dx update nuget"), "{summary}");
}

#[test]
fn summary_carries_major_bump_migrate_hint_for_semver() {
    // Issue #931 (See: `docs/cli/commands/migrate.md`): semver plans print
    // the missing-manifest hint with exit mapping; Git shapes never hint.
    let bump = BumpRequest::parse("cargo:anyhow", "2.0.0").expect("cargo major");
    let summary = bump.summary();
    assert!(summary.contains("major bump"), "{summary}");
    assert!(summary.contains("dx migrate --from"), "{summary}");
    assert!(summary.contains("migrate_failed"), "{summary}");
    assert!(summary.contains("missing-versions"), "{summary}");
    let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
    let bump = BumpRequest::parse("github-actions:actions/checkout", sha).expect("gha");
    assert!(!bump.summary().contains("major bump"), "{}", bump.summary());
}

#[test]
fn major_bump_hint_needs_old_major_crossing() {
    // Issue #931: major hint fires only when new major exceeds old major.
    let bump = BumpRequest::parse("cargo:anyhow", "2.0.0").expect("major");
    let hint = bump.major_bump_hint("1.2.3").expect("hint");
    assert!(hint.contains("major bump 1.2.3 -> 2.0.0"), "{hint}");
    assert!(hint.contains("migrate-v1-to-v2.json"), "{hint}");
    assert!(hint.contains("migrate_failed"), "{hint}");
    assert!(hint.contains("missing-versions"), "{hint}");
    let bump = BumpRequest::parse("cargo:anyhow", "1.3.0").expect("minor");
    assert!(bump.major_bump_hint("1.2.3").is_none());
    let bump = BumpRequest::parse("npm:jest", "30.3.0").expect("npm minor");
    assert!(bump.major_bump_hint("30.2.0").is_none());
    let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
    let bump = BumpRequest::parse("github-actions:actions/checkout", sha).expect("gha");
    assert!(bump.major_bump_hint("v4").is_none());
}

#[test]
fn refresh_selector_chains_automatically_per_set() {
    // Issue #638: Cargo full, npm selective, Go noop, Maven full, NuGet
    // full; file-only empty.
    let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
    assert_eq!(bump.refresh_selector(), "cargo");
    assert!(bump.needs_update_refresh());
    let bump = BumpRequest::parse("npm:jest", "30.3.0").expect("npm");
    assert_eq!(bump.refresh_selector(), "npm:jest");
    assert!(bump.needs_update_refresh());
    let bump = BumpRequest::parse("npm:@astrojs/compiler", "1.2.3").expect("scoped");
    assert_eq!(bump.refresh_selector(), "npm:@astrojs/compiler");
    let bump = BumpRequest::parse("go:example.com/mod", "1.2.3").expect("go");
    assert_eq!(bump.refresh_selector(), "go");
    assert!(bump.needs_update_refresh());
    let bump = BumpRequest::parse("maven:junit:junit", "4.13.2").expect("maven");
    assert_eq!(bump.refresh_selector(), "maven");
    assert!(bump.needs_update_refresh());
    let bump = BumpRequest::parse("nuget:FSharp.Core", "10.1.201").expect("nuget");
    assert_eq!(bump.refresh_selector(), "nuget");
    assert!(bump.needs_update_refresh());
    let bump = BumpRequest::parse("bazel:rules_rust", "0.74.0").expect("bazel");
    assert_eq!(bump.refresh_selector(), "");
    assert!(!bump.needs_update_refresh());
    let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
    let bump = BumpRequest::parse("github-actions:actions/checkout", sha).expect("gha");
    assert_eq!(bump.refresh_selector(), "");
    assert!(!bump.needs_update_refresh());
}

#[test]
fn edits_rewrite_exactly_one_requirement() {
    // `.bazelversion`: single-line replace, newline preserved.
    let bump = BumpRequest::parse("bazel:.bazelversion", "9.3.0").expect("bazelversion");
    assert_eq!(bump.plan_edit("9.2.0\n").expect("edit"), "9.3.0\n");
    assert_eq!(bump.plan_edit("9.2.0").expect("edit"), "9.3.0");
    assert!(matches!(
        bump.plan_edit("9.2.0\n9.3.0\n"),
        Err(BumpError::UnsupportedManifest { .. })
    ));

    // `MODULE.bazel`: one `bazel_dep` line rewrites, others preserved.
    let module = "bazel_dep(name = \"rules_rust\", version = \"0.74.0\")\n\
                  bazel_dep(name = \"gazelle\", version = \"0.52.2\")\n";
    let bump = BumpRequest::parse("bazel:rules_rust", "0.75.0").expect("module");
    let widened = bump.plan_edit(module).expect("edit");
    assert!(
        widened.contains("name = \"rules_rust\", version = \"0.75.0\""),
        "{widened}"
    );
    assert!(
        widened.contains("name = \"gazelle\", version = \"0.52.2\""),
        "{widened}"
    );
    assert!(matches!(
        bump.plan_edit("bazel_dep(name = \"other\", version = \"1.0.0\")\n"),
        Err(BumpError::NotFound { .. })
    ));

    // `Cargo.toml`: `package = "old"` rewrites once.
    let cargo = "[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n";
    let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
    let widened = bump.plan_edit(cargo).expect("edit");
    assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
    assert!(widened.contains("serde = \"1\""), "{widened}");

    // `package.json`: `"package": "old"` rewrites once, formatting kept.
    let npm =
        "{\n  \"devDependencies\": {\n    \"jest\": \"30.2.0\",\n    \"vue\": \"3.5.42\"\n  }\n}\n";
    let bump = BumpRequest::parse("npm:jest", "30.3.0").expect("npm");
    let widened = bump.plan_edit(npm).expect("edit");
    assert!(widened.contains("\"jest\": \"30.3.0\""), "{widened}");
    assert!(widened.contains("\"vue\": \"3.5.42\""), "{widened}");
    assert!(matches!(
        bump.plan_edit("{\n}\n"),
        Err(BumpError::NotFound { .. })
    ));

    // `go.mod`: `require <mod> vX` rewrites once.
    let gomod = "module example.com/root\n\nrequire example.com/mod v1.2.3\n";
    let bump = BumpRequest::parse("go:example.com/mod", "1.3.0").expect("go");
    let widened = bump.plan_edit(gomod).expect("edit");
    assert!(widened.contains("example.com/mod v1.3.0"), "{widened}");

    // `MODULE.bazel` Maven artifacts: one `group:artifact:version`
    // rewrites, others preserved.
    let module = "maven.install(\n    artifacts = [\n        \"junit:junit:4.13.2\",\n        \"org.junit.jupiter:junit-jupiter-api:6.1.3\",\n    ],\n)\n";
    let bump = BumpRequest::parse("maven:junit:junit", "4.13.3").expect("maven");
    let widened = bump.plan_edit(module).expect("edit");
    assert!(widened.contains("\"junit:junit:4.13.3\""), "{widened}");
    assert!(
        widened.contains("\"org.junit.jupiter:junit-jupiter-api:6.1.3\""),
        "{widened}"
    );
    assert!(matches!(
        bump.plan_edit("maven.install(\n    artifacts = [\n    ],\n)\n"),
        Err(BumpError::NotFound { .. })
    ));

    // `paket.dependencies`: one `nuget <id> <version>` rewrites.
    let paket = "source https://api.nuget.org/v3/index.json\nframework: net10.0\n\nnuget FSharp.Core 10.1.201\nnuget xunit.v3 4.0.0\n";
    let bump = BumpRequest::parse("nuget:FSharp.Core", "10.1.202").expect("nuget");
    let widened = bump.plan_edit(paket).expect("edit");
    assert!(widened.contains("nuget FSharp.Core 10.1.202"), "{widened}");
    assert!(widened.contains("nuget xunit.v3 4.0.0"), "{widened}");
    assert!(matches!(
        bump.plan_edit("source https://api.nuget.org/v3/index.json\n"),
        Err(BumpError::NotFound { .. })
    ));
}

#[test]
fn maven_and_nuget_edits_fail_closed_on_ambiguous_and_prefixes() {
    // Duplicate Maven artifact lines are ambiguous (never batch).
    let module = "        \"junit:junit:4.13.2\",\n        \"junit:junit:4.13.2\",\n";
    let bump = BumpRequest::parse("maven:junit:junit", "4.13.3").expect("maven");
    assert!(matches!(
        bump.plan_edit(module),
        Err(BumpError::Ambiguous { count: 2, .. })
    ));
    // `junit:junit` never matches `junit:junit-jupiter` prefixes.
    let module = "        \"junit:junit-jupiter:1.0.0\",\n";
    assert!(matches!(
        bump.plan_edit(module),
        Err(BumpError::NotFound { .. })
    ));
    // Duplicate paket lines are ambiguous (never batch).
    let paket = "nuget FSharp.Core 10.1.201\nnuget FSharp.Core 10.1.201\n";
    let bump = BumpRequest::parse("nuget:FSharp.Core", "10.1.202").expect("nuget");
    assert!(matches!(
        bump.plan_edit(paket),
        Err(BumpError::Ambiguous { count: 2, .. })
    ));
    // `xunit.v3` never matches `xunit.v3.assert` prefixes.
    let paket = "nuget xunit.v3.assert 4.0.0\n";
    let bump = BumpRequest::parse("nuget:xunit.v3", "4.0.1").expect("nuget prefix");
    assert!(matches!(
        bump.plan_edit(paket),
        Err(BumpError::NotFound { .. })
    ));
}

#[test]
fn cargo_toml_preserves_format_and_fails_closed() {
    let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");

    // Inline table keeps sibling keys, comments, and order.
    let cargo = "[dependencies]\nanyhow = { version = \"1\", features = [\"derive\"] } # keep\nserde = \"1\"\n";
    let widened = bump.plan_edit(cargo).expect("inline");
    assert!(
        widened.contains("anyhow = { version = \"1.2.3\""),
        "{widened}"
    );
    assert!(widened.contains("features = [\"derive\"]"), "{widened}");
    assert!(widened.contains("# keep"), "{widened}");
    assert!(widened.contains("serde = \"1\""), "{widened}");

    // `[dependencies.package]` table form widens `version`.
    let cargo = "[dependencies.anyhow]\nversion = \"1\"\nfeatures = [\"derive\"]\n";
    let widened = bump.plan_edit(cargo).expect("table");
    assert!(widened.contains("version = \"1.2.3\""), "{widened}");
    assert!(widened.contains("features ="), "{widened}");

    // Single `dev-dependencies` entry widens.
    let cargo = "[dev-dependencies]\nanyhow = \"1\"\n";
    let widened = bump.plan_edit(cargo).expect("dev");
    assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");

    // Same crate in two tables is ambiguous (never batch).
    let cargo = "[dependencies]\nanyhow = \"1\"\n[dev-dependencies]\nanyhow = \"1\"\n";
    assert!(matches!(
        bump.plan_edit(cargo),
        Err(BumpError::Ambiguous { count: 2, .. })
    ));

    // Git/path shapes fail closed as unsupported.
    let cargo = "[dependencies]\nanyhow = { git = \"https://example.com/repo\", tag = \"v1\" }\n";
    assert!(matches!(
        bump.plan_edit(cargo),
        Err(BumpError::UnsupportedManifest { .. })
    ));
    let cargo = "[dependencies]\nanyhow = { path = \"../anyhow\" }\n";
    assert!(matches!(
        bump.plan_edit(cargo),
        Err(BumpError::UnsupportedManifest { .. })
    ));

    // Workspace inheritance and missing versions fail closed.
    let cargo = "[dependencies]\nanyhow = { workspace = true }\n";
    assert!(matches!(
        bump.plan_edit(cargo),
        Err(BumpError::UnsupportedManifest { .. })
    ));
    let cargo = "[dependencies]\nanyhow = { optional = true }\n";
    assert!(matches!(
        bump.plan_edit(cargo),
        Err(BumpError::UnsupportedManifest { .. })
    ));

    // Invalid TOML and missing deps fail closed without widening.
    assert!(matches!(
        bump.plan_edit("[dependencies\nanyhow = "),
        Err(BumpError::UnsupportedManifest { .. })
    ));
    assert!(matches!(
        bump.plan_edit("[dependencies]\nserde = \"1\"\n"),
        Err(BumpError::NotFound { .. })
    ));
}

#[test]
fn github_actions_needs_sha_for_tags() {
    let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let workflow = format!("      - uses: actions/checkout@{old} # v7\n      - uses: actions/cache@bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb # v6\n", old = "3d3c42e5aac5ba805825da76410c181273ba90b1");
    let bump = BumpRequest::parse("github-actions:actions/checkout", &sha).expect("sha");
    let widened = bump.plan_edit(&workflow).expect("edit");
    assert!(
        widened.contains(&format!("actions/checkout@{sha}")),
        "{widened}"
    );
    assert!(widened.contains("actions/cache@bbbb"), "{widened}");

    let tag = BumpRequest::parse("github-actions:actions/checkout", "v5").expect("tag");
    assert!(matches!(
        tag.plan_edit(&workflow),
        Err(BumpError::NeedsSha { .. })
    ));
}

#[test]
fn regex_go_module_token_respects_boundaries() {
    // `example.com/mod-extra` must not count as `example.com/mod`.
    assert!(line_contains_module_token(
        "require example.com/mod v1.2.3",
        "example.com/mod"
    ));
    assert!(!line_contains_module_token(
        "require example.com/mod-extra v1.2.3",
        "example.com/mod"
    ));
    assert!(!line_contains_module_token(
        "require example.com/modx v1.2.3",
        "example.com/mod"
    ));
}

#[test]
fn regex_go_version_token_replaces_last_and_keeps_suffix() {
    // Last `v` token wins; prerelease suffix is replaced wholesale.
    let line = "require example.com/mod v1.2.3 // keep\n";
    let replaced = replace_go_version_token(line, "v1.3.0").expect("replace");
    assert!(replaced.contains("v1.3.0"), "{replaced}");
    assert!(replaced.contains("// keep"), "{replaced}");

    let pre = "require example.com/mod v1.2.3-alpha+001\n";
    let replaced = replace_go_version_token(pre, "v1.3.0").expect("pre");
    assert!(replaced.contains("v1.3.0"), "{replaced}");
    assert!(!replaced.contains("alpha"), "{replaced}");

    // Bare `v1` (no dot) is not a version token.
    assert!(replace_go_version_token("require example.com/mod v1\n", "v2.0.0").is_none());
}

#[test]
fn regex_cargo_boundary_prefers_exact_table_key() {
    // `my-anyhow` must not widen when asking for `anyhow`.
    let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
    let cargo = "[dependencies]\nmy-anyhow = \"1\"\n";
    assert!(matches!(
        bump.plan_edit(cargo),
        Err(BumpError::NotFound { .. })
    ));
    let cargo = "[dependencies]\nmy-anyhow = \"1\"\nanyhow = \"1\"\n";
    let widened = bump.plan_edit(cargo).expect("exact");
    assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
    assert!(widened.contains("my-anyhow = \"1\""), "{widened}");
}

#[test]
fn regex_version_attr_and_json_keep_spacing() {
    let tight = "bazel_dep(name = \"rules_rust\",version=\"0.74.0\")\n";
    let replaced = replace_version_attr(tight, "0.75.0").expect("tight");
    assert!(replaced.contains("version=\"0.75.0\""), "{replaced}");

    let spaced = "bazel_dep(name = \"rules_rust\", version   =   \"0.74.0\")\n";
    let replaced = replace_version_attr(spaced, "0.75.0").expect("spaced");
    assert!(replaced.contains("version   =   \"0.75.0\""), "{replaced}");

    let line = "    \"jest\": \"30.2.0\",";
    let replaced = replace_first_quoted_version_after_colon(line, "30.3.0").expect("json");
    assert!(replaced.contains("\"jest\": \"30.3.0\""), "{replaced}");
}

//! V1 widen-one-requirement set registry for `dx bump`.
//!
//! Pure registry over the seven v1 manager sets named in the issue,
//! covering the native updater scope: Bazel modules plus
//! `.bazelversion`, Cargo, npm/pnpm (both lock graphs), Go (`gomod`),
//! GitHub Actions, Maven (`group:artifact` in `MODULE.bazel`), and NuGet
//! (`paket.dependencies`). Each set owns its declared-requirement
//! manifests; lock refresh stays resolver-owned through `dx update`
//! (`dx_update::backend`) for Cargo/npm/Go/Maven/NuGet, while Bazel and
//! GitHub Actions are file-only (verified through
//! `preset.update --verify-only` plus `bazel build //...`).
//!
//! Set identity, manifests, and locks are pinned here so selector
//! resolution, widen-edit planning, and per-set reporting agree on one
//! source of truth without a CLI filesystem scan. This registry never
//! fetches registries, compares versions, or resolves locks: registry
//! discovery and version comparison use upstream libraries (`semver` for
//! version parsing, `serde_json`/`toml` for manifest shapes, upstream
//! resolver backends for lock refresh), never custom HTTP/version/solver
//! code. Custom code is limited to the thin single-requirement edit
//! planned in [`crate::request`].
//!
//! Independence: widen edits exactly one declared requirement per
//! invocation (never batch). Sets sharing a lockfile or resolver
//! workspace must never be treated as independent merely because they
//! have different labels (see `dx_update::outcome` for the update half).

/// V1 widen set identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BumpSet {
    /// Bazel modules plus `.bazelversion` (`.bazelversion`, `MODULE.bazel`).
    Bazel,
    /// Rust/Cargo (`rust/tests/fixtures/hello/Cargo.toml`).
    Cargo,
    /// GitHub Actions (`.github/workflows/*.yml`, SHA-plus-tag pins).
    GithubActions,
    /// Go (`third_party/go/go.mod` via `go_deps.from_file`).
    Go,
    /// JVM/Maven (`MODULE.bazel` `maven.install` artifacts).
    Maven,
    /// JS/TS/npm (`package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`).
    Npm,
    /// .NET/NuGet (`third_party/dotnet/paket.dependencies`).
    NuGet,
}

impl BumpSet {
    /// All seven v1 sets, in deterministic alphabetical order.
    pub const ALL: [BumpSet; 7] = [
        BumpSet::Bazel,
        BumpSet::Cargo,
        BumpSet::GithubActions,
        BumpSet::Go,
        BumpSet::Maven,
        BumpSet::Npm,
        BumpSet::NuGet,
    ];

    /// Stable selector spelling for this set (with `go` covering the
    /// `gomod` spelling).
    pub fn name(self) -> &'static str {
        match self {
            BumpSet::Bazel => "bazel",
            BumpSet::Cargo => "cargo",
            BumpSet::GithubActions => "github-actions",
            BumpSet::Go => "go",
            BumpSet::Maven => "maven",
            BumpSet::Npm => "npm",
            BumpSet::NuGet => "nuget",
        }
    }

    /// Parses a set selector spelling. Case-sensitive; no aliases except
    /// `gomod` for `go` and `gha` for
    /// `github-actions` (workflow shorthand).
    pub fn parse(text: &str) -> Option<BumpSet> {
        match text {
            "bazel" => Some(BumpSet::Bazel),
            "cargo" => Some(BumpSet::Cargo),
            "github-actions" | "gha" => Some(BumpSet::GithubActions),
            "go" | "gomod" => Some(BumpSet::Go),
            "maven" => Some(BumpSet::Maven),
            "npm" => Some(BumpSet::Npm),
            "nuget" => Some(BumpSet::NuGet),
            _ => None,
        }
    }

    /// Canonical name for reporting (always the full spelling, never an
    /// alias: `gomod` reports as `go`, `gha` as `github-actions`).
    pub fn canonical_alias(text: &str) -> Option<&'static str> {
        Self::parse(text).map(|set| set.name())
    }

    /// Workspace-relative manifests owning declared requirements for this
    /// set. Verbatim paths; the widen edit touches exactly one requirement
    /// in one of these files per invocation.
    pub fn manifests(self) -> &'static [&'static str] {
        match self {
            BumpSet::Bazel => &[".bazelversion", "MODULE.bazel"],
            BumpSet::Cargo => &["rust/tests/fixtures/hello/Cargo.toml"],
            BumpSet::GithubActions => &[".github/workflows/ci.yml"],
            BumpSet::Go => &["third_party/go/go.mod"],
            BumpSet::Maven => &["MODULE.bazel"],
            BumpSet::Npm => &["package.json"],
            BumpSet::NuGet => &["third_party/dotnet/paket.dependencies"],
        }
    }

    /// Workspace-relative lockfiles refreshed resolver-owned after the
    /// widen edit (via `dx update <set>`). Empty means file-only (Bazel,
    /// GitHub Actions): verification runs `preset.update --verify-only`
    /// plus `bazel build //...` with no resolver refresh.
    pub fn locks(self) -> &'static [&'static str] {
        match self {
            BumpSet::Bazel => &[],
            BumpSet::Cargo => &[
                "rust/tests/fixtures/hello/Cargo.lock",
                "cargo-bazel-lock.json",
            ],
            BumpSet::GithubActions => &[],
            BumpSet::Go => &["third_party/go/go.mod", "third_party/go/go.sum"],
            BumpSet::Maven => &["third_party/jvm/maven_install.json"],
            BumpSet::Npm => &["pnpm-lock.yaml"],
            BumpSet::NuGet => &["third_party/dotnet/paket.lock", "third_party/dotnet/deps"],
        }
    }

    /// Whether lock refresh runs resolver-owned after widening (Cargo,
    /// npm, Go, Maven, NuGet) or the set is file-only (Bazel, GitHub
    /// Actions).
    pub fn needs_update_refresh(self) -> bool {
        !self.locks().is_empty()
    }

    /// Human updater description (never argv; argv lives in
    /// `dx_update::backend` for resolver sets and in the widen loop docs
    /// for file-only sets).
    pub fn updater(self) -> &'static str {
        match self {
            BumpSet::Bazel => {
                "file-only (edit .bazelversion or MODULE.bazel pin, then preset flag-diff review)"
            }
            BumpSet::Cargo => {
                "crate_universe repin (CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello)"
            }
            BumpSet::GithubActions => {
                "file-only (edit workflow SHA-plus-tag pin via upstream GitHub releases client)"
            }
            BumpSet::Go => {
                "pinned go_deps.from_file module lock (pins track Gazelle; explicit widen via `dx bump` plus the pinned SDK tidy)"
            }
            BumpSet::Maven => "rules_jvm_external pin (REPIN=1 bazel run @maven//:pin)",
            BumpSet::Npm => "Bazel-pinned pnpm update (bazel run @pnpm//:pnpm -- update)",
            BumpSet::NuGet => {
                "paket2bazel regeneration (bazel run @rules_dotnet//tools/paket2bazel -- ...)"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn all_sets_are_distinct_and_parse_round_trip() {
        let mut seen = BTreeSet::new();
        for set in BumpSet::ALL {
            assert!(seen.insert(set.name()), "duplicate set name");
            assert_eq!(BumpSet::parse(set.name()), Some(set));
        }
        assert_eq!(seen.len(), 7);
        assert_eq!(BumpSet::parse("Cargo"), None);
        assert_eq!(BumpSet::parse("cargo-lock"), None);
        assert_eq!(BumpSet::parse(""), None);
    }

    #[test]
    fn aliases_canonicalize_without_new_sets() {
        assert_eq!(BumpSet::parse("gomod"), Some(BumpSet::Go));
        assert_eq!(BumpSet::parse("gha"), Some(BumpSet::GithubActions));
        assert_eq!(BumpSet::canonical_alias("gomod"), Some("go"));
        assert_eq!(BumpSet::canonical_alias("gha"), Some("github-actions"));
        assert_eq!(BumpSet::canonical_alias("cargo"), Some("cargo"));
        assert_eq!(BumpSet::canonical_alias("unknown"), None);
    }

    #[test]
    fn order_is_alphabetical_and_deterministic() {
        let names: Vec<&str> = BumpSet::ALL.iter().map(|set| set.name()).collect();
        assert_eq!(
            names,
            vec![
                "bazel",
                "cargo",
                "github-actions",
                "go",
                "maven",
                "npm",
                "nuget"
            ]
        );
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn no_set_owns_a_dx_lockfile() {
        for set in BumpSet::ALL {
            for path in set.manifests().iter().chain(set.locks().iter()) {
                assert!(
                    !path.contains("dx.lock") && !path.contains(".dx-lock"),
                    "{set:?} must not own a dx lockfile: {path}"
                );
            }
        }
    }

    #[test]
    fn resolver_sets_need_refresh_while_file_only_do_not() {
        assert!(!BumpSet::Bazel.needs_update_refresh());
        assert!(!BumpSet::GithubActions.needs_update_refresh());
        assert!(BumpSet::Cargo.needs_update_refresh());
        assert!(BumpSet::Npm.needs_update_refresh());
        assert!(BumpSet::Go.needs_update_refresh());
        assert!(BumpSet::Maven.needs_update_refresh());
        assert!(BumpSet::NuGet.needs_update_refresh());
        assert!(BumpSet::Bazel.locks().is_empty());
        assert!(BumpSet::GithubActions.locks().is_empty());
        assert!(!BumpSet::Cargo.locks().is_empty());
        assert!(!BumpSet::Maven.locks().is_empty());
        assert!(!BumpSet::NuGet.locks().is_empty());
    }

    #[test]
    fn manifests_cover_the_v1_manager_set() {
        assert!(BumpSet::Bazel.manifests().contains(&".bazelversion"));
        assert!(BumpSet::Bazel.manifests().contains(&"MODULE.bazel"));
        assert_eq!(
            BumpSet::Cargo.manifests(),
            &["rust/tests/fixtures/hello/Cargo.toml"]
        );
        assert_eq!(BumpSet::Npm.manifests(), &["package.json"]);
        assert!(BumpSet::GithubActions
            .manifests()
            .iter()
            .any(|p| p.contains(".github")));
        assert_eq!(BumpSet::Maven.manifests(), &["MODULE.bazel"]);
        assert_eq!(
            BumpSet::NuGet.manifests(),
            &["third_party/dotnet/paket.dependencies"]
        );
        assert!(BumpSet::Maven
            .locks()
            .contains(&"third_party/jvm/maven_install.json"));
        assert!(BumpSet::NuGet
            .locks()
            .contains(&"third_party/dotnet/paket.lock"));
    }
}

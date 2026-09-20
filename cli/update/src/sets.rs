//! V1 dependency-set registry for `dx update`.
//!
//! Pure registry over the five supported sets named in the issue:
//! Cargo, npm, Maven, NuGet, and Go. Each set is resolver-owned through
//! its approved Bazel integration, refreshes standard locks or equivalent
//! resolved files within declared requirements, and owns no `dx` lockfile
//! or private resolver. Set identity, manifests, and locks are pinned here
//! so selector resolution, backend argv, and per-set reporting agree on
//! one source of truth without a CLI filesystem scan.
//!
//! Manifest/lock paths are workspace-relative and verbatim:
//! - Cargo shares one `crate_universe` lock: manifests under
//!   `rust/tests/fixtures/hello/Cargo.toml` (plus the workspace Rust crates listed in
//!   `MODULE.bazel` sharing `rust/tests/fixtures/hello/Cargo.lock`), lock
//!   `rust/tests/fixtures/hello/Cargo.lock`, derived `cargo-bazel-lock.json` regenerated
//!   via the documented repin.
//! - npm is the root JS/TS graph: manifest `package.json`, lock
//!   `pnpm-lock.yaml` via the Bazel-pinned pnpm.
//! - Maven declares artifacts in `MODULE.bazel`, lock
//!   `third_party/jvm/maven_install.json` via `REPIN=1 bazel run @maven//:pin`.
//! - NuGet declares `third_party/dotnet/paket.dependencies`, lock
//!   `third_party/dotnet/paket.lock`, derived hub under
//!   `third_party/dotnet/deps` via the documented `paket2bazel` run.
//! - Go has no `go.mod` in the main workspace: an empty set that always
//!   succeeds with no launch and no file changes.
//!
//! Independence: the five sets use distinct lockfiles/resolver workspaces,
//! so they are independent for continuation. Sets sharing a lockfile or
//! resolver workspace must never be treated as independent merely because
//! they have different Bazel labels (see [`crate::outcome`]).

/// V1 dependency-set identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SetId {
    /// Rust/Cargo via `crate_universe` (`rust/tests/fixtures/hello/Cargo.lock`).
    Cargo,
    /// Empty Go set (no `go.mod` in the main workspace).
    Go,
    /// JVM/Maven via `rules_jvm_external` (`third_party/jvm/maven_install.json`).
    Maven,
    /// JS/TS/npm via `npm_translate_lock` (`pnpm-lock.yaml`).
    Npm,
    /// .NET/NuGet via Paket (`third_party/dotnet/paket.lock`).
    NuGet,
}

impl SetId {
    /// All five supported sets, in deterministic alphabetical order.
    pub const ALL: [SetId; 5] = [
        SetId::Cargo,
        SetId::Go,
        SetId::Maven,
        SetId::Npm,
        SetId::NuGet,
    ];

    /// Stable selector spelling for this set.
    pub fn name(self) -> &'static str {
        match self {
            SetId::Cargo => "cargo",
            SetId::Go => "go",
            SetId::Maven => "maven",
            SetId::Npm => "npm",
            SetId::NuGet => "nuget",
        }
    }

    /// Parses a set selector spelling. Case-sensitive; no aliases.
    pub fn parse(text: &str) -> Option<SetId> {
        match text {
            "cargo" => Some(SetId::Cargo),
            "go" => Some(SetId::Go),
            "maven" => Some(SetId::Maven),
            "npm" => Some(SetId::Npm),
            "nuget" => Some(SetId::NuGet),
            _ => None,
        }
    }

    /// Workspace-relative manifests owned by this set.
    pub fn manifests(self) -> &'static [&'static str] {
        match self {
            SetId::Cargo => &["rust/tests/fixtures/hello/Cargo.toml"],
            SetId::Go => &[],
            SetId::Maven => &["MODULE.bazel"],
            SetId::Npm => &["package.json"],
            SetId::NuGet => &["third_party/dotnet/paket.dependencies"],
        }
    }

    /// Workspace-relative lockfiles refreshed by this set.
    pub fn locks(self) -> &'static [&'static str] {
        match self {
            SetId::Cargo => &[
                "rust/tests/fixtures/hello/Cargo.lock",
                "cargo-bazel-lock.json",
            ],
            SetId::Go => &[],
            SetId::Maven => &["third_party/jvm/maven_install.json"],
            SetId::Npm => &["pnpm-lock.yaml"],
            SetId::NuGet => &["third_party/dotnet/paket.lock", "third_party/dotnet/deps"],
        }
    }

    /// Human updater description (never argv; argv lives in [`crate::backend`]).
    pub fn updater(self) -> &'static str {
        match self {
            SetId::Cargo => {
                "crate_universe repin (CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello)"
            }
            SetId::Go => "empty set (no go.mod in the main workspace; no-op success)",
            SetId::Maven => "rules_jvm_external pin (REPIN=1 bazel run @maven//:pin)",
            SetId::Npm => "Bazel-pinned pnpm update (bazel run @pnpm//:pnpm -- update)",
            SetId::NuGet => {
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
        for set in SetId::ALL {
            assert!(seen.insert(set.name()), "duplicate set name");
            assert_eq!(SetId::parse(set.name()), Some(set));
        }
        assert_eq!(seen.len(), 5);
        assert_eq!(SetId::parse("Cargo"), None);
        assert_eq!(SetId::parse("cargo-lock"), None);
        assert_eq!(SetId::parse(""), None);
    }

    #[test]
    fn order_is_alphabetical_and_deterministic() {
        let names: Vec<&str> = SetId::ALL.iter().map(|set| set.name()).collect();
        assert_eq!(names, vec!["cargo", "go", "maven", "npm", "nuget"]);
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn no_set_owns_a_dx_lockfile() {
        for set in SetId::ALL {
            for path in set.manifests().iter().chain(set.locks().iter()) {
                assert!(
                    !path.contains("dx.lock") && !path.contains(".dx-lock"),
                    "{set:?} must not own a dx lockfile: {path}"
                );
            }
        }
    }

    #[test]
    fn go_is_empty_while_others_own_locks() {
        assert!(SetId::Go.manifests().is_empty());
        assert!(SetId::Go.locks().is_empty());
        for set in [SetId::Cargo, SetId::Maven, SetId::Npm, SetId::NuGet] {
            assert!(!set.locks().is_empty(), "{set:?} owns a lock");
        }
    }

    #[test]
    fn cargo_npm_maven_nuget_paths_are_pinned() {
        assert_eq!(
            SetId::Cargo.manifests(),
            &["rust/tests/fixtures/hello/Cargo.toml"]
        );
        assert_eq!(
            SetId::Cargo.locks(),
            &[
                "rust/tests/fixtures/hello/Cargo.lock",
                "cargo-bazel-lock.json"
            ]
        );
        assert_eq!(SetId::Npm.manifests(), &["package.json"]);
        assert_eq!(SetId::Npm.locks(), &["pnpm-lock.yaml"]);
        assert_eq!(
            SetId::Maven.locks(),
            &["third_party/jvm/maven_install.json"]
        );
        assert!(SetId::NuGet
            .locks()
            .contains(&"third_party/dotnet/paket.lock"));
    }
}

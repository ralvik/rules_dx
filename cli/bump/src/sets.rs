//! V1 widen-one-requirement set registry for `dx bump` (issue #260).
//!
//! Pure registry over the five v1 manager sets named in the issue,
//! matching the retained `renovate.json` manager set: Bazel modules plus
//! `.bazelversion`, Cargo, npm/pnpm (both lock graphs), Go (`gomod`), and
//! GitHub Actions. Each set owns its declared-requirement manifests; lock
//! refresh stays resolver-owned through `dx update` (`dx_update::backend`)
//! for Cargo/npm/Go, while Bazel and GitHub Actions are file-only (verified
//! through `preset.update --verify-only` plus `bazel build //...`).
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
    /// Rust/Cargo (`rust/hello/Cargo.toml`).
    Cargo,
    /// GitHub Actions (`.github/workflows/*.yml`, SHA-plus-tag pins).
    GithubActions,
    /// Go (`go.mod` graphs where present; empty in the main workspace).
    Go,
    /// JS/TS/npm (`package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`).
    Npm,
}

impl BumpSet {
    /// All five v1 sets, in deterministic alphabetical order.
    pub const ALL: [BumpSet; 5] = [
        BumpSet::Bazel,
        BumpSet::Cargo,
        BumpSet::GithubActions,
        BumpSet::Go,
        BumpSet::Npm,
    ];

    /// Stable selector spelling for this set (matches `renovate.json`
    /// manager names, with `go` covering the `gomod` manager).
    pub fn name(self) -> &'static str {
        match self {
            BumpSet::Bazel => "bazel",
            BumpSet::Cargo => "cargo",
            BumpSet::GithubActions => "github-actions",
            BumpSet::Go => "go",
            BumpSet::Npm => "npm",
        }
    }

    /// Parses a set selector spelling. Case-sensitive; no aliases except
    /// `gomod` for `go` (renovate manager spelling) and `gha` for
    /// `github-actions` (workflow shorthand).
    pub fn parse(text: &str) -> Option<BumpSet> {
        match text {
            "bazel" => Some(BumpSet::Bazel),
            "cargo" => Some(BumpSet::Cargo),
            "github-actions" | "gha" => Some(BumpSet::GithubActions),
            "go" | "gomod" => Some(BumpSet::Go),
            "npm" => Some(BumpSet::Npm),
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
            BumpSet::Cargo => &["rust/hello/Cargo.toml"],
            BumpSet::GithubActions => &[".github/workflows/ci.yml"],
            BumpSet::Go => &["go/go.mod"],
            BumpSet::Npm => &["package.json"],
        }
    }

    /// Workspace-relative lockfiles refreshed resolver-owned after the
    /// widen edit (via `dx update <set>`). Empty means file-only (Bazel,
    /// GitHub Actions): verification runs `preset.update --verify-only`
    /// plus `bazel build //...` with no resolver refresh.
    pub fn locks(self) -> &'static [&'static str] {
        match self {
            BumpSet::Bazel => &[],
            BumpSet::Cargo => &["rust/hello/Cargo.lock", "cargo-bazel-lock.json"],
            BumpSet::GithubActions => &[],
            BumpSet::Go => &["go/go.sum"],
            BumpSet::Npm => &["pnpm-lock.yaml"],
        }
    }

    /// Whether lock refresh runs resolver-owned after widening (Cargo,
    /// npm, Go) or the set is file-only (Bazel, GitHub Actions).
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
                "crate_universe repin (CARGO_BAZEL_REPIN=1 bazel build //rust/hello:hello)"
            }
            BumpSet::GithubActions => {
                "file-only (edit workflow SHA-plus-tag pin via upstream GitHub releases client)"
            }
            BumpSet::Go => "empty in the main workspace (no go.mod; no-op success)",
            BumpSet::Npm => "Bazel-pinned pnpm update (bazel run @pnpm//:pnpm -- update)",
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
        assert_eq!(seen.len(), 5);
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
        assert_eq!(names, vec!["bazel", "cargo", "github-actions", "go", "npm"]);
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
        assert!(BumpSet::Bazel.locks().is_empty());
        assert!(BumpSet::GithubActions.locks().is_empty());
        assert!(!BumpSet::Cargo.locks().is_empty());
    }

    #[test]
    fn manifests_cover_the_v1_manager_set() {
        assert!(BumpSet::Bazel.manifests().contains(&".bazelversion"));
        assert!(BumpSet::Bazel.manifests().contains(&"MODULE.bazel"));
        assert_eq!(BumpSet::Cargo.manifests(), &["rust/hello/Cargo.toml"]);
        assert_eq!(BumpSet::Npm.manifests(), &["package.json"]);
        assert!(BumpSet::GithubActions
            .manifests()
            .iter()
            .any(|p| p.contains(".github")));
    }
}

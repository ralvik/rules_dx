//! Resolver-owned backend operations for `dx update`.
//!
//! Pure argv planning over [`crate::sets::SetId`]: every changed file and
//! invoked operation is attributable to the underlying updater, never to a
//! `dx` lockfile or private resolver (there is none). Backends refresh
//! standard locks or equivalent resolved files within declared requirements
//! ([`crate::semantics`]): exact pins stay constraints, lockfile-only
//! entries may advance, prereleases follow upstream configuration, Git
//! branches may advance while tags/commit pins stay unchanged, and path
//! dependencies are upstream-owned no-ops. Transitive changes remain
//! permitted under upstream resolver semantics; a newer release outside the
//! declared requirements is never a failure.
//!
//! V1 operations (all through approved Bazel integrations, never a CLI
//! filesystem scan):
//! - Cargo (full only): `CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello`
//!   regenerates `cargo-bazel-lock.json` from `Cargo.lock` via
//!   `crate_universe`. Selective `cargo:crate` is unsupported in V1 and
//!   reports `unsupported` rather than silently substituting a full update.
//! - npm (full + selective): `bazel run @pnpm//:pnpm -- update [...]`
//!   through the Bazel-pinned pnpm, refreshing `pnpm-lock.yaml`.
//! - Maven (full only): `REPIN=1 bazel run @maven//:pin` refreshes
//!   `maven_install.json` from `MODULE.bazel` pins (exact pins stay).
//!   Selective `maven:group:artifact` is unsupported in V1.
//! - NuGet (full only): the documented `paket2bazel` regeneration
//!   refreshes `third_party/dotnet/deps` from `paket.dependencies`/
//!   `paket.lock` (exact pin stays). Selective is unsupported in V1.
//! - Go (full only): the pinned `go_deps.from_file` module lock
//!   (`third_party/go/go.mod` plus `go.sum`) intentionally tracks
//!   Gazelle's `go.mod` for the shared extension, so a full update is a
//!   no-op success with no launch; selective is unsupported (explicit
//!   widening runs through `dx bump` plus the pinned SDK tidy).
//!
//! Summaries never render argv, option values, or environment values; the
//! vectors below are passed directly to the process runner.

use super::selector::SetRequest;
use super::sets::SetId;

/// Planned backend operation for one set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendPlan {
    /// Invoke the updater (`argv[0]` is the binary).
    Run {
        /// Argument vector passed directly to the runner.
        argv: Vec<String>,
        /// Extra environment (parent environment is always inherited).
        env: Vec<(String, String)>,
    },
    /// No-op success (Go full: the pinned module lock tracks Gazelle,
    /// nothing to refresh).
    Noop,
}

/// Backend planning failure (execution-time per-set failure, exit 1 for
/// the invocation overall via [`crate::outcome`]/[`crate::report`], never
/// a silent success and never a full-update substitution).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BackendError {
    /// Selective update is not supported for this set in V1.
    #[error("unsupported selective update for {set}: {reason}")]
    Unsupported {
        /// Owning set name.
        set: &'static str,
        /// Why selective is unsupported.
        reason: &'static str,
    },
    /// Cache-only `--offline`/`--frozen` run would need network for this
    /// set: re-run without the flag once connected, or use the vendored
    /// bundle per `docs/deploy/offline-bootstrap.md`.
    /// See: `docs/deploy/offline-bootstrap.md`.
    #[error("offline_required: cannot update {set} without network (re-run without --offline/--frozen once connected, or use the vendored bundle per docs/deploy/offline-bootstrap.md)")]
    OfflineRequired {
        /// Owning set name.
        set: &'static str,
    },
}

/// Plans one set's backend operation.
///
/// `offline` forces cache-only: every `Run` plan that would fetch fails
/// with [`BackendError::OfflineRequired`] instead of launching, while the
/// Go pinned no-op still succeeds (no fetch, no launch). Unsupported
/// selective shapes stay [`BackendError::Unsupported`] regardless of
/// `offline` so the full-set hint never hides behind the network gate.
// See: `docs/deploy/offline-bootstrap.md`.
pub fn plan(set: SetId, request: &SetRequest, offline: bool) -> Result<BackendPlan, BackendError> {
    // Offline gate for fetch-owned backends: cache-only runs never launch
    // a resolver that would fetch. Go full stays the pinned no-op success
    // (no launch, no fetch); unsupported selectives stay unsupported.
    if offline {
        match (set, request) {
            (SetId::Go, SetRequest::Full) => return Ok(BackendPlan::Noop),
            (SetId::Cargo, SetRequest::Packages(_))
            | (SetId::Maven, SetRequest::Packages(_))
            | (SetId::NuGet, SetRequest::Packages(_))
            | (SetId::Go, SetRequest::Packages(_)) => {}
            _ => {
                // Any full/selective `Run` below would fetch; fail before
                // planning argv so no launch is attempted.
                let would_run = matches!(
                    (set, request),
                    (SetId::Cargo, SetRequest::Full)
                        | (SetId::Npm, _)
                        | (SetId::Maven, SetRequest::Full)
                        | (SetId::NuGet, SetRequest::Full)
                );
                if would_run {
                    return Err(BackendError::OfflineRequired { set: set.name() });
                }
            }
        }
    }
    match (set, request) {
        (SetId::Cargo, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&["bazel", "build", "//rust/tests/fixtures/hello:hello"]),
            env: vec![("CARGO_BAZEL_REPIN".to_owned(), "1".to_owned())],
        }),
        (SetId::Cargo, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason: "crate_universe repin refreshes the whole Cargo lock; use `dx update cargo` for the set",
        }),
        (SetId::Npm, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&["bazel", "run", "@pnpm//:pnpm", "--", "update"]),
            env: vec![],
        }),
        (SetId::Npm, SetRequest::Packages(packages)) => Ok(BackendPlan::Run {
            argv: [
                strings(&["bazel", "run", "@pnpm//:pnpm", "--", "update"]),
                packages.clone(),
            ]
            .concat(),
            env: vec![],
        }),
        (SetId::Maven, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&["bazel", "run", "@maven//:pin"]),
            env: vec![("REPIN".to_owned(), "1".to_owned())],
        }),
        (SetId::Maven, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason:
                "maven pins are exact in MODULE.bazel; use `dx update maven` for the set",
        }),
        (SetId::NuGet, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&[
                "bazel",
                "run",
                "@rules_dotnet//tools/paket2bazel",
                "--",
                "--dependencies-file",
                "third_party/dotnet/paket.dependencies",
                "--output-folder",
                "third_party/dotnet/deps",
            ]),
            env: vec![],
        }),
        (SetId::NuGet, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason:
                "nuget pins are exact in paket.dependencies; use `dx update nuget` for the set",
        }),
        (SetId::Go, SetRequest::Full) => Ok(BackendPlan::Noop),
        (SetId::Go, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason: "go pins track Gazelle for the shared go_deps extension; widen explicitly via `dx bump gomod:<module> <version>`",
        }),
    }
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_plans_are_pinned_and_hermetic() {
        let cargo = plan(SetId::Cargo, &SetRequest::Full, false).expect("cargo");
        assert_eq!(
            cargo,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "build".to_owned(),
                    "//rust/tests/fixtures/hello:hello".to_owned(),
                ],
                env: vec![("CARGO_BAZEL_REPIN".to_owned(), "1".to_owned())],
            }
        );
        let npm = plan(SetId::Npm, &SetRequest::Full, false).expect("npm");
        assert_eq!(
            npm,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "run".to_owned(),
                    "@pnpm//:pnpm".to_owned(),
                    "--".to_owned(),
                    "update".to_owned(),
                ],
                env: vec![],
            }
        );
        let maven = plan(SetId::Maven, &SetRequest::Full, false).expect("maven");
        assert_eq!(
            maven,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "run".to_owned(),
                    "@maven//:pin".to_owned(),
                ],
                env: vec![("REPIN".to_owned(), "1".to_owned())],
            }
        );
        let nuget = plan(SetId::NuGet, &SetRequest::Full, false).expect("nuget");
        match nuget {
            BackendPlan::Run { argv, env } => {
                assert_eq!(argv[0], "bazel");
                assert!(argv.contains(&"@rules_dotnet//tools/paket2bazel".to_owned()));
                assert!(argv.contains(&"third_party/dotnet/paket.dependencies".to_owned()));
                assert!(argv.contains(&"third_party/dotnet/deps".to_owned()));
                assert!(env.is_empty());
            }
            BackendPlan::Noop => panic!("nuget runs paket2bazel"),
        }
        assert_eq!(
            plan(SetId::Go, &SetRequest::Full, false).expect("go"),
            BackendPlan::Noop
        );
    }

    #[test]
    fn npm_selective_delegates_packages_to_pnpm() {
        let selective = plan(
            SetId::Npm,
            &SetRequest::Packages(vec!["jest".to_owned(), "react".to_owned()]),
            false,
        )
        .expect("npm selective");
        assert_eq!(
            selective,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "run".to_owned(),
                    "@pnpm//:pnpm".to_owned(),
                    "--".to_owned(),
                    "update".to_owned(),
                    "jest".to_owned(),
                    "react".to_owned(),
                ],
                env: vec![],
            }
        );
    }

    #[test]
    fn cargo_selective_reports_unsupported_never_full() {
        // Issue #633 (See: `docs/decisions/0024-selective-update.md`): per-crate `cargo:<crate>` parses in the selector
        // but the approved `crate_universe` repin has no per-crate flag,
        // so execution fails closed with the full-set hint and never
        // substitutes a full update; private `cargo update -p` stays
        // rejected (resolver-owned backends only).
        let error = plan(
            SetId::Cargo,
            &SetRequest::Packages(vec!["anyhow".to_owned()]),
            false,
        )
        .expect_err("cargo selective is wont-fix");
        assert!(
            matches!(error, BackendError::Unsupported { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("cargo"));
        assert!(error
            .to_string()
            .contains("use `dx update cargo` for the set"));
    }

    #[test]
    fn nuget_selective_reports_unsupported_never_full() {
        // Issue #635 (See: `docs/decisions/0024-selective-update.md`): per-package `nuget:<id>` parses in the selector
        // but the approved `paket2bazel` regen has no per-id flag, so
        // execution fails closed with the full-set hint and never
        // substitutes a full update; private `paket.lock` surgery stays
        // rejected (resolver-owned backends only).
        let error = plan(
            SetId::NuGet,
            &SetRequest::Packages(vec!["FSharp.Core".to_owned()]),
            false,
        )
        .expect_err("nuget selective is wont-fix");
        assert!(
            matches!(error, BackendError::Unsupported { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("nuget"));
        assert!(error
            .to_string()
            .contains("use `dx update nuget` for the set"));
    }

    #[test]
    fn go_selective_reports_unsupported_never_full() {
        // Issue #636 (See: `docs/decisions/0024-selective-update.md`): per-module `go:<module-path>` parses in the
        // selector but the pinned `go_deps.from_file` module lock
        // (`third_party/go/go.mod` plus `go.sum` tracking Gazelle) has
        // no per-module update flag, so execution fails closed with the
        // `dx bump gomod:<module> <version>` hint and never silently
        // substitutes the full no-op; private `go get` plus `go mod tidy`
        // stays rejected (resolver-owned backends only).
        let error = plan(
            SetId::Go,
            &SetRequest::Packages(vec!["github.com/google/go-cmp/cmp".to_owned()]),
            false,
        )
        .expect_err("go selective is wont-fix");
        assert!(
            matches!(error, BackendError::Unsupported { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("go"));
        assert!(error.to_string().contains("dx bump gomod"));
    }

    #[test]
    fn go_full_is_pinned_noop_success() {
        // Issue #636 (See: `docs/decisions/0024-selective-update.md`): the main workspace has no `go.mod` by design; the
        // single-module `go_deps.from_file` lock tracks Gazelle, so the
        // full update is an intentional no-op success with no launch.
        // Real `go get -u` wiring stays rejected (would diverge the
        // shared extension); explicit widening runs through `dx bump`.
        assert_eq!(
            plan(SetId::Go, &SetRequest::Full, false).expect("go full"),
            BackendPlan::Noop
        );
    }

    #[test]
    fn non_npm_selective_reports_unsupported_never_full() {
        for (set, packages) in [
            (
                SetId::Cargo,
                SetRequest::Packages(vec!["anyhow".to_owned()]),
            ),
            (
                SetId::Maven,
                SetRequest::Packages(vec!["junit:junit".to_owned()]),
            ),
            (
                SetId::NuGet,
                SetRequest::Packages(vec!["FSharp.Core".to_owned()]),
            ),
            (
                SetId::Go,
                SetRequest::Packages(vec!["rules_dx/go/tests/fixtures/hello".to_owned()]),
            ),
        ] {
            let error = plan(set, &packages, false).expect_err("unsupported");
            assert!(matches!(error, BackendError::Unsupported { .. }), "{set:?}");
            assert!(error.to_string().contains(set.name()));
        }
    }

    #[test]
    fn maven_selective_reports_unsupported_with_set_hint() {
        // Issue #634 (See: `docs/decisions/0024-selective-update.md`): per-artifact selective is wont-fix in V1. Both the
        // seed (`junit:junit`) and the Jupiter (`org.junit.jupiter:...`)
        // identities fail closed with the full-set hint, never a silent
        // full substitution.
        for artifact in [
            "junit:junit".to_owned(),
            "org.junit.jupiter:junit-jupiter-api".to_owned(),
        ] {
            let error = plan(
                SetId::Maven,
                &SetRequest::Packages(vec![artifact.clone()]),
                false,
            )
            .expect_err("maven selective unsupported");
            let message = error.to_string();
            assert!(
                matches!(error, BackendError::Unsupported { .. }),
                "{artifact}"
            );
            assert!(message.contains("maven"), "{artifact}: {message}");
            assert!(
                message.contains("use `dx update maven` for the set"),
                "{artifact}: {message}"
            );
        }
    }

    #[test]
    fn argv_never_names_a_dx_lockfile() {
        for set in SetId::ALL {
            if let Ok(BackendPlan::Run { argv, .. }) = plan(set, &SetRequest::Full, false) {
                for arg in argv {
                    assert!(
                        !arg.contains("dx.lock"),
                        "{set:?} argv must not name a dx lockfile: {arg}"
                    );
                }
            }
        }
    }

    #[test]
    fn offline_forces_cache_only_except_go_noop() {
        // See: `docs/deploy/offline-bootstrap.md`. Cache-only runs never
        // launch a fetching resolver; the pinned Go no-op still succeeds
        // with no launch, and unsupported selectives stay unsupported so
        // the full-set hint never hides behind the network gate.
        for set in [SetId::Cargo, SetId::Npm, SetId::Maven, SetId::NuGet] {
            let error = plan(set, &SetRequest::Full, true).expect_err("offline needs network");
            assert!(
                matches!(error, BackendError::OfflineRequired { .. }),
                "{set:?}: {error:?}"
            );
            assert!(error.to_string().contains("offline_required"), "{error}");
            assert!(error.to_string().contains(set.name()), "{error}");
        }
        let selective = plan(
            SetId::Npm,
            &SetRequest::Packages(vec!["jest".to_owned()]),
            true,
        )
        .expect_err("npm selective offline needs network");
        assert!(
            matches!(selective, BackendError::OfflineRequired { .. }),
            "{selective:?}"
        );
        assert_eq!(
            plan(SetId::Go, &SetRequest::Full, true).expect("go offline noop"),
            BackendPlan::Noop
        );
        for (set, packages) in [
            (
                SetId::Cargo,
                SetRequest::Packages(vec!["anyhow".to_owned()]),
            ),
            (
                SetId::Maven,
                SetRequest::Packages(vec!["junit:junit".to_owned()]),
            ),
            (
                SetId::NuGet,
                SetRequest::Packages(vec!["FSharp.Core".to_owned()]),
            ),
            (
                SetId::Go,
                SetRequest::Packages(vec!["example.com/mod".to_owned()]),
            ),
        ] {
            let error = plan(set, &packages, true).expect_err("unsupported stays unsupported");
            assert!(
                matches!(error, BackendError::Unsupported { .. }),
                "{set:?}: {error:?}"
            );
        }
    }
}

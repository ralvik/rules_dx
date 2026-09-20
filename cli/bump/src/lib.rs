//! Pure `dx bump` widen-one-requirement planning.
//!
//! This crate owns the explicit widen operation, separate from `dx
//! update`: `dx bump <selector> <version>` rewrites exactly one declared
//! requirement in the working copy. `dx update` keeps its never-rewrites
//! contract (`dx_update::semantics::may_be_rewritten` stays false); this
//! crate owns the single-requirement rewrite, including exact pins,
//! bounded ranges, and Git tag/commit shapes per the ecosystem mapping in
//! [`sets`]. It plans over injected argument strings only, so widening
//! stays deterministic and unit-testable without a workspace, a Bazel
//! server, registries, or any upstream updater.
//!
//! Library-first (ADR 0008): registry discovery, version comparison, and
//! manifest parsing use upstream libraries (BCR / crates.io / npm / Go
//! proxy / Maven Central / NuGet / GitHub releases clients plus `semver`,
//! `serde_json`, `toml`, `toml_edit`), never custom HTTP/version/resolver
//! code. Custom code here is limited to the thin widen-one-requirement
//! edit, loop orchestration docs, and PR handling. All deps pin per ADR
//! 0008 (latest stable, pinned exactly).
//!
//! Frozen command shape (`docs/cli/commands/audit-update-bazel.md`):
//! `dx bump <set:package> <version>`. One invocation widens one
//! requirement (never batch) then chains the refresh automatically
//! (issue #638). Discovery enumerates outdated via the upstream registry
//! clients and proposes stable versions only (issue #639, planned in
//! [`discovery`]); prerelease
//! eligibility follows the upstream resolver and project configuration,
//! never a private policy. Transitive versions stay resolver-governed;
//! lock refresh chains automatically resolver-owned (`dx update cargo`
//! full, `dx update npm:<package>` selective, `dx update go` noop,
//! `dx update maven` full, `dx update nuget` full for
//! Cargo/npm/Go/Maven/NuGet), while Bazel and GitHub Actions verify
//! file-only through `preset.update --verify-only` plus
//! `bazel build //...`.

#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod discovery;
pub mod request;
pub mod sets;
pub mod version;

pub use request::{BumpError, BumpRequest};
pub use sets::BumpSet;
pub use version::{compare, is_stable, prerelease_follows_upstream, VersionError, WidenVersion};

//! Pure documentation-delivery planning (M30a slices 1-4: doc-IR version,
//! identity, validation, mode shape; guides/examples corpus shape;
//! site-build action planning; `dx docs` invocation planning).
//!
//! This crate owns the documentation pipeline shape before any extractor,
//! schema-number freeze, adapter, site-build rule, or `dx docs` command
//! lands: IR version compatibility, stable symbol identities, the
//! extraction-validation gate, check-vs-build mode selection, the drift
//! upgrade gate, the guides/examples corpus shape, the site-build action
//! graph, and the `dx docs` invocation mapping (scope, serve/port, shared
//! graph, failure identities). It plans over injected argument strings
//! only, so the rules stay deterministic and unit-testable without
//! extractors, toolchains, a Bazel server, or any renderer.
//!
//! Out of scope here (O54 qualification): exact `.proto` field/enum numbers
//! and reserved ranges, per-language input pins and adapter mappings,
//! per-language overload-disambiguation schemes, link/reference completeness
//! proofs, renderer behavior, exact guide-step/CI wiring, rule labels,
//! check/serve combination semantics, and any YAML/rule/CLI implementation.
//! Those stay deferred; this crate preserves spellings verbatim and never
//! substitutes an implicit default.

/// One versioned documentation-IR identity (`doc_ir_version`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrVersion {
    /// Major version: breaking changes increment it with a recorded migration.
    pub major: u32,
    /// Minor version: additive-only within a major.
    pub minor: u32,
}

/// Malformed IR version: versions are explicit, never implicit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VersionError {
    /// Major version zero carries no compatibility meaning (no implicit v0).
    MissingMajor,
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionError::MissingMajor => {
                write!(f, "explicit nonzero IR major version is required")
            }
        }
    }
}

impl std::error::Error for VersionError {}

/// Whether a produced IR shard is readable by a consumer at another version.
///
/// Same major versions are compatible in either minor direction: minor
/// versions are additive-only and unknown extension data is preserved
/// verbatim, so an older reader ignores what it does not know and a newer
/// reader accepts older shards. Different majors require the recorded
/// migration — never silent acceptance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionCompat {
    /// Readable as-is (same major; minor skew is additive-only).
    Compatible,
    /// Blocked until the recorded major migration runs.
    RequiresMigration,
}

/// Plan IR version compatibility between producer and reader.
pub fn plan_version_compat(
    produced: IrVersion,
    reader: IrVersion,
) -> Result<VersionCompat, VersionError> {
    if produced.major == 0 || reader.major == 0 {
        return Err(VersionError::MissingMajor);
    }
    if produced.major == reader.major {
        Ok(VersionCompat::Compatible)
    } else {
        Ok(VersionCompat::RequiresMigration)
    }
}

/// Unknown extension payloads are preserved verbatim, never dropped.
pub fn drops_unknown_extensions() -> bool {
    false
}

/// Malformed symbol identity: every segment is explicit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolError {
    /// Language segment missing or empty.
    MissingLanguage,
    /// Package segment missing or empty.
    MissingPackage,
    /// Qualified-name segment missing or empty.
    MissingQualifiedName,
}

impl std::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolError::MissingLanguage => write!(f, "symbol language is required"),
            SymbolError::MissingPackage => write!(f, "symbol package is required"),
            SymbolError::MissingQualifiedName => {
                write!(f, "symbol qualified name is required")
            }
        }
    }
}

impl std::error::Error for SymbolError {}

/// Plan the stable symbol ID: `language:package:qualified_name`.
///
/// IDs are stable across rebuilds; source paths stay workspace-relative
/// (callers must not pass absolute paths). Per-language overload
/// disambiguation schemes freeze under O54; use [`plan_overload_id`] for
/// the explicit normalized parameter-type suffix.
pub fn plan_symbol_id(
    language: &str,
    package: &str,
    qualified: &str,
) -> Result<String, SymbolError> {
    if language.is_empty() {
        return Err(SymbolError::MissingLanguage);
    }
    if package.is_empty() {
        return Err(SymbolError::MissingPackage);
    }
    if qualified.is_empty() {
        return Err(SymbolError::MissingQualifiedName);
    }
    Ok(format!("{language}:{package}:{qualified}"))
}

/// Plan the overload-disambiguated symbol ID by appending the normalized
/// parameter-type list (`Base(T1,T2)`).
///
/// Normalization here is only whitespace trimming with empty entries
/// dropped; per-language type normalization freezes under O54. Types pass
/// through verbatim otherwise.
pub fn plan_overload_id(base: &str, param_types: &[String]) -> String {
    let normalized: Vec<&str> = param_types
        .iter()
        .map(|ty| ty.trim())
        .filter(|ty| !ty.is_empty())
        .collect();
    format!("{}({})", base, normalized.join(","))
}

/// Absolute source paths must never enter the IR.
pub fn is_workspace_relative(path: &str) -> bool {
    !path.is_empty() && !path.starts_with('/')
}

/// Planned validation outcome for one extraction unit.
///
/// Protocol failures fail the extraction action rather than emitting
/// partial shards: every gate below must hold before any shard is emitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationOutcome {
    /// All gates hold: the shard may be emitted.
    Emit,
    /// A gate failed: the action fails with no partial shard.
    FailAction,
}

/// Plan extraction validation: IR decodes against the schema, the
/// symbol-count inventory shows no silent omission, IDs/links are stable,
/// and references resolve. Any failure fails the action — partial shards
/// are never emitted.
pub fn plan_validation(
    decodes_against_schema: bool,
    inventory_complete: bool,
    ids_stable: bool,
    references_resolved: bool,
) -> ValidationOutcome {
    if decodes_against_schema && inventory_complete && ids_stable && references_resolved {
        ValidationOutcome::Emit
    } else {
        ValidationOutcome::FailAction
    }
}

/// Partial IR shards are never emitted.
pub fn emits_partial_shards() -> bool {
    false
}

/// Planned `dx docs` mode selection.
///
/// `--check` performs extraction plus shared validation without rendering;
/// normal build performs the same validation and then renders. Both modes
/// run the same validation and reject the same invalid IR and references;
/// check mode never compares against committed IR or previous cache
/// contents, and a cache miss is never a check failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocsMode {
    /// Extraction plus shared validation, no rendering.
    Check,
    /// Same validation, then render.
    Build,
}

/// Plan the mode from the `--check` flag: the flag selects check mode,
/// its absence selects build mode.
pub fn plan_docs_mode(check: bool) -> DocsMode {
    if check {
        DocsMode::Check
    } else {
        DocsMode::Build
    }
}

/// Whether the mode selects the render action: build only.
pub fn mode_selects_render(mode: DocsMode) -> bool {
    mode == DocsMode::Build
}

/// Check mode never compares IR against committed snapshots.
pub fn check_compares_committed_ir() -> bool {
    false
}

/// A cache miss is never a check failure.
pub fn cache_miss_fails_check() -> bool {
    false
}

/// `--serve` previews the last build outputs locally; it is not a build
/// action and performs no caching of its own.
pub fn serve_is_build_action() -> bool {
    false
}

/// The docs build is non-mutating: Bazel outputs and cache writes are
/// permitted, source-tree writes are not.
pub fn docs_build_mutates_sources() -> bool {
    false
}

/// IR shards, render inputs, and rendered HTML are never committed.
pub fn commits_ir_shards() -> bool {
    false
}

/// Planned drift-upgrade gate.
///
/// Pinned extractor/toolchain upgrades arrive only through rules_dx
/// releases: the contract suite, golden fixtures, and determinism evidence
/// must be green plus explicit review. An upstream change may turn release
/// preparation red but never reaches users outside a release — users stay
/// on pinned, checksummed inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriftDecision {
    /// All gates hold: the pin bump may ship in the release.
    Ship,
    /// A gate failed: the bump is held out of the release.
    Hold,
}

/// Plan whether a pinned-input upgrade may ship.
pub fn plan_drift_upgrade(
    contract_suite_green: bool,
    golden_fixtures_green: bool,
    determinism_ok: bool,
    reviewed: bool,
) -> DriftDecision {
    if contract_suite_green && golden_fixtures_green && determinism_ok && reviewed {
        DriftDecision::Ship
    } else {
        DriftDecision::Hold
    }
}

/// Upstream changes never reach users except through a rules_dx release.
pub fn drift_reaches_users_without_release() -> bool {
    false
}

// ---------------------------------------------------------------------------
// Guides/examples corpus shape (M30a slice 2).
// ---------------------------------------------------------------------------

/// Frozen release-blocking guide identities: quickstart, tutorial, and
/// migration (from existing Bazel setups). Spellings pass through verbatim;
/// no extra guide is claimed and no implicit default is substituted. Exact
/// guide-step text and CI wiring freeze under O54.
pub fn is_known_guide(name: &str) -> bool {
    matches!(name, "quickstart" | "tutorial" | "migration")
}

/// Planned guide-freshness outcome.
///
/// Every guide step is CI-executed so docs cannot rot: a guide is fresh
/// only when every step ran in CI and the `examples/` corpus run stayed
/// green. Any gap leaves the guide stale — never silently fresh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuideFreshness {
    /// Every step CI-executed and the examples corpus green.
    Fresh,
    /// A step went unexecuted or the examples run failed: docs may have rotted.
    Stale,
}

/// Plan guide freshness from the injected CI-execution record.
pub fn plan_guide_freshness(all_steps_executed: bool, examples_green: bool) -> GuideFreshness {
    if all_steps_executed && examples_green {
        GuideFreshness::Fresh
    } else {
        GuideFreshness::Stale
    }
}

/// Guide steps are never allowed to go unexecuted.
pub fn guide_steps_may_go_unexecuted() -> bool {
    false
}

/// Examples corpus root: worked examples live under `examples/` and are the
/// executable backing for guide steps. Paths pass through verbatim; this
/// crate never remaps them onto source or output trees.
pub fn examples_root() -> &'static str {
    "examples/"
}

/// Whether a workspace-relative path selects the examples corpus.
pub fn is_under_examples(path: &str) -> bool {
    path == "examples" || path.starts_with("examples/")
}

// ---------------------------------------------------------------------------
// Site-build action planning (M30a slice 3).
// ---------------------------------------------------------------------------

/// Planned site-build action in the extract → aggregate → render chain.
///
/// One `Extract` runs per (language, package) unit and emits one IR shard;
/// one `Aggregate` consumes shards plus prose plus theme/config with shared
/// validation and emits render inputs; one `Render` runs the pinned mdBook
/// artifact and emits the static site tree. No watcher or refresh engine:
/// Bazel incrementality is the only rebuild mechanism.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocsAction {
    /// Per-unit extraction to one cached IR shard.
    Extract,
    /// Shared-validation aggregation to render inputs.
    Aggregate,
    /// Pinned-renderer site emission.
    Render,
}

/// Plan the action chain for a `dx docs` mode: check selects extraction
/// plus shared-validation aggregation without rendering; build selects the
/// same validation and then renders. Both modes reject the same invalid IR
/// and references; only build exercises renderer failures.
pub fn plan_mode_actions(mode: DocsMode) -> &'static [DocsAction] {
    match mode {
        DocsMode::Check => &[DocsAction::Extract, DocsAction::Aggregate],
        DocsMode::Build => &[
            DocsAction::Extract,
            DocsAction::Aggregate,
            DocsAction::Render,
        ],
    }
}

/// Extraction actions declare every input and never touch the network;
/// required upstream data arrives as declared inputs.
pub fn docs_actions_use_network() -> bool {
    false
}

/// Outputs are deterministic by construction: sorted keys and symbol order,
/// workspace-relative paths only, no timestamps, no absolute paths, no host
/// environment in outputs, locale-independent ordering, UTF-8.
pub fn docs_outputs_allow_timestamps() -> bool {
    false
}

/// Absolute paths must never appear in IR shards, render inputs, or the
/// site tree.
pub fn docs_outputs_allow_absolute_paths() -> bool {
    false
}

/// Same pinned producer plus same declared inputs rebuild byte-identical;
/// cross-version compatibility compares decoded semantics, never bytes.
pub fn same_producer_requires_byte_equality() -> bool {
    true
}

/// Cross-serializer or cross-upgrade byte equality is never required.
pub fn cross_version_requires_byte_equality() -> bool {
    false
}

/// Planned cache-miss outcome: a miss causes normal execution, never a
/// freshness failure. IR shards, render inputs, and HTML are ordinary
/// generated Bazel artifacts — never committed files or source-adjacent
/// snapshots, with no snapshot refresh/apply step and no separate docs cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheMissOutcome {
    /// Re-execute the action normally.
    Execute,
    /// Freshness failure (never selected).
    FailFreshness,
}

/// Plan the cache-miss outcome: always re-execute.
pub fn plan_cache_miss() -> CacheMissOutcome {
    CacheMissOutcome::Execute
}

/// Documentation follows normal target scope with no language enable lists:
/// units from unused foundations emit nothing unless selected.
pub fn unused_units_emit_without_selection() -> bool {
    false
}

/// Bare scope selects the repository.
pub fn bare_docs_scope_selects_repository() -> bool {
    true
}

// ---------------------------------------------------------------------------
// `dx docs` invocation planning (M30a slice 4).
// ---------------------------------------------------------------------------

/// Scope follows the same label/pattern/path resolution as the other
/// workflow commands; no docs-specific scope syntax is introduced.
pub fn docs_scope_reuses_workflow_resolution() -> bool {
    true
}

/// Check and build select the shared Bazel extraction/aggregation graph,
/// never separate checker implementations. O54 must prove the pre-render
/// checks are complete; if a required check depended on rendered output,
/// that conflict is reported before any weaker check mode lands.
pub fn check_uses_separate_graph() -> bool {
    false
}

/// `--serve` builds once and previews the last build outputs locally for
/// authoring. It performs no caching of its own and stays outside the Bazel
/// action graph; the served bytes are exactly the last build outputs.
pub fn serve_caches_own_outputs() -> bool {
    false
}

/// `--port` only refines `--serve`; a port flag without serve selects no
/// preview and is rejected rather than silently ignored.
pub fn port_without_serve_allowed() -> bool {
    false
}

/// Failures name the affected (language, package) unit.
pub fn failure_names_unit() -> bool {
    true
}

/// On extractor drift, failures additionally name the pinned input whose
/// schema changed.
pub fn drift_failure_names_pinned_input() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn same_major_is_compatible_in_either_minor_direction() {
        let old = IrVersion { major: 1, minor: 0 };
        let new = IrVersion { major: 1, minor: 3 };
        assert_eq!(plan_version_compat(new, old), Ok(VersionCompat::Compatible));
        assert_eq!(plan_version_compat(old, new), Ok(VersionCompat::Compatible));
        assert_eq!(plan_version_compat(old, old), Ok(VersionCompat::Compatible));
    }

    #[test]
    fn major_skew_requires_the_recorded_migration() {
        let v1 = IrVersion { major: 1, minor: 9 };
        let v2 = IrVersion { major: 2, minor: 0 };
        assert_eq!(
            plan_version_compat(v1, v2),
            Ok(VersionCompat::RequiresMigration)
        );
        assert_eq!(
            plan_version_compat(v2, v1),
            Ok(VersionCompat::RequiresMigration)
        );
    }

    #[test]
    fn zero_major_fails_closed() {
        let zero = IrVersion { major: 0, minor: 1 };
        let one = IrVersion { major: 1, minor: 0 };
        assert_eq!(
            plan_version_compat(zero, one),
            Err(VersionError::MissingMajor)
        );
        assert_eq!(
            plan_version_compat(one, zero),
            Err(VersionError::MissingMajor)
        );
    }

    #[test]
    fn unknown_extensions_are_preserved_verbatim() {
        assert!(!drops_unknown_extensions());
    }

    #[test]
    fn symbol_ids_join_language_package_and_name() {
        assert_eq!(
            plan_symbol_id("python", "mylib", "AccountService.create"),
            Ok("python:mylib:AccountService.create".to_owned())
        );
    }

    #[test]
    fn symbol_id_segments_are_all_required() {
        assert_eq!(
            plan_symbol_id("", "mylib", "Name"),
            Err(SymbolError::MissingLanguage)
        );
        assert_eq!(
            plan_symbol_id("python", "", "Name"),
            Err(SymbolError::MissingPackage)
        );
        assert_eq!(
            plan_symbol_id("python", "mylib", ""),
            Err(SymbolError::MissingQualifiedName)
        );
    }

    #[test]
    fn overload_suffix_carries_the_normalized_param_list() {
        let base = "python:mylib:AccountService.create";
        assert_eq!(
            plan_overload_id(base, &strings(&["AccountInput"])),
            format!("{base}(AccountInput)")
        );
        // Whitespace trims, empties drop, order and spelling preserved.
        assert_eq!(
            plan_overload_id(base, &strings(&[" int ", "", "string "])),
            format!("{base}(int,string)")
        );
        assert_eq!(plan_overload_id(base, &[]), format!("{base}()"));
    }

    #[test]
    fn only_workspace_relative_paths_enter_the_ir() {
        assert!(is_workspace_relative("src/account.py"));
        assert!(!is_workspace_relative("/home/user/src/account.py"));
        assert!(!is_workspace_relative(""));
    }

    #[test]
    fn validation_emits_only_when_every_gate_holds() {
        assert_eq!(
            plan_validation(true, true, true, true),
            ValidationOutcome::Emit
        );
        assert_eq!(
            plan_validation(false, true, true, true),
            ValidationOutcome::FailAction
        );
        assert_eq!(
            plan_validation(true, false, true, true),
            ValidationOutcome::FailAction
        );
        assert_eq!(
            plan_validation(true, true, false, true),
            ValidationOutcome::FailAction
        );
        assert_eq!(
            plan_validation(true, true, true, false),
            ValidationOutcome::FailAction
        );
    }

    #[test]
    fn partial_shards_are_never_emitted() {
        assert!(!emits_partial_shards());
    }

    #[test]
    fn check_flag_selects_validation_only_build_validates_and_renders() {
        assert_eq!(plan_docs_mode(true), DocsMode::Check);
        assert_eq!(plan_docs_mode(false), DocsMode::Build);
        assert!(!mode_selects_render(DocsMode::Check));
        assert!(mode_selects_render(DocsMode::Build));
    }

    #[test]
    fn check_mode_never_diffs_committed_ir_or_fails_on_cache_miss() {
        assert!(!check_compares_committed_ir());
        assert!(!cache_miss_fails_check());
    }

    #[test]
    fn serve_is_a_local_preview_not_a_build_action() {
        assert!(!serve_is_build_action());
    }

    #[test]
    fn docs_builds_never_touch_sources_or_commit_ir() {
        assert!(!docs_build_mutates_sources());
        assert!(!commits_ir_shards());
    }

    #[test]
    fn drift_upgrades_ship_only_when_green_and_reviewed() {
        assert_eq!(
            plan_drift_upgrade(true, true, true, true),
            DriftDecision::Ship
        );
        assert_eq!(
            plan_drift_upgrade(false, true, true, true),
            DriftDecision::Hold
        );
        assert_eq!(
            plan_drift_upgrade(true, false, true, true),
            DriftDecision::Hold
        );
        assert_eq!(
            plan_drift_upgrade(true, true, false, true),
            DriftDecision::Hold
        );
        assert_eq!(
            plan_drift_upgrade(true, true, true, false),
            DriftDecision::Hold
        );
    }

    #[test]
    fn upstream_changes_never_reach_users_outside_a_release() {
        assert!(!drift_reaches_users_without_release());
    }

    #[test]
    fn only_the_three_release_blocking_guides_are_known() {
        assert!(is_known_guide("quickstart"));
        assert!(is_known_guide("tutorial"));
        assert!(is_known_guide("migration"));
        assert!(!is_known_guide(""));
        assert!(!is_known_guide("howto"));
        assert!(!is_known_guide("Quickstart"));
    }

    #[test]
    fn guide_freshness_requires_every_step_executed_and_green_examples() {
        assert_eq!(plan_guide_freshness(true, true), GuideFreshness::Fresh);
        assert_eq!(plan_guide_freshness(false, true), GuideFreshness::Stale);
        assert_eq!(plan_guide_freshness(true, false), GuideFreshness::Stale);
        assert_eq!(plan_guide_freshness(false, false), GuideFreshness::Stale);
    }

    #[test]
    fn no_guide_step_may_go_unexecuted() {
        assert!(!guide_steps_may_go_unexecuted());
    }

    #[test]
    fn examples_corpus_lives_under_the_examples_root() {
        assert_eq!(examples_root(), "examples/");
        assert!(is_under_examples("examples"));
        assert!(is_under_examples("examples/quickstart"));
        assert!(is_under_examples("examples/quickstart/main.py"));
        assert!(!is_under_examples(""));
        assert!(!is_under_examples("docs/quickstart.md"));
        assert!(!is_under_examples("example"));
    }

    #[test]
    fn check_skips_render_while_build_renders_after_shared_validation() {
        assert_eq!(
            plan_mode_actions(DocsMode::Check),
            &[DocsAction::Extract, DocsAction::Aggregate]
        );
        assert_eq!(
            plan_mode_actions(DocsMode::Build),
            &[
                DocsAction::Extract,
                DocsAction::Aggregate,
                DocsAction::Render
            ]
        );
    }

    #[test]
    fn site_actions_are_hermetic_and_deterministic_by_construction() {
        assert!(!docs_actions_use_network());
        assert!(!docs_outputs_allow_timestamps());
        assert!(!docs_outputs_allow_absolute_paths());
    }

    #[test]
    fn byte_equality_holds_only_for_the_same_pinned_producer() {
        assert!(same_producer_requires_byte_equality());
        assert!(!cross_version_requires_byte_equality());
    }

    #[test]
    fn cache_miss_re_executes_never_fails_freshness() {
        assert_eq!(plan_cache_miss(), CacheMissOutcome::Execute);
    }

    #[test]
    fn scope_is_lazy_with_bare_scope_selecting_the_repository() {
        assert!(!unused_units_emit_without_selection());
        assert!(bare_docs_scope_selects_repository());
    }

    #[test]
    fn docs_scope_reuses_the_shared_workflow_resolution() {
        assert!(docs_scope_reuses_workflow_resolution());
    }

    #[test]
    fn check_shares_the_extraction_graph_never_a_separate_checker() {
        assert!(!check_uses_separate_graph());
    }

    #[test]
    fn serve_previews_without_caching_and_port_requires_serve() {
        assert!(!serve_is_build_action());
        assert!(!serve_caches_own_outputs());
        assert!(!port_without_serve_allowed());
    }

    #[test]
    fn failures_name_the_unit_and_the_drifted_pin() {
        assert!(failure_names_unit());
        assert!(drift_failure_names_pinned_input());
    }
}

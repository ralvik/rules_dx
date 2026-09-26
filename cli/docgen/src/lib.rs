// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrVersion {
    pub major: u32,
    pub minor: u32,
}

/// Malformed IR version: versions are explicit, never implicit.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VersionError {
    #[error("explicit nonzero IR major version is required")]
    MissingMajor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionCompat {
    Compatible,
    RequiresMigration,
}

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

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SymbolError {
    #[error("symbol language is required")]
    MissingLanguage,
    #[error("symbol package is required")]
    MissingPackage,
    #[error("symbol qualified name is required")]
    MissingQualifiedName,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationOutcome {
    Emit,
    FailAction,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocsMode {
    Check,
    Build,
}

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

pub fn serve_is_build_action() -> bool {
    false
}

pub fn docs_build_mutates_sources() -> bool {
    false
}

/// IR shards, render inputs, and rendered HTML are never committed.
pub fn commits_ir_shards() -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriftDecision {
    Ship,
    Hold,
}

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
// Guides/examples corpus shape (slice 2).
// ---------------------------------------------------------------------------

pub fn is_known_guide(name: &str) -> bool {
    matches!(name, "quickstart" | "tutorial" | "migration")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuideFreshness {
    Fresh,
    Stale,
}

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

pub fn examples_root() -> &'static str {
    "examples/"
}

pub fn is_under_examples(path: &str) -> bool {
    path == "examples" || path.starts_with("examples/")
}

// ---------------------------------------------------------------------------
// Site-build action planning (slice 3).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocsAction {
    Extract,
    Aggregate,
    Render,
}

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

pub fn docs_actions_use_network() -> bool {
    false
}

pub fn docs_outputs_allow_timestamps() -> bool {
    false
}

/// Absolute paths must never appear in IR shards, render inputs, or the
pub fn docs_outputs_allow_absolute_paths() -> bool {
    false
}

pub fn same_producer_requires_byte_equality() -> bool {
    true
}

/// Cross-serializer or cross-upgrade byte equality is never required.
pub fn cross_version_requires_byte_equality() -> bool {
    false
}

/// Planned cache-miss outcome: a miss causes normal execution, never a
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheMissOutcome {
    Execute,
    FailFreshness,
}

pub fn plan_cache_miss() -> CacheMissOutcome {
    CacheMissOutcome::Execute
}

pub fn unused_units_emit_without_selection() -> bool {
    false
}

pub fn bare_docs_scope_selects_repository() -> bool {
    true
}

// ---------------------------------------------------------------------------
// `dx docs` invocation planning (slice 4).
// ---------------------------------------------------------------------------

pub fn docs_scope_reuses_workflow_resolution() -> bool {
    true
}

pub fn check_uses_separate_graph() -> bool {
    false
}

pub fn serve_caches_own_outputs() -> bool {
    false
}

/// `--port` only refines `--serve`; a port flag without serve selects no
pub fn port_without_serve_allowed() -> bool {
    false
}

/// `--host` only refines `--serve`; a host flag without serve selects no
pub fn host_without_serve_allowed() -> bool {
    false
}

/// `--open` only refines `--serve`; an open flag without serve selects no
pub fn open_without_serve_allowed() -> bool {
    false
}

pub fn port_zero_allowed() -> bool {
    false
}

pub fn failure_names_unit() -> bool {
    true
}

pub fn drift_failure_names_pinned_input() -> bool {
    true
}

pub fn first_hour_journey_steps() -> &'static [&'static str] {
    &["demo_site", "docs corpus", "site tests"]
}

/// Timing proof is one-shot evidence, never a standing benchmark.
pub fn timing_proof_is_one_shot() -> bool {
    true
}

pub fn timing_proof_enforces_budget() -> bool {
    false
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
        assert!(!host_without_serve_allowed());
        assert!(!open_without_serve_allowed());
        assert!(!port_zero_allowed());
    }

    #[test]
    fn failures_name_the_unit_and_the_drifted_pin() {
        assert!(failure_names_unit());
        assert!(drift_failure_names_pinned_input());
    }

    #[test]
    fn first_hour_timing_is_one_shot_without_budget() {
        assert_eq!(
            first_hour_journey_steps(),
            &["demo_site", "docs corpus", "site tests"]
        );
        assert!(timing_proof_is_one_shot());
        assert!(!timing_proof_enforces_budget());
    }
}

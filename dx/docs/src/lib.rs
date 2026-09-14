//! Pure documentation-delivery planning (M30a slice 1: doc-IR version,
//! identity, validation, and mode shape).
//!
//! This crate owns the documentation pipeline shape before any extractor,
//! schema-number freeze, adapter, site-build rule, or `dx docs` command
//! lands: IR version compatibility, stable symbol identities, the
//! extraction-validation gate, check-vs-build mode selection, and the drift
//! upgrade gate. It plans over injected argument strings only, so the rules
//! stay deterministic and unit-testable without extractors, toolchains, a
//! Bazel server, or any renderer.
//!
//! Out of scope here (O54 qualification): exact `.proto` field/enum numbers
//! and reserved ranges, per-language input pins and adapter mappings,
//! per-language overload-disambiguation schemes, link/reference completeness
//! proofs, renderer behavior, and any YAML/rule/CLI implementation. Those
//! arrive in later M30a slices; this crate preserves spellings verbatim and
//! never substitutes an implicit default.

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
}

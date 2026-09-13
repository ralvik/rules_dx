//! Explicit managed-state cleanup planning for `dx clean` (M25 WP5, O60).
//!
//! Contract: `docs/cli/commands/check-fix-clean.md` (`dx clean
//! [--dry-run] [--bazel]`: prune only validated unselected and unused
//! `.dx` generations and setup records; never delete
//! `.dx/setups/current`, its selected generations, tracked sources,
//! BUILD files, Bazel outputs, shell profiles, or global PATH entries;
//! refuse unmanaged or digest-spoofed paths; `--dry-run` deletes
//! nothing; no automatic pruning, age policy, or count limit) and
//! `docs/environments/managed-state.md#retention-and-recovery`.
//!
//! This crate owns the pure planning layer only: flag-shape constants,
//! setup-record validation against the [`dx_setup`] pair identity,
//! prune selection over an injected inventory, dry-run rendering, and
//! the `bazel clean` forwarding shape with its recovery guidance.
//! Filesystem inventory collection and the locked apply step land in
//! later WP5 slices; planning over injected views keeps this layer
//! deterministic and unit-testable without a workspace.

/// `--dry-run` flag: list reclaimable generations and links without
/// deleting. Matches the `dx clean` contract; frozen here so CLI
/// parsing and help text cannot drift from the qualified shape (O60).
pub const DRY_RUN_FLAG: &str = "--dry-run";

/// `--bazel` flag: additionally forward `bazel clean` and print
/// [`RECOVERY_GUIDANCE`]. Explicit opt-in only; default `dx clean`
/// never touches Bazel outputs.
pub const BAZEL_FLAG: &str = "--bazel";

/// Recovery guidance printed after an explicit `dx clean --bazel`
/// forward, per the clean contract: `bazel clean` leaves managed links
/// dangling, and only explicit `dx env` / `dx codegen` / `dx setup`
/// repairs the projection. Links never self-repair.
pub const RECOVERY_GUIDANCE: &str = "bazel clean forwarded; managed links may now dangle: \
    re-run `dx setup` (or `dx env` / `dx codegen`) to repair the selection";

/// `bazel` subcommand forwarded by `dx clean --bazel`: exactly
/// `bazel clean`, never any other Bazel verb.
pub fn bazel_forward_argv() -> Vec<String> {
    vec!["clean".to_owned()]
}

/// Which managed generation tree a prunable directory belongs to.
/// Generations are link trees into Bazel outputs, never artifact
/// copies, so pruning one only removes metadata plus links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenerationKind {
    Environment,
    Generated,
}

impl GenerationKind {
    /// Directory name under `.dx` holding this generation kind.
    /// Matches `ENVIRONMENTS_DIR_NAME` / `GENERATED_DIR_NAME` in
    /// `dx_setup`.
    pub fn dir_name(&self) -> &'static str {
        match self {
            GenerationKind::Environment => dx_setup::ENVIRONMENTS_DIR_NAME,
            GenerationKind::Generated => dx_setup::GENERATED_DIR_NAME,
        }
    }
}

/// One validated setup record: a hash-addressed directory under
/// `.dx/setups` whose links resolve to exactly the pair its name
/// addresses. Validation happens in [`validate_record`]; only
/// validated records reach [`plan_prune`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRecordView {
    /// Record directory name: lowercase hex of the setup digest.
    pub hex: String,
    /// Environment generation digest the record's `environment` link
    /// addresses.
    pub environment_hex: String,
    /// Generated-code generation digest the record's `generated` link
    /// addresses.
    pub generated_hex: String,
}

/// Setup-record validation failure. Invalid records are refused, never
/// adopted or repaired: the operator removes the offending path or
/// re-runs setup from a clean selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordProblem {
    /// Record, environment, or generated name is not a 64-character
    /// lowercase hexadecimal digest.
    MalformedDigest { value: String },
    /// Record links resolve to a pair whose digest differs from the
    /// record directory name (digest-spoofed path).
    Spoofed { record: String, pair: String },
}

impl std::fmt::Display for RecordProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RecordProblem {}

/// Validates one setup record against the [`dx_setup`] pair identity:
/// every name must be digest-shaped and the record name must equal the
/// digest of the linked pair. Returns the validated view or the reason
/// the record is refused as unmanaged/spoofed (never pruned by
/// [`plan_prune`]).
pub fn validate_record(
    hex: &str,
    environment_hex: &str,
    generated_hex: &str,
) -> Result<SetupRecordView, RecordProblem> {
    for value in [hex, environment_hex, generated_hex] {
        if dx_setup::GenerationId::new(value).is_err() {
            return Err(RecordProblem::MalformedDigest {
                value: value.to_owned(),
            });
        }
    }
    let pair = dx_setup::SetupPair {
        environment: dx_setup::GenerationId::new(environment_hex)
            .expect("validated environment digest"),
        generated: dx_setup::GenerationId::new(generated_hex).expect("validated generated digest"),
    };
    let want = dx_setup::setup_hex(&pair);
    if want != hex {
        return Err(RecordProblem::Spoofed {
            record: hex.to_owned(),
            pair: want,
        });
    }
    Ok(SetupRecordView {
        hex: hex.to_owned(),
        environment_hex: environment_hex.to_owned(),
        generated_hex: generated_hex.to_owned(),
    })
}

/// One present generation directory (digest-shaped name only;
/// anything else is unmanaged and refused, never inventoried here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationView {
    pub kind: GenerationKind,
    pub hex: String,
}

/// Inventory [`plan_prune`] selects from. Filesystem collection lands
/// in a later WP5 slice; planning over injected views keeps selection
/// pure and pinned by test.
pub struct PruneInputs<'a> {
    /// Validated setup records present under `.dx/setups` (validated
    /// by [`validate_record` upstream of this call).
    pub records: &'a [SetupRecordView],
    /// Generations present under `.dx/environments` and
    /// `.dx/generated` (digest-shaped names only).
    pub generations: &'a [GenerationView],
    /// Currently selected setup record hex (`.dx/setups/current`
    /// target), if any.
    pub current_hex: Option<&'a str>,
    /// Setup-record hexes still referenced by an active process
    /// (open shells, editors): never pruned while in use.
    pub active_setup_hexes: &'a [String],
    /// Generation hexes still referenced by an active process: never
    /// pruned while in use.
    pub active_generation_hexes: &'a [String],
    /// Directory names under the managed roots that are not
    /// digest-shaped (or otherwise unmanaged): refused, never deleted.
    pub unmanaged_names: &'a [String],
}

/// Prune selection: validated unselected and unused generations and
/// setup records only. The current record and its selected
/// generations, every active (in-use) record and generation, every
/// generation referenced by a retained record, and every unmanaged
/// path are preserved or refused, never deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanPlan {
    /// Validated setup-record hexes safe to remove: neither current,
    /// nor active, and (by construction) unselected.
    pub prune_setup_records: Vec<String>,
    /// Present generations safe to remove: unreferenced by every
    /// retained record and not active.
    pub prune_generations: Vec<GenerationView>,
    /// Unmanaged names refused (never deleted).
    pub refused_unmanaged: Vec<String>,
    /// Currently selected setup hex, preserved by this plan.
    pub preserved_current: Option<String>,
}

/// Selects the prune set from `inputs`, per
/// `docs/environments/managed-state.md#retention-and-recovery`: a
/// setup record may be removed only when it is neither
/// `.dx/setups/current` nor used by an active process; a generation
/// may be removed only when no retained record references it and no
/// active process uses it. Deterministic: outputs sort ascending.
pub fn plan_prune(inputs: PruneInputs<'_>) -> CleanPlan {
    let mut retained: Vec<&str> = Vec::new();
    if let Some(current) = inputs.current_hex {
        retained.push(current);
    }
    retained.extend(
        inputs
            .active_setup_hexes
            .iter()
            .map(std::string::String::as_str),
    );
    retained.sort_unstable();
    retained.dedup();

    let mut prune_setup_records: Vec<String> = inputs
        .records
        .iter()
        .filter(|record| !retained.contains(&record.hex.as_str()))
        .map(|record| record.hex.clone())
        .collect();
    prune_setup_records.sort();
    prune_setup_records.dedup();

    let mut referenced: Vec<(GenerationKind, &str)> = Vec::new();
    for record in inputs.records {
        if retained.contains(&record.hex.as_str()) {
            referenced.push((GenerationKind::Environment, record.environment_hex.as_str()));
            referenced.push((GenerationKind::Generated, record.generated_hex.as_str()));
        }
    }
    let mut prune_generations: Vec<GenerationView> = inputs
        .generations
        .iter()
        .filter(|generation| {
            !inputs
                .active_generation_hexes
                .iter()
                .any(|active| active == &generation.hex)
                && !referenced
                    .iter()
                    .any(|(kind, hex)| *kind == generation.kind && *hex == generation.hex.as_str())
        })
        .cloned()
        .collect();
    prune_generations.sort_by(|left, right| {
        left.kind
            .dir_name()
            .cmp(right.kind.dir_name())
            .then_with(|| left.hex.cmp(&right.hex))
    });

    let mut refused_unmanaged: Vec<String> =
        inputs.unmanaged_names.iter().map(Clone::clone).collect();
    refused_unmanaged.sort();
    refused_unmanaged.dedup();

    CleanPlan {
        prune_setup_records,
        prune_generations,
        refused_unmanaged,
        preserved_current: inputs.current_hex.map(str::to_owned),
    }
}

/// Renders the `--dry-run` listing for `plan`: reclaimable setup
/// records and generation links, the preserved current selection, and
/// refused unmanaged paths. Deletes nothing; the apply step (later
/// WP5 slice) deletes exactly the listed prune sets.
pub fn render_dry_run(plan: &CleanPlan) -> String {
    let mut lines = vec!["dx clean --dry-run: reclaimable managed state".to_owned()];
    if plan.prune_setup_records.is_empty() && plan.prune_generations.is_empty() {
        lines.push("nothing to prune".to_owned());
    }
    for hex in &plan.prune_setup_records {
        lines.push(format!("prune setup record: .dx/setups/{hex}"));
    }
    for generation in &plan.prune_generations {
        lines.push(format!(
            "prune generation: .dx/{}/{}",
            generation.kind.dir_name(),
            generation.hex
        ));
    }
    match &plan.preserved_current {
        Some(current) => lines.push(format!("preserve current: .dx/setups/{current}")),
        None => lines.push("no current selection".to_owned()),
    }
    for unmanaged in &plan.refused_unmanaged {
        lines.push(format!("refuse unmanaged path: {unmanaged}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: char) -> String {
        tag.to_string().repeat(64)
    }

    fn pair_env_gen(env: char, gen: char) -> (String, String, String) {
        let environment = dx_setup::GenerationId::new(&digest(env)).expect("env digest");
        let generated = dx_setup::GenerationId::new(&digest(gen)).expect("gen digest");
        let pair = dx_setup::SetupPair {
            environment,
            generated,
        };
        (dx_setup::setup_hex(&pair), digest(env), digest(gen))
    }

    fn record(env: char, gen: char) -> SetupRecordView {
        let (hex, environment_hex, generated_hex) = pair_env_gen(env, gen);
        validate_record(&hex, &environment_hex, &generated_hex).expect("valid record")
    }

    fn generation(kind: GenerationKind, tag: char) -> GenerationView {
        GenerationView {
            kind,
            hex: digest(tag),
        }
    }

    fn inputs<'a>(
        records: &'a [SetupRecordView],
        generations: &'a [GenerationView],
        current_hex: Option<&'a str>,
    ) -> PruneInputs<'a> {
        PruneInputs {
            records,
            generations,
            current_hex,
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &[],
        }
    }

    #[test]
    fn flag_shape_is_frozen() {
        assert_eq!(DRY_RUN_FLAG, "--dry-run");
        assert_eq!(BAZEL_FLAG, "--bazel");
        assert_eq!(bazel_forward_argv(), vec!["clean".to_owned()]);
        assert!(RECOVERY_GUIDANCE.contains("dx setup"));
    }

    #[test]
    fn record_validation_pins_pair_identity() {
        let (hex, env_hex, gen_hex) = pair_env_gen('1', '2');
        let view = validate_record(&hex, &env_hex, &gen_hex).expect("valid");
        assert_eq!(view.hex, hex);
        assert_eq!(view.environment_hex, env_hex);
        assert_eq!(view.generated_hex, gen_hex);
    }

    #[test]
    fn malformed_digests_are_refused() {
        assert!(matches!(
            validate_record("not-a-digest", &digest('1'), &digest('2')),
            Err(RecordProblem::MalformedDigest { .. })
        ));
        assert!(matches!(
            validate_record(&digest('a'), &digest('1'), "SPOOF"),
            Err(RecordProblem::MalformedDigest { .. })
        ));
    }

    #[test]
    fn spoofed_record_names_are_refused() {
        let (_, env_hex, gen_hex) = pair_env_gen('1', '2');
        let (other_hex, _, _) = pair_env_gen('3', '4');
        let error = validate_record(&other_hex, &env_hex, &gen_hex).unwrap_err();
        assert!(matches!(error, RecordProblem::Spoofed { .. }));
        assert!(!format!("{error}").is_empty());
    }

    #[test]
    fn current_record_and_its_generations_are_preserved() {
        let current = record('1', '2');
        let stale = record('3', '4');
        let records = vec![current.clone(), stale.clone()];
        let generations = vec![
            generation(GenerationKind::Environment, '1'),
            generation(GenerationKind::Generated, '2'),
            generation(GenerationKind::Environment, '3'),
            generation(GenerationKind::Generated, '4'),
        ];
        let plan = plan_prune(inputs(&records, &generations, Some(&current.hex)));
        assert_eq!(plan.prune_setup_records, vec![stale.hex.clone()]);
        assert_eq!(
            plan.prune_generations,
            vec![
                generation(GenerationKind::Environment, '3'),
                generation(GenerationKind::Generated, '4'),
            ]
        );
        assert_eq!(plan.preserved_current, Some(current.hex.clone()));
        assert!(plan.refused_unmanaged.is_empty());
    }

    #[test]
    fn generations_shared_with_retained_records_are_preserved() {
        let current = record('1', '2');
        let records = vec![current.clone()];
        let generations = vec![
            generation(GenerationKind::Environment, '1'),
            generation(GenerationKind::Generated, '2'),
        ];
        let plan = plan_prune(inputs(&records, &generations, Some(&current.hex)));
        assert!(plan.prune_setup_records.is_empty());
        assert!(plan.prune_generations.is_empty());
    }

    #[test]
    fn active_records_and_generations_are_preserved() {
        let current = record('1', '2');
        let active_record = record('3', '4');
        let records = vec![current.clone(), active_record.clone()];
        let generations = vec![
            generation(GenerationKind::Environment, '3'),
            generation(GenerationKind::Generated, '4'),
        ];
        let active_setup = vec![active_record.hex.clone()];
        let active_generations = vec![digest('3'), digest('4')];
        let plan = plan_prune(PruneInputs {
            records: &records,
            generations: &generations,
            current_hex: Some(&current.hex),
            active_setup_hexes: &active_setup,
            active_generation_hexes: &active_generations,
            unmanaged_names: &[],
        });
        assert!(plan.prune_setup_records.is_empty());
        assert!(plan.prune_generations.is_empty());
    }

    #[test]
    fn unmanaged_names_are_refused_never_pruned() {
        let records = vec![record('1', '2')];
        let unmanaged = vec!["latest".to_owned(), "not-hex".to_owned()];
        let plan = plan_prune(PruneInputs {
            records: &records,
            generations: &[],
            current_hex: None,
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &unmanaged,
        });
        assert_eq!(plan.refused_unmanaged, unmanaged);
        assert!(!plan
            .prune_setup_records
            .iter()
            .any(|hex| unmanaged.contains(hex)));
    }

    #[test]
    fn dry_run_lists_prune_preserve_and_refuse() {
        let current = record('1', '2');
        let stale = record('3', '4');
        let records = vec![current.clone(), stale.clone()];
        let generations = vec![generation(GenerationKind::Environment, '3')];
        let unmanaged = vec!["latest".to_owned()];
        let plan = plan_prune(PruneInputs {
            records: &records,
            generations: &generations,
            current_hex: Some(&current.hex),
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &unmanaged,
        });
        let listing = render_dry_run(&plan);
        assert!(listing.contains(&format!(".dx/setups/{}", stale.hex)));
        assert!(listing.contains(&format!(".dx/environments/{}", digest('3'))));
        assert!(listing.contains(&format!(".dx/setups/{}", current.hex)));
        assert!(listing.contains("latest"));
    }

    #[test]
    fn dry_run_reports_empty_prune_set() {
        let current = record('1', '2');
        let records = vec![current.clone()];
        let plan = plan_prune(inputs(&records, &[], Some(&current.hex)));
        assert!(render_dry_run(&plan).contains("nothing to prune"));
    }
}

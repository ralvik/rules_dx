//! Prune planning for `dx clean` (issue #236).
//!
//! Split from `super` (`lib.rs`): owns [`GenerationView`],
//! [`PruneInputs`], [`CleanPlan`], and [`plan_prune`] (pure selection of
//! validated unselected and unused setup records and generations per
//! `docs/environments/managed-state.md#retention-and-recovery`).
//! Re-exported through `super` so the public paths stay
//! `dx_clean::{GenerationView, PruneInputs, CleanPlan, plan_prune}`.
//! Distinct from the `flags` module (frozen flag shapes), the `records`
//! module (setup-record validation), and the inventory/apply/bytes
//! modules.

use super::records::{GenerationKind, SetupRecordView};

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
    /// by [`super::validate_record`] upstream of this call).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::validate_record;

    fn digest(tag: char) -> String {
        tag.to_string().repeat(64)
    }

    fn record(env: char, gen: char) -> SetupRecordView {
        let environment_hex = digest(env);
        let generated_hex = digest(gen);
        let pair = dx_setup::SetupPair {
            environment: dx_setup::GenerationId::new(&environment_hex).expect("env digest"),
            generated: dx_setup::GenerationId::new(&generated_hex).expect("gen digest"),
        };
        let hex = dx_setup::setup_hex(&pair);
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
}

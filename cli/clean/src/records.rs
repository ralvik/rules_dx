//! Setup-record validation for `dx clean` (issue #236).
//!
//! Split from `super` (`lib.rs`): owns [`GenerationKind`],
//! [`SetupRecordView`], [`RecordProblem`], and [`validate_record`]
//! (digest-shaped name checks plus the [`dx_setup`] pair-identity check
//! that refuses unmanaged/spoofed records). Re-exported through `super`
//! so the public paths stay
//! `dx_clean::{GenerationKind, SetupRecordView, RecordProblem,
//! validate_record}`. Distinct from the `flags` module (frozen flag
//! shapes) and the planning/inventory/apply/bytes modules.

use dx_setup::{setup_hex, GenerationId, SetupPair, ENVIRONMENTS_DIR_NAME, GENERATED_DIR_NAME};

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
            GenerationKind::Environment => ENVIRONMENTS_DIR_NAME,
            GenerationKind::Generated => GENERATED_DIR_NAME,
        }
    }
}

/// One validated setup record: a hash-addressed directory under
/// `.dx/setups` whose links resolve to exactly the pair its name
/// addresses. Validation happens in [`validate_record`]; only
/// validated records reach `plan_prune`.
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordProblem {
    /// Record, environment, or generated name is not a 64-character
    /// lowercase hexadecimal digest.
    #[error("malformed digest {value:?}: want 64-character lowercase hex")]
    MalformedDigest { value: String },
    /// Record links resolve to a pair whose digest differs from the
    /// record directory name (digest-spoofed path).
    #[error("spoofed record {record:?}: links resolve to {pair:?}")]
    Spoofed { record: String, pair: String },
}

/// Parses one digest-shaped name, refusing malformed values instead of
/// panicking at the call site.
fn validated_generation_id(hex: &str) -> Result<GenerationId, RecordProblem> {
    GenerationId::new(hex).map_err(|_| RecordProblem::MalformedDigest {
        value: hex.to_owned(),
    })
}

/// Validates one setup record against the [`dx_setup`] pair identity:
/// every name must be digest-shaped and the record name must equal the
/// digest of the linked pair. Returns the validated view or the reason
/// the record is refused as unmanaged/spoofed (never pruned by
/// `plan_prune`).
pub fn validate_record(
    hex: &str,
    environment_hex: &str,
    generated_hex: &str,
) -> Result<SetupRecordView, RecordProblem> {
    // Each name validates exactly once through one helper, so a malformed
    // digest fails here instead of panicking at pair construction.
    let environment = validated_generation_id(environment_hex)?;
    let generated = validated_generation_id(generated_hex)?;
    validated_generation_id(hex)?;
    let pair = SetupPair {
        environment,
        generated,
    };
    let want = setup_hex(&pair);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: char) -> String {
        tag.to_string().repeat(64)
    }

    fn pair_env_gen(env: char, gen: char) -> (String, String, String) {
        let environment = GenerationId::new(&digest(env)).expect("env digest");
        let generated = GenerationId::new(&digest(gen)).expect("gen digest");
        let pair = SetupPair {
            environment,
            generated,
        };
        (setup_hex(&pair), digest(env), digest(gen))
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
}

//! Validation and codec helpers for the normalized codegen plan shard
//! (M25 WP1, O33).
//!
//! Contract: `docs/environments/codegen.md` (provider contract), schema
//! `//generation:codegen.proto`. This crate enforces the checks that mirror
//! the frozen Starlark semantics in `//generation:codegen.bzl`: producer
//! label shape, non-empty language, non-empty entries, workspace-relative
//! paths, within-shard duplicate logical paths, the always-true
//! `read_only` wire bit, and the optional `exec_path` BEP-matching suffix
//! (empty for logical-only entries, never the reserved shard suffix).
//! Cross-record conflict detection and deterministic
//! merge stay in Starlark (`codegen_conflict_error`, `codegen_merge_records`)
//! and in the `dx` CLI collection; this crate only validates and encodes
//! one contributor shard.

pub use codegen_proto::rules_dx::codegen as proto;
use proto::DxCodegenShard;

/// Validation or codec failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{self:?}")]
pub enum Error {
    Decode(String),
    EmptyProducer,
    BadProducer {
        value: String,
    },
    EmptyLanguage {
        producer: String,
    },
    EmptyEntries {
        producer: String,
    },
    BadPath {
        producer: String,
        path: String,
        reason: &'static str,
    },
    BadExecPath {
        producer: String,
        path: String,
        reason: &'static str,
    },
    DuplicateLogicalPath {
        producer: String,
        path: String,
    },
    NotReadOnly {
        producer: String,
        path: String,
    },
}

fn check_path(producer: &str, path: &str) -> Result<(), Error> {
    // Uses `dx_path::classify` for ladder order; EmptyComponent is
    // intentionally allowed to preserve parity with Starlark
    // `codegen_path_error`, which only rejects empty/absolute/backslash/dot
    // segments (#72 slice 3).
    let reason = match dx_path::classify(path) {
        None => None,
        Some(dx_path::PathProblem::Empty) => Some("must be a non-empty workspace-relative path"),
        Some(dx_path::PathProblem::Absolute) => Some("must not be absolute"),
        Some(dx_path::PathProblem::Backslash) => Some("must not contain '\\'"),
        Some(dx_path::PathProblem::EmptyComponent) => None,
        Some(dx_path::PathProblem::Dot) | Some(dx_path::PathProblem::DotDot) => {
            Some("must not contain '.' or '..' segments")
        }
    };
    match reason {
        Some(reason) => Err(Error::BadPath {
            producer: producer.to_owned(),
            path: path.to_owned(),
            reason,
        }),
        None => Ok(()),
    }
}

fn check_exec_path(producer: &str, path: &str) -> Result<(), Error> {
    // Empty exec paths are logical-only entries requiring no artifact.
    // Non-empty exec paths are BEP-matching suffixes: same shape rules
    // as logical paths, plus refusal of the reserved shard suffix so a
    // shard can never claim another shard as its backing artifact.
    if path.is_empty() {
        return Ok(());
    }
    if path.ends_with(".dxcodegen.pb") {
        return Err(Error::BadExecPath {
            producer: producer.to_owned(),
            path: path.to_owned(),
            reason: "must not use the reserved shard suffix",
        });
    }
    check_path(producer, path).map_err(|error| match error {
        Error::BadPath {
            producer,
            path,
            reason,
        } => Error::BadExecPath {
            producer,
            path,
            reason,
        },
        other => other,
    })
}

/// Validates one contributor shard, mirroring `codegen_record_error`.
pub fn validate(shard: &DxCodegenShard) -> Result<(), Error> {
    if shard.producer.is_empty() {
        return Err(Error::EmptyProducer);
    }
    if !(shard.producer.starts_with("//") || shard.producer.starts_with('@')) {
        return Err(Error::BadProducer {
            value: shard.producer.clone(),
        });
    }
    if shard.language.is_empty() {
        return Err(Error::EmptyLanguage {
            producer: shard.producer.clone(),
        });
    }
    if shard.entries.is_empty() {
        return Err(Error::EmptyEntries {
            producer: shard.producer.clone(),
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &shard.entries {
        check_path(&shard.producer, &entry.logical_path)?;
        check_path(&shard.producer, &entry.import_root)?;
        check_exec_path(&shard.producer, &entry.exec_path)?;
        if !entry.read_only {
            return Err(Error::NotReadOnly {
                producer: shard.producer.clone(),
                path: entry.logical_path.clone(),
            });
        }
        // Shared uniqueness control flow lives in `dx_proto_validate`; only
        // the crate-local `Error` payload stays here (#72 slice).
        dx_proto_validate::check_unique_insert(&mut seen, &entry.logical_path, |existing| {
            Error::DuplicateLogicalPath {
                producer: shard.producer.clone(),
                path: (*existing).clone(),
            }
        })?;
    }
    Ok(())
}

/// Encodes one validated shard to its binary wire form.
pub fn encode_validated(shard: &DxCodegenShard) -> Result<Vec<u8>, Error> {
    dx_proto_validate::encode_with_validation(shard, validate)
}

/// Decodes and validates one shard from its binary wire form.
pub fn decode_validated(bytes: &[u8]) -> Result<DxCodegenShard, Error> {
    dx_proto_validate::decode_with_validation(bytes, validate, Error::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use proto::DxCodegenEntry;

    fn entry(logical_path: &str, import_root: &str, namespace: &str) -> DxCodegenEntry {
        DxCodegenEntry {
            logical_path: logical_path.into(),
            import_root: import_root.into(),
            namespace: namespace.into(),
            read_only: true,
            exec_path: String::new(),
        }
    }

    fn entry_with_exec(
        logical_path: &str,
        import_root: &str,
        namespace: &str,
        exec_path: &str,
    ) -> DxCodegenEntry {
        DxCodegenEntry {
            logical_path: logical_path.into(),
            import_root: import_root.into(),
            namespace: namespace.into(),
            read_only: true,
            exec_path: exec_path.into(),
        }
    }

    fn sample() -> DxCodegenShard {
        DxCodegenShard {
            producer: "//gen:alpha".into(),
            language: "rust".into(),
            entries: vec![
                entry("src/beta.rs", "src", "beta"),
                entry("src/alpha.rs", "src", "alpha"),
            ],
        }
    }

    #[test]
    fn roundtrip_preserves_field_order_and_read_only() {
        let shard = sample();
        let bytes = encode_validated(&shard).unwrap();
        assert_eq!(decode_validated(&bytes).unwrap(), shard);
        // Exec paths round-trip as well.
        let mut with_exec = sample();
        with_exec.entries[0] = entry_with_exec("src/beta.rs", "src", "beta", "result.lib.rs");
        let bytes = encode_validated(&with_exec).unwrap();
        assert_eq!(decode_validated(&bytes).unwrap(), with_exec);
    }

    #[test]
    fn rejects_bad_producers() {
        let mut shard = sample();
        shard.producer.clear();
        assert_eq!(validate(&shard), Err(Error::EmptyProducer));
        shard.producer = "gen:alpha".into();
        assert_eq!(
            validate(&shard),
            Err(Error::BadProducer {
                value: "gen:alpha".into(),
            })
        );
    }

    #[test]
    fn rejects_empty_language_and_empty_entries() {
        let mut shard = sample();
        shard.language.clear();
        assert_eq!(
            validate(&shard),
            Err(Error::EmptyLanguage {
                producer: "//gen:alpha".into(),
            })
        );
        let mut shard = sample();
        shard.entries.clear();
        assert_eq!(
            validate(&shard),
            Err(Error::EmptyEntries {
                producer: "//gen:alpha".into(),
            })
        );
    }

    #[test]
    fn rejects_bad_paths() {
        for path in ["", "/src/a.rs", "src\\a.rs", "src/./a.rs", "src/../a.rs"] {
            let mut shard = sample();
            shard.entries[0].logical_path = path.into();
            assert!(
                matches!(validate(&shard), Err(Error::BadPath { .. })),
                "logical path {path:?} must fail",
            );
            let mut shard = sample();
            shard.entries[0].import_root = path.into();
            assert!(
                matches!(validate(&shard), Err(Error::BadPath { .. })),
                "import root {path:?} must fail",
            );
        }
        // Empty exec paths are logical-only and valid; non-empty ones
        // follow the same shape rules and never use the shard suffix.
        assert!(validate(&sample()).is_ok());
        for path in ["/out/a.rs", "src\\a.rs", "src/./a.rs", "a.dxcodegen.pb"] {
            let mut shard = sample();
            shard.entries[0].exec_path = path.into();
            assert!(
                matches!(validate(&shard), Err(Error::BadExecPath { .. })),
                "exec path {path:?} must fail",
            );
        }
        let mut ok = sample();
        ok.entries[0].exec_path = "result_proto.lib.rs".into();
        assert!(validate(&ok).is_ok());
    }

    #[test]
    fn rejects_duplicates_and_writable_entries() {
        let mut shard = sample();
        shard.entries[1].logical_path = shard.entries[0].logical_path.clone();
        assert_eq!(
            validate(&shard),
            Err(Error::DuplicateLogicalPath {
                producer: "//gen:alpha".into(),
                path: "src/beta.rs".into(),
            })
        );
        let mut shard = sample();
        shard.entries[0].read_only = false;
        assert_eq!(
            validate(&shard),
            Err(Error::NotReadOnly {
                producer: "//gen:alpha".into(),
                path: "src/beta.rs".into(),
            })
        );
    }

    #[test]
    fn rejects_garbage_bytes() {
        assert!(matches!(
            decode_validated(&[0xff; 5]),
            Err(Error::Decode(_))
        ));
        let mut shard = sample();
        shard.language.clear();
        let bytes = shard.encode_to_vec();
        assert_eq!(
            decode_validated(&bytes),
            Err(Error::EmptyLanguage {
                producer: "//gen:alpha".into(),
            })
        );
    }

    #[test]
    fn display_names_the_error() {
        assert!(Error::EmptyProducer.to_string().contains("EmptyProducer"));
    }
}

//! Validation and codec helpers for the Generation Result Protocol.

use prost::Message;

use proto::{
    file_result, FileResult, GenerationManifest, IgnoredImport, Mode, Modification, Scope,
};
pub use result_proto::dx::generation::v1 as proto;

pub const SCHEMA_MAJOR: u32 = 1;
pub const SCHEMA_MINOR: u32 = 0;
pub const DIGEST_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Decode(String),
    UnsupportedMajor {
        found: u32,
    },
    InvalidMode {
        found: i32,
    },
    EmptyScopes,
    EmptyScope {
        index: usize,
    },
    DuplicateScope {
        value: String,
    },
    ScopeCompletion {
        index: usize,
    },
    BadPath {
        at: String,
        path: String,
        reason: &'static str,
    },
    BadBuildBasename {
        path: String,
    },
    ScopeIndex {
        at: String,
        found: u32,
    },
    DuplicateFile {
        path: String,
    },
    MissingChange {
        path: String,
    },
    InvalidUtf8 {
        at: String,
    },
    BadDigestLen {
        path: String,
        found: usize,
    },
    DigestMismatch {
        path: String,
    },
    EmptyEdits {
        path: String,
    },
    InvertedEdit {
        path: String,
        index: usize,
    },
    EditOutOfBounds {
        path: String,
        index: usize,
    },
    EditNotUtf8Boundary {
        path: String,
        index: usize,
    },
    InvalidUtf8Replacement {
        path: String,
        index: usize,
    },
    NoopEdit {
        path: String,
        index: usize,
    },
    EditOrder {
        path: String,
        index: usize,
    },
    UnchangedCandidate {
        path: String,
    },
    CheckOutcome {
        path: String,
    },
    MissingOutcome {
        path: String,
    },
    InvalidOutcome {
        path: String,
        found: i32,
    },
    AppliedFailureCode {
        path: String,
    },
    MissingFailureCode {
        path: String,
    },
    EmptyIgnoredLanguage {
        index: usize,
    },
    EmptyIgnoredImport {
        index: usize,
    },
    IgnoredOrder {
        index: usize,
    },
    DuplicateIgnoredImport {
        index: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

pub fn digest(bytes: &[u8]) -> [u8; DIGEST_LEN] {
    *blake3::hash(bytes).as_bytes()
}

fn check_path(at: &str, path: &str) -> Result<(), Error> {
    let reason = if path.is_empty() {
        Some("path must be non-empty")
    } else if path.starts_with('/') {
        Some("path must be workspace-relative")
    } else if path.contains('\\') {
        Some("path must use forward slashes")
    } else if path.split('/').any(str::is_empty) {
        Some("path must have no empty component")
    } else if path.split('/').any(|part| part == "." || part == "..") {
        Some("path must have no dot component")
    } else {
        None
    };
    match reason {
        Some(reason) => Err(Error::BadPath {
            at: at.to_owned(),
            path: path.to_owned(),
            reason,
        }),
        None => Ok(()),
    }
}

fn check_scope_index(at: &str, scope_index: u32, count: usize) -> Result<(), Error> {
    if usize::try_from(scope_index).map_or(true, |index| index >= count) {
        return Err(Error::ScopeIndex {
            at: at.to_owned(),
            found: scope_index,
        });
    }
    Ok(())
}

fn validate_scopes(scopes: &[Scope]) -> Result<(), Error> {
    if scopes.is_empty() {
        return Err(Error::EmptyScopes);
    }
    let mut seen = std::collections::BTreeSet::new();
    for (index, scope) in scopes.iter().enumerate() {
        if scope.value.is_empty() {
            return Err(Error::EmptyScope { index });
        }
        if !seen.insert(&scope.value) {
            return Err(Error::DuplicateScope {
                value: scope.value.clone(),
            });
        }
    }
    Ok(())
}

fn apply_modification(path: &str, modification: &Modification) -> Result<Vec<u8>, Error> {
    let original =
        str::from_utf8(&modification.original_content).map_err(|_| Error::InvalidUtf8 {
            at: path.to_owned(),
        })?;
    if modification.original_digest.len() != DIGEST_LEN {
        return Err(Error::BadDigestLen {
            path: path.to_owned(),
            found: modification.original_digest.len(),
        });
    }
    if modification.original_digest.as_slice() != digest(original.as_bytes()) {
        return Err(Error::DigestMismatch {
            path: path.to_owned(),
        });
    }
    if modification.edits.is_empty() {
        return Err(Error::EmptyEdits {
            path: path.to_owned(),
        });
    }

    let mut previous_end = 0_u64;
    let mut previous_start = None;
    for (index, edit) in modification.edits.iter().enumerate() {
        if edit.start_byte > edit.end_byte {
            return Err(Error::InvertedEdit {
                path: path.to_owned(),
                index,
            });
        }
        // LCOV_EXCL_START - reason: u64 byte offsets always fit usize on the required 64-bit hosts, so these conversions never fail there; the end-vs-length bound below enforces range.
        let Ok(start) = usize::try_from(edit.start_byte) else {
            return Err(Error::EditOutOfBounds {
                path: path.to_owned(),
                index,
            });
        };
        let Ok(end) = usize::try_from(edit.end_byte) else {
            return Err(Error::EditOutOfBounds {
                path: path.to_owned(),
                index,
            });
        };
        // LCOV_EXCL_STOP - reason: end of 64-bit-unreachable conversion exclusion.
        if end > original.len() {
            return Err(Error::EditOutOfBounds {
                path: path.to_owned(),
                index,
            });
        }
        if !original.is_char_boundary(start) || !original.is_char_boundary(end) {
            return Err(Error::EditNotUtf8Boundary {
                path: path.to_owned(),
                index,
            });
        }
        if str::from_utf8(&edit.replacement).is_err() {
            return Err(Error::InvalidUtf8Replacement {
                path: path.to_owned(),
                index,
            });
        }
        if &original.as_bytes()[start..end] == edit.replacement.as_slice() {
            return Err(Error::NoopEdit {
                path: path.to_owned(),
                index,
            });
        }
        if previous_start.is_some_and(|start| start >= edit.start_byte)
            || (index > 0 && previous_end > edit.start_byte)
        {
            return Err(Error::EditOrder {
                path: path.to_owned(),
                index,
            });
        }
        previous_start = Some(edit.start_byte);
        previous_end = edit.end_byte;
    }

    let capacity = original.len()
        + modification
            .edits
            .iter()
            .map(|edit| edit.replacement.len())
            .sum::<usize>();
    let mut candidate = Vec::with_capacity(capacity);
    let mut cursor = 0;
    for edit in &modification.edits {
        let start = edit.start_byte as usize;
        let end = edit.end_byte as usize;
        candidate.extend_from_slice(&original.as_bytes()[cursor..start]);
        candidate.extend_from_slice(&edit.replacement);
        cursor = end;
    }
    candidate.extend_from_slice(&original.as_bytes()[cursor..]);
    if candidate == original.as_bytes() {
        return Err(Error::UnchangedCandidate {
            path: path.to_owned(),
        });
    }
    Ok(candidate)
}

/// Reconstructs the exact candidate after validating the file-local change.
pub fn candidate(file: &FileResult) -> Result<Vec<u8>, Error> {
    match &file.change {
        Some(file_result::Change::CreateContent(content)) => {
            str::from_utf8(content).map_err(|_| Error::InvalidUtf8 {
                at: file.path.clone(),
            })?;
            Ok(content.clone())
        }
        Some(file_result::Change::Modification(modification)) => {
            apply_modification(&file.path, modification)
        }
        None => Err(Error::MissingChange {
            path: file.path.clone(),
        }),
    }
}

fn validate_file(file: &FileResult, mode: Mode, scope_count: usize) -> Result<(), Error> {
    check_path("file", &file.path)?;
    if !matches!(file.path.rsplit('/').next(), Some("BUILD" | "BUILD.bazel")) {
        return Err(Error::BadBuildBasename {
            path: file.path.clone(),
        });
    }
    check_scope_index("file", file.scope_index, scope_count)?;
    candidate(file)?;

    if mode == Mode::Check {
        if file.outcome != 0 || !file.failure_code.is_empty() {
            return Err(Error::CheckOutcome {
                path: file.path.clone(),
            });
        }
        return Ok(());
    }
    match proto::WriteOutcome::try_from(file.outcome) {
        Ok(proto::WriteOutcome::Applied) if file.failure_code.is_empty() => Ok(()),
        Ok(proto::WriteOutcome::Applied) => Err(Error::AppliedFailureCode {
            path: file.path.clone(),
        }),
        Ok(proto::WriteOutcome::NotApplied) if !file.failure_code.is_empty() => Ok(()),
        Ok(proto::WriteOutcome::NotApplied) => Err(Error::MissingFailureCode {
            path: file.path.clone(),
        }),
        Ok(proto::WriteOutcome::Unspecified) => Err(Error::MissingOutcome {
            path: file.path.clone(),
        }),
        Err(_) => Err(Error::InvalidOutcome {
            path: file.path.clone(),
            found: file.outcome,
        }),
    }
}

fn validate_ignored(ignored: &[IgnoredImport], scope_count: usize) -> Result<(), Error> {
    let mut previous: Option<(&str, &str, &str)> = None;
    for (index, item) in ignored.iter().enumerate() {
        check_path(&format!("ignored_imports[{index}]"), &item.path)?;
        check_scope_index("ignored_import", item.scope_index, scope_count)?;
        if item.language.is_empty() {
            return Err(Error::EmptyIgnoredLanguage { index });
        }
        if item.import.is_empty() {
            return Err(Error::EmptyIgnoredImport { index });
        }
        let key = (
            item.path.as_str(),
            item.language.as_str(),
            item.import.as_str(),
        );
        if let Some(previous) = previous {
            if previous == key {
                return Err(Error::DuplicateIgnoredImport { index });
            }
            if previous > key {
                return Err(Error::IgnoredOrder { index });
            }
        }
        previous = Some(key);
    }
    Ok(())
}

pub fn validate(manifest: &GenerationManifest) -> Result<(), Error> {
    if manifest.schema_major != SCHEMA_MAJOR {
        return Err(Error::UnsupportedMajor {
            found: manifest.schema_major,
        });
    }
    let mode = Mode::try_from(manifest.mode).map_err(|_| Error::InvalidMode {
        found: manifest.mode,
    })?;
    if mode == Mode::Unspecified {
        return Err(Error::InvalidMode {
            found: manifest.mode,
        });
    }
    validate_scopes(&manifest.scopes)?;
    if mode == Mode::Check {
        for (index, scope) in manifest.scopes.iter().enumerate() {
            if scope.results_complete != Some(true) {
                return Err(Error::ScopeCompletion { index });
            }
        }
    }
    let mut files = std::collections::BTreeSet::new();
    for file in &manifest.files {
        validate_file(file, mode, manifest.scopes.len())?;
        if !files.insert(&file.path) {
            return Err(Error::DuplicateFile {
                path: file.path.clone(),
            });
        }
    }
    validate_ignored(&manifest.ignored_imports, manifest.scopes.len())
}

pub fn encode_validated(manifest: &GenerationManifest) -> Result<Vec<u8>, Error> {
    validate(manifest)?;
    Ok(manifest.encode_to_vec())
}

pub fn decode_validated(bytes: &[u8]) -> Result<GenerationManifest, Error> {
    let manifest =
        GenerationManifest::decode(bytes).map_err(|error| Error::Decode(error.to_string()))?;
    validate(&manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::{Edit, WriteOutcome};

    fn modification(content: &[u8]) -> Modification {
        Modification {
            original_digest: digest(content).to_vec(),
            original_content: content.to_vec(),
            edits: vec![Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"z".to_vec(),
            }],
        }
    }

    fn sample(mode: Mode) -> GenerationManifest {
        GenerationManifest {
            schema_major: SCHEMA_MAJOR,
            schema_minor: SCHEMA_MINOR,
            mode: mode as i32,
            scopes: vec![Scope {
                value: "//...".into(),
                results_complete: Some(true),
            }],
            files: vec![FileResult {
                path: "pkg/BUILD.bazel".into(),
                scope_index: 0,
                change: Some(file_result::Change::Modification(modification(b"abc\n"))),
                outcome: if mode == Mode::Default {
                    WriteOutcome::Applied as i32
                } else {
                    0
                },
                failure_code: String::new(),
            }],
            ignored_imports: vec![IgnoredImport {
                path: "pkg/a.rs".into(),
                language: "rust".into(),
                import: "optional".into(),
                scope_index: 0,
            }],
        }
    }

    fn sample_with(mode: Mode, modification: Modification) -> GenerationManifest {
        let mut manifest = sample(mode);
        manifest.files[0].change = Some(file_result::Change::Modification(modification));
        manifest
    }

    fn assert_error(
        mut manifest: GenerationManifest,
        mutate: impl FnOnce(&mut GenerationManifest),
        expected: Error,
    ) {
        mutate(&mut manifest);
        assert_eq!(validate(&manifest), Err(expected));
    }

    #[test]
    fn digest_roundtrip_and_compatibility() {
        let expected = [
            0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc,
            0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca,
            0xe4, 0x1f, 0x32, 0x62,
        ];
        assert_eq!(digest(b""), expected);
        let mut manifest = sample(Mode::Default);
        manifest.schema_minor += 1;
        let bytes = encode_validated(&manifest).unwrap();
        assert_eq!(decode_validated(&bytes).unwrap(), manifest);
        let mut unknown = bytes;
        unknown.extend_from_slice(&[0xa0, 0x06, 0x07]);
        assert_eq!(decode_validated(&unknown).unwrap(), manifest);
        assert!(matches!(
            decode_validated(&[0xff; 5]),
            Err(Error::Decode(_))
        ));
    }

    #[test]
    fn header_and_scope_rules() {
        assert_error(
            sample(Mode::Check),
            |m| m.schema_major = 2,
            Error::UnsupportedMajor { found: 2 },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.mode = 99,
            Error::InvalidMode { found: 99 },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.mode = 0,
            Error::InvalidMode { found: 0 },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.scopes.clear(),
            Error::EmptyScopes,
        );
        assert_error(
            sample(Mode::Check),
            |m| m.scopes[0].value.clear(),
            Error::EmptyScope { index: 0 },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.scopes.push(m.scopes[0].clone()),
            Error::DuplicateScope {
                value: "//...".into(),
            },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.scopes[0].results_complete = None,
            Error::ScopeCompletion { index: 0 },
        );
        let mut partial = sample(Mode::Default);
        partial.scopes[0].results_complete = Some(false);
        assert!(validate(&partial).is_ok());
    }

    #[test]
    fn paths_scope_and_unique_files() {
        for path in [
            "",
            "/BUILD",
            "a//BUILD",
            "a/./BUILD",
            "a/../BUILD",
            "a\\BUILD",
        ] {
            let mut manifest = sample(Mode::Check);
            manifest.files[0].path = path.into();
            assert!(matches!(validate(&manifest), Err(Error::BadPath { .. })));
        }
        assert_error(
            sample(Mode::Check),
            |m| m.files[0].path = "pkg/file.rs".into(),
            Error::BadBuildBasename {
                path: "pkg/file.rs".into(),
            },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.files[0].scope_index = 1,
            Error::ScopeIndex {
                at: "file".into(),
                found: 1,
            },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.files.push(m.files[0].clone()),
            Error::DuplicateFile {
                path: "pkg/BUILD.bazel".into(),
            },
        );
    }

    #[test]
    fn create_and_change_presence_rules() {
        let mut manifest = sample(Mode::Check);
        manifest.files[0].change = Some(file_result::Change::CreateContent(b"x\n".to_vec()));
        assert_eq!(candidate(&manifest.files[0]).unwrap(), b"x\n");
        manifest.files[0].change = Some(file_result::Change::CreateContent(vec![]));
        assert_eq!(candidate(&manifest.files[0]).unwrap(), b"");
        manifest.files[0].change = Some(file_result::Change::CreateContent(vec![0xff]));
        assert!(matches!(
            validate(&manifest),
            Err(Error::InvalidUtf8 { .. })
        ));
        manifest.files[0].change = None;
        assert!(matches!(
            validate(&manifest),
            Err(Error::MissingChange { .. })
        ));
    }

    #[test]
    fn modification_content_digest_and_edits() {
        let mut manifest = sample(Mode::Check);
        assert_eq!(candidate(&manifest.files[0]).unwrap(), b"zbc\n");
        let mut invalid = modification(b"abc\n");
        invalid.original_content = vec![0xff];
        assert!(matches!(
            validate(&sample_with(Mode::Check, invalid)),
            Err(Error::InvalidUtf8 { .. })
        ));

        let cases: Vec<(Box<dyn Fn(&mut Modification)>, Error)> = vec![
            (
                Box::new(|m| m.original_digest.pop().map(drop).unwrap_or(())),
                Error::BadDigestLen {
                    path: "pkg/BUILD.bazel".into(),
                    found: 31,
                },
            ),
            (
                Box::new(|m| m.original_digest[0] ^= 1),
                Error::DigestMismatch {
                    path: "pkg/BUILD.bazel".into(),
                },
            ),
            (
                Box::new(|m| m.edits.clear()),
                Error::EmptyEdits {
                    path: "pkg/BUILD.bazel".into(),
                },
            ),
            (
                Box::new(|m| m.edits[0].start_byte = 2),
                Error::InvertedEdit {
                    path: "pkg/BUILD.bazel".into(),
                    index: 0,
                },
            ),
            (
                Box::new(|m| m.edits[0].end_byte = 99),
                Error::EditOutOfBounds {
                    path: "pkg/BUILD.bazel".into(),
                    index: 0,
                },
            ),
            (
                Box::new(|m| m.edits[0].replacement = vec![0xff]),
                Error::InvalidUtf8Replacement {
                    path: "pkg/BUILD.bazel".into(),
                    index: 0,
                },
            ),
            (
                Box::new(|m| m.edits[0].replacement = b"a".to_vec()),
                Error::NoopEdit {
                    path: "pkg/BUILD.bazel".into(),
                    index: 0,
                },
            ),
        ];
        for (mutate, expected) in cases {
            let mut inner = modification(b"abc\n");
            mutate(&mut inner);
            assert_eq!(validate(&sample_with(Mode::Check, inner)), Err(expected));
        }
    }

    #[test]
    fn utf8_order_overlap_and_candidate_rules() {
        let mut split = modification("éx".as_bytes());
        split.edits[0].end_byte = 1;
        assert!(matches!(
            validate(&sample_with(Mode::Check, split)),
            Err(Error::EditNotUtf8Boundary { .. })
        ));

        let mut overlapping = modification(b"abc\n");
        overlapping.edits = vec![
            Edit {
                start_byte: 0,
                end_byte: 2,
                replacement: b"x".to_vec(),
            },
            Edit {
                start_byte: 1,
                end_byte: 1,
                replacement: b"y".to_vec(),
            },
        ];
        assert!(matches!(
            validate(&sample_with(Mode::Check, overlapping)),
            Err(Error::EditOrder { index: 1, .. })
        ));

        let mut same_start = modification(b"abc\n");
        same_start.edits = vec![
            Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"x".to_vec(),
            },
            Edit {
                start_byte: 0,
                end_byte: 0,
                replacement: b"y".to_vec(),
            },
        ];
        assert!(matches!(
            validate(&sample_with(Mode::Check, same_start)),
            Err(Error::EditOrder { index: 1, .. })
        ));

        let mut compensated = modification(b"abc\n");
        compensated.edits = vec![
            Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: Vec::new(),
            },
            Edit {
                start_byte: 1,
                end_byte: 1,
                replacement: b"a".to_vec(),
            },
        ];
        assert_eq!(
            validate(&sample_with(Mode::Check, compensated)),
            Err(Error::UnchangedCandidate {
                path: "pkg/BUILD.bazel".into()
            })
        );
    }

    #[test]
    fn mode_outcome_consistency() {
        assert_error(
            sample(Mode::Check),
            |m| m.files[0].outcome = WriteOutcome::Applied as i32,
            Error::CheckOutcome {
                path: "pkg/BUILD.bazel".into(),
            },
        );
        assert_error(
            sample(Mode::Default),
            |m| m.files[0].outcome = 0,
            Error::MissingOutcome {
                path: "pkg/BUILD.bazel".into(),
            },
        );
        assert_error(
            sample(Mode::Default),
            |m| m.files[0].outcome = 99,
            Error::InvalidOutcome {
                path: "pkg/BUILD.bazel".into(),
                found: 99,
            },
        );
        assert_error(
            sample(Mode::Default),
            |m| m.files[0].failure_code = "write".into(),
            Error::AppliedFailureCode {
                path: "pkg/BUILD.bazel".into(),
            },
        );
        let mut failed = sample(Mode::Default);
        failed.files[0].outcome = WriteOutcome::NotApplied as i32;
        assert_eq!(
            validate(&failed),
            Err(Error::MissingFailureCode {
                path: "pkg/BUILD.bazel".into()
            })
        );
        failed.files[0].failure_code = "write".into();
        assert!(validate(&failed).is_ok());
    }

    #[test]
    fn ignored_import_shape_order_and_uniqueness() {
        assert_error(
            sample(Mode::Check),
            |m| m.ignored_imports[0].path = "../x".into(),
            Error::BadPath {
                at: "ignored_imports[0]".into(),
                path: "../x".into(),
                reason: "path must have no dot component",
            },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.ignored_imports[0].scope_index = 1,
            Error::ScopeIndex {
                at: "ignored_import".into(),
                found: 1,
            },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.ignored_imports[0].language.clear(),
            Error::EmptyIgnoredLanguage { index: 0 },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.ignored_imports[0].import.clear(),
            Error::EmptyIgnoredImport { index: 0 },
        );
        assert_error(
            sample(Mode::Check),
            |m| m.ignored_imports.push(m.ignored_imports[0].clone()),
            Error::DuplicateIgnoredImport { index: 1 },
        );
        let mut unordered = sample(Mode::Check);
        let mut first = unordered.ignored_imports[0].clone();
        first.path = "a.rs".into();
        unordered.ignored_imports.push(first);
        assert_eq!(validate(&unordered), Err(Error::IgnoredOrder { index: 1 }));
        let mut ordered = sample(Mode::Check);
        ordered.ignored_imports.push(IgnoredImport {
            path: "pkg/b.rs".into(),
            language: "rust".into(),
            import: "optional".into(),
            scope_index: 0,
        });
        assert!(validate(&ordered).is_ok());
    }

    #[test]
    fn display_names_the_error() {
        assert!(Error::EmptyScopes.to_string().contains("EmptyScopes"));
    }
}

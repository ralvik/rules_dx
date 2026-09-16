//! Validation and codec helpers for the Documentation IR (#10).

use prost::Message;

pub use doc_ir_proto::dx::documentation::v1 as proto;
use proto::{DocIr, Symbol};

pub const SCHEMA_MAJOR: u32 = 1;
pub const SCHEMA_MINOR: u32 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Decode(String),
    UnsupportedMajor { found: u32 },
    EmptyLanguage,
    EmptyPackage,
    EmptySymbolId { index: usize },
    DuplicateSymbolId { id: String },
    UnsortedSymbols { id: String },
    AbsoluteSourcePath { id: String, path: String },
    UnsortedExtensions { id: String },
    DuplicateExtension { id: String, key: String },
}

/// Validate one shard: schema version, identity segments, strictly
/// increasing symbol IDs, symbol-ID uniqueness, workspace-relative source
/// paths, and strictly increasing extension keys (symbol and extension
/// order both keep same-producer rebuilds byte-identical).
/// Any failure fails the action — partial shards are never emitted.
pub fn validate_shard(shard: &DocIr) -> Result<(), Error> {
    if shard.schema_major != SCHEMA_MAJOR {
        return Err(Error::UnsupportedMajor {
            found: shard.schema_major,
        });
    }
    if shard.language.is_empty() {
        return Err(Error::EmptyLanguage);
    }
    if shard.package.is_empty() {
        return Err(Error::EmptyPackage);
    }
    let mut seen: Vec<&str> = Vec::with_capacity(shard.symbols.len());
    let mut previous_id: Option<&str> = None;
    for (index, symbol) in shard.symbols.iter().enumerate() {
        validate_symbol(symbol, index)?;
        // Strictly decreasing IDs are an ordering violation; equal IDs
        // fall through to the duplicate check below.
        if previous_id.is_some_and(|prev| symbol.id.as_str() < prev) {
            return Err(Error::UnsortedSymbols {
                id: symbol.id.clone(),
            });
        }
        previous_id = Some(symbol.id.as_str());
        if seen.contains(&symbol.id.as_str()) {
            return Err(Error::DuplicateSymbolId {
                id: symbol.id.clone(),
            });
        }
        seen.push(symbol.id.as_str());
    }
    Ok(())
}

fn validate_symbol(symbol: &Symbol, index: usize) -> Result<(), Error> {
    if symbol.id.is_empty() {
        return Err(Error::EmptySymbolId { index });
    }
    if let Some(source) = symbol.source.as_ref() {
        // Uses `dx_path::classify` for ladder order; only Absolute is
        // rejected to preserve current behavior (empty means no source and
        // stays valid; backslash/empty-component/dot segments remain allowed
        // until a future tightening) (#72 slice 7).
        let is_absolute = matches!(
            dx_path::classify(&source.file),
            Some(dx_path::PathProblem::Absolute)
        );
        if is_absolute {
            return Err(Error::AbsoluteSourcePath {
                id: symbol.id.clone(),
                path: source.file.clone(),
            });
        }
    }
    let mut previous: Option<&str> = None;
    for extension in symbol.extensions.iter() {
        match previous {
            Some(prev) if extension.key.as_str() <= prev => {
                if extension.key.as_str() == prev {
                    return Err(Error::DuplicateExtension {
                        id: symbol.id.clone(),
                        key: extension.key.clone(),
                    });
                }
                return Err(Error::UnsortedExtensions {
                    id: symbol.id.clone(),
                });
            }
            _ => {}
        }
        previous = Some(extension.key.as_str());
    }
    Ok(())
}

/// Encode a validated shard. Validation failures fail encoding: no partial
/// shard bytes are ever produced.
pub fn encode_shard(shard: &DocIr) -> Result<Vec<u8>, Error> {
    validate_shard(shard)?;
    Ok(shard.encode_to_vec())
}

/// Decode and validate shard bytes. Decode and encode reject the same
/// invalid shards: both run [`validate_shard`].
pub fn decode_shard(bytes: &[u8]) -> Result<DocIr, Error> {
    let shard = DocIr::decode(bytes).map_err(|error| Error::Decode(error.to_string()))?;
    validate_shard(&shard)?;
    Ok(shard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::{Extension, Param, Relations, Returns, SourceRef, SymbolKind, Visibility};

    fn example_shard() -> DocIr {
        DocIr {
            schema_major: 1,
            schema_minor: 0,
            language: "python".to_owned(),
            package: "mylib".to_owned(),
            symbols: vec![Symbol {
                id: "python:mylib:AccountService.create".to_owned(),
                kind: SymbolKind::Method as i32,
                signature_text: "def create(self, input: AccountInput) -> Account".to_owned(),
                doc_markdown: "Creates a new account.".to_owned(),
                params: vec![Param {
                    name: "input".to_owned(),
                    r#type: "AccountInput".to_owned(),
                    doc: "Validated input.".to_owned(),
                }],
                returns: Some(Returns {
                    r#type: "Account".to_owned(),
                    doc: "The created account.".to_owned(),
                }),
                examples: vec!["```python\nsvc.create(data)\n```".to_owned()],
                source: Some(SourceRef {
                    file: "src/account.py".to_owned(),
                    line: 42,
                }),
                visibility: Visibility::Public as i32,
                relations: Some(Relations {
                    member_of: vec!["python:mylib:AccountService".to_owned()],
                    implements: vec![],
                }),
                extensions: vec![],
            }],
        }
    }

    #[test]
    fn documented_example_roundtrips_byte_identical() {
        let shard = example_shard();
        let bytes = encode_shard(&shard).unwrap();
        assert_eq!(decode_shard(&bytes).unwrap(), shard);
        // Same producer plus same inputs rebuild byte-identical.
        assert_eq!(encode_shard(&shard).unwrap(), bytes);
    }

    #[test]
    fn encode_and_decode_reject_the_same_invalid_shards() {
        let mut bad_major = example_shard();
        bad_major.schema_major = 0;
        assert_eq!(
            encode_shard(&bad_major),
            Err(Error::UnsupportedMajor { found: 0 })
        );

        let mut bad_lang = example_shard();
        bad_lang.language.clear();
        assert_eq!(encode_shard(&bad_lang), Err(Error::EmptyLanguage));

        let mut bad_pkg = example_shard();
        bad_pkg.package.clear();
        assert_eq!(encode_shard(&bad_pkg), Err(Error::EmptyPackage));

        let mut bad_id = example_shard();
        bad_id.symbols[0].id.clear();
        assert_eq!(
            encode_shard(&bad_id),
            Err(Error::EmptySymbolId { index: 0 })
        );

        let mut dup = example_shard();
        dup.symbols.push(dup.symbols[0].clone());
        assert_eq!(
            encode_shard(&dup),
            Err(Error::DuplicateSymbolId {
                id: "python:mylib:AccountService.create".to_owned(),
            })
        );

        let mut bad_path = example_shard();
        bad_path.symbols[0].source = Some(SourceRef {
            file: "/home/user/src/account.py".to_owned(),
            line: 42,
        });
        assert_eq!(
            encode_shard(&bad_path),
            Err(Error::AbsoluteSourcePath {
                id: "python:mylib:AccountService.create".to_owned(),
                path: "/home/user/src/account.py".to_owned(),
            })
        );

        // A structurally valid encoding of an invalid shard is still
        // rejected on decode: bypass validation via raw prost encode.
        let raw = bad_id.encode_to_vec();
        assert_eq!(decode_shard(&raw), Err(Error::EmptySymbolId { index: 0 }));
        assert!(matches!(decode_shard(&[0xff; 5]), Err(Error::Decode(_))));
    }

    #[test]
    fn extension_keys_must_be_strictly_increasing() {
        let mut shard = example_shard();
        shard.symbols[0].extensions = vec![
            Extension {
                key: "b".to_owned(),
                value: b"2".to_vec(),
            },
            Extension {
                key: "a".to_owned(),
                value: b"1".to_vec(),
            },
        ];
        assert_eq!(
            encode_shard(&shard),
            Err(Error::UnsortedExtensions {
                id: "python:mylib:AccountService.create".to_owned(),
            })
        );

        shard.symbols[0].extensions = vec![
            Extension {
                key: "a".to_owned(),
                value: b"1".to_vec(),
            },
            Extension {
                key: "a".to_owned(),
                value: b"2".to_vec(),
            },
        ];
        assert_eq!(
            encode_shard(&shard),
            Err(Error::DuplicateExtension {
                id: "python:mylib:AccountService.create".to_owned(),
                key: "a".to_owned(),
            })
        );

        shard.symbols[0].extensions = vec![Extension {
            key: "a".to_owned(),
            value: b"1".to_vec(),
        }];
        let bytes = encode_shard(&shard).unwrap();
        assert_eq!(encode_shard(&shard).unwrap(), bytes);
    }

    #[test]
    fn symbols_must_be_strictly_increasing() {
        let mut second = example_shard().symbols[0].clone();
        second.id = "python:mylib:AccountService.delete".to_owned();
        second.source = Some(SourceRef {
            file: "src/account.py".to_owned(),
            line: 84,
        });

        // Decreasing IDs fail on encode with the offending ID.
        let mut reversed = example_shard();
        reversed.symbols = vec![second.clone(), reversed.symbols.remove(0)];
        assert_eq!(
            encode_shard(&reversed),
            Err(Error::UnsortedSymbols {
                id: "python:mylib:AccountService.create".to_owned(),
            })
        );

        // Increasing IDs encode, decode, and rebuild byte-identical.
        let mut ordered = example_shard();
        ordered.symbols.push(second);
        let bytes = encode_shard(&ordered).unwrap();
        assert_eq!(decode_shard(&bytes).unwrap(), ordered);
        assert_eq!(encode_shard(&ordered).unwrap(), bytes);

        // Rejection parity: raw prost bytes bypassing validation still
        // fail on decode.
        let raw = reversed.encode_to_vec();
        assert_eq!(
            decode_shard(&raw),
            Err(Error::UnsortedSymbols {
                id: "python:mylib:AccountService.create".to_owned(),
            })
        );
    }

    #[test]
    fn newer_minors_decode_when_understood() {
        let mut shard = example_shard();
        shard.schema_minor = 3;
        let bytes = encode_shard(&shard).unwrap();
        assert_eq!(decode_shard(&bytes).unwrap(), shard);
    }
}

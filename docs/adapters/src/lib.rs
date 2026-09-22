//! Thin per-language doc adapters normalizing pinned native inputs into the versioned IR.
//!
//! Owning contract: `docs/documentation/doc-ir.md`.
//! See: `docs/documentation/README.md#contracts` for the pipeline slice.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use documentation_ir::proto::{
    DocIr, Extension, Param, Relations, SourceRef, Symbol, SymbolKind, Visibility,
};

/// Pinned nightly rustdoc route (`-Z unstable-options --output-format json`).
pub const RUST_RUSTDOC_PIN: &str = "nightly-2026-09-01";
/// Pinned rustdoc JSON format version observed on the pin.
pub const RUST_FORMAT_VERSION: u32 = 30;
/// Pinned Griffe (`griffe dump --full`).
pub const PYTHON_GRIFFE_PIN: &str = "2.2.0";
/// Pinned TypeDoc (`--json --emit none`).
pub const TYPESCRIPT_TYPEDOC_PIN: &str = "0.28.20";
/// Pinned JDK for the custom Doclet route.
pub const JAVA_JDK_PIN: &str = "25";
/// Pinned Kotlin for the Dokka route.
pub const KOTLIN_PIN: &str = "2.2.20";
/// Pinned Dokka (custom plugin emitting owned JSON).
pub const KOTLIN_DOKKA_PIN: &str = "2.2.0";
/// Pinned Go toolchain for the `go/packages` extractor.
pub const GO_TOOLCHAIN_PIN: &str = "1.26.6";
/// Pinned `golang.org/x/tools` pseudo-version for `packages.Load`.
pub const GO_XTOOLS_PIN: &str = "v0.36.0";
/// Pinned Doxygen XML generator.
pub const CPP_DOXYGEN_PIN: &str = "1.18.0";
/// Pinned .NET SDK carrying Roslyn (C# assembly plus `/doc` XML join).
pub const CSHARP_DOTNET_PIN: &str = "10.0.201";
/// Pinned .NET SDK for the F# route.
pub const FSHARP_DOTNET_PIN: &str = "10.0.201";
/// Pinned compiler-service build for the F# join.
pub const FSHARP_SERVICE_PIN: &str = "43.9.200";
/// Pinned `vue-docgen-api` (`parseMulti`, arrays-only).
pub const VUE_DOCGEN_PIN: &str = "4.79.2";
/// Pinned `sveld` (compared against compiler plus `svelte2tsx`).
pub const SVELTE_SVELD_PIN: &str = "0.37.3";
/// Pinned Scala 3 for the TASTy Inspector spike.
pub const SCALA_PIN: &str = "3.3.6";

/// Adapter scopes with extraction (twelve) plus the prose-only path.
pub const ADAPTER_SCOPES: &[&str] = &[
    "rust",
    "python",
    "typescript",
    "java",
    "kotlin",
    "go",
    "cpp",
    "csharp",
    "fsharp",
    "vue",
    "svelte",
    "scala",
    "astromdx",
];

/// Adapter failure: every malformed or unpinned input fails the run.
/// Partial shards are never emitted (see `docs/documentation/doc-ir.md`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdapterError {
    #[error("empty native input")]
    EmptyInput,
    #[error("pinned-producer mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
    #[error("native JSON does not parse: {0}")]
    InvalidJson(String),
    #[error("empty language or package")]
    EmptyIdentity,
    #[error("absolute source path rejected for {id}: {path}")]
    AbsolutePath { id: String, path: String },
    #[error("duplicate symbol id: {0}")]
    DuplicateId(String),
    #[error("missing or incompatible TASTy: empty inputs never inventory")]
    MissingTasty,
    #[error("prose-only package saw a non-markdown file: {0}")]
    NonMarkdown(String),
    #[error("IR rejected by the versioned codec: {0}")]
    InvalidIr(String),
}

fn shard(language: &str, package: &str, symbols: Vec<Symbol>) -> Result<DocIr, AdapterError> {
    if language.is_empty() || package.is_empty() {
        return Err(AdapterError::EmptyIdentity);
    }
    let mut ordered = symbols;
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let mut previous: Option<&str> = None;
    for symbol in &ordered {
        if let Some(prev) = previous {
            if prev == symbol.id {
                return Err(AdapterError::DuplicateId(symbol.id.clone()));
            }
        }
        previous = Some(symbol.id.as_str());
        if let Some(source) = symbol.source.as_ref() {
            if source.file.starts_with('/') {
                return Err(AdapterError::AbsolutePath {
                    id: symbol.id.clone(),
                    path: source.file.clone(),
                });
            }
        }
    }
    let shard = DocIr {
        schema_major: 1,
        schema_minor: 0,
        language: language.to_owned(),
        package: package.to_owned(),
        symbols: ordered,
    };
    documentation_ir::validate_shard(&shard)
        .map_err(|err| AdapterError::InvalidIr(format!("{err:?}")))?;
    Ok(shard)
}

fn symbol_id(language: &str, package: &str, qualified: &str) -> Result<String, AdapterError> {
    // Stable `language:package:qualified_name` identity (see `docs/documentation/doc-ir.md`).
    if language.is_empty() || package.is_empty() || qualified.is_empty() {
        return Err(AdapterError::EmptyIdentity);
    }
    Ok(format!("{language}:{package}:{qualified}"))
}

fn overload_id(base: &str, params: &[String]) -> String {
    // Normalized parameter-type suffix `Base(T1,T2)`; whitespace trims, empties drop.
    let normalized: Vec<&str> = params
        .iter()
        .map(|ty| ty.trim())
        .filter(|ty| !ty.is_empty())
        .collect();
    format!("{}({})", base, normalized.join(","))
}

fn text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn kind_of(name: &str) -> i32 {
    match name {
        "module" => SymbolKind::Module as i32,
        "class" | "struct" | "interface" => SymbolKind::Class as i32,
        "function" => SymbolKind::Function as i32,
        "method" => SymbolKind::Method as i32,
        "field" | "prop" | "property" => SymbolKind::Property as i32,
        "constant" | "const" => SymbolKind::Constant as i32,
        "enum" => SymbolKind::Enum as i32,
        "variant" => SymbolKind::EnumVariant as i32,
        "alias" | "type" => SymbolKind::TypeAlias as i32,
        _ => SymbolKind::Function as i32,
    }
}

/// Grouped `make_symbol` inputs so the 8-value symbol constructor takes
/// one params struct instead of eight positionals.
struct SymbolSpec<'a> {
    language: &'a str,
    package: &'a str,
    qualified: &'a str,
    kind_name: &'a str,
    signature: String,
    doc: String,
    file: String,
    line: u64,
}

fn make_symbol(spec: SymbolSpec<'_>) -> Result<Symbol, AdapterError> {
    let SymbolSpec {
        language,
        package,
        qualified,
        kind_name,
        signature,
        doc,
        file,
        line,
    } = spec;
    let id = symbol_id(language, package, qualified)?;
    Ok(Symbol {
        id,
        kind: kind_of(kind_name),
        signature_text: signature,
        doc_markdown: doc,
        params: vec![],
        returns: None,
        examples: vec![],
        source: Some(SourceRef { file, line }),
        visibility: Visibility::Public as i32,
        relations: Some(Relations {
            member_of: vec![],
            implements: vec![],
        }),
        extensions: vec![],
    })
}

/// Rust: pinned nightly rustdoc JSON `index` into IR.
/// See: `docs/documentation/doc-ir.md#machine-inputs`.
pub fn normalize_rust(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        .to_string();
    if found != RUST_FORMAT_VERSION.to_string() {
        return Err(AdapterError::VersionMismatch {
            expected: RUST_FORMAT_VERSION.to_string(),
            found,
        });
    }
    let nightly = value
        .get("nightly")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if nightly != RUST_RUSTDOC_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: RUST_RUSTDOC_PIN.to_owned(),
            found: nightly.to_owned(),
        });
    }
    let index = value
        .get("index")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| AdapterError::InvalidJson("missing index".to_owned()))?;
    let mut symbols = Vec::new();
    for item in index.values() {
        let name = text(item.get("name").unwrap_or(&serde_json::Value::Null));
        let kind = text(item.get("kind").unwrap_or(&serde_json::Value::Null));
        let docs = text(item.get("docs").unwrap_or(&serde_json::Value::Null));
        let file = text(
            item.get("span")
                .and_then(|span| span.get("filename"))
                .unwrap_or(&serde_json::Value::Null),
        );
        let line = item
            .get("span")
            .and_then(|span| span.get("line"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        if name.is_empty() {
            continue;
        }
        // Overloads disambiguate by normalized parameter-type list.
        let params: Vec<String> = item
            .get("params")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().map(text).collect())
            .unwrap_or_default();
        let base = symbol_id("rust", package, &name)?;
        let id = if params.is_empty() {
            base.clone()
        } else {
            overload_id(&base, &params)
        };
        symbols.push(Symbol {
            id,
            kind: kind_of(&kind),
            signature_text: format!("{kind} {name}"),
            doc_markdown: docs,
            params: params
                .iter()
                .map(|ty| Param {
                    name: String::new(),
                    r#type: ty.clone(),
                    doc: String::new(),
                })
                .collect(),
            returns: None,
            examples: vec![],
            source: Some(SourceRef { file, line }),
            visibility: Visibility::Public as i32,
            relations: Some(Relations {
                member_of: vec![],
                implements: vec![],
            }),
            extensions: vec![Extension {
                key: "rustdoc.format_version".to_owned(),
                value: RUST_FORMAT_VERSION.to_string().into_bytes(),
            }],
        });
    }
    shard("rust", package, symbols)
}

/// Python: Griffe `--full` model into IR.
pub fn normalize_python(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = text(
        value
            .get("griffe_version")
            .unwrap_or(&serde_json::Value::Null),
    );
    if found != PYTHON_GRIFFE_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: PYTHON_GRIFFE_PIN.to_owned(),
            found,
        });
    }
    let mut symbols = Vec::new();
    let members = value
        .get("members")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing members".to_owned()))?;
    for member in members {
        collect_griffe("python", package, member, &mut symbols)?;
    }
    shard("python", package, symbols)
}

fn collect_griffe(
    language: &str,
    package: &str,
    node: &serde_json::Value,
    out: &mut Vec<Symbol>,
) -> Result<(), AdapterError> {
    let kind = text(node.get("kind").unwrap_or(&serde_json::Value::Null));
    let name = text(node.get("name").unwrap_or(&serde_json::Value::Null));
    let path = text(node.get("path").unwrap_or(&serde_json::Value::Null));
    let qualified = if path.is_empty() { name.clone() } else { path };
    if qualified.is_empty() {
        return Err(AdapterError::InvalidJson("member without path".to_owned()));
    }
    let doc = text(node.get("docstring").unwrap_or(&serde_json::Value::Null));
    let file = text(node.get("file").unwrap_or(&serde_json::Value::Null));
    let line = node
        .get("lineno")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    let params: Vec<String> = node
        .get("parameters")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|param| text(param.get("type").unwrap_or(&serde_json::Value::Null)))
                .collect()
        })
        .unwrap_or_default();
    let base = symbol_id(language, package, &qualified)?;
    let id = if params.is_empty() {
        base.clone()
    } else {
        overload_id(&base, &params)
    };
    out.push(Symbol {
        id,
        kind: kind_of(&kind),
        signature_text: format!("{kind} {qualified}"),
        doc_markdown: doc,
        params: params
            .iter()
            .map(|ty| Param {
                name: String::new(),
                r#type: ty.clone(),
                doc: String::new(),
            })
            .collect(),
        returns: None,
        examples: vec![],
        source: Some(SourceRef { file, line }),
        visibility: Visibility::Public as i32,
        relations: Some(Relations {
            member_of: vec![],
            implements: vec![],
        }),
        extensions: vec![],
    });
    if let Some(children) = node.get("members").and_then(serde_json::Value::as_array) {
        for child in children {
            collect_griffe(language, package, child, out)?;
        }
    }
    Ok(())
}

/// TypeScript/JavaScript: TypeDoc JSON (`schemaVersion`) into IR.
pub fn normalize_typescript(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = text(value.get("typedoc").unwrap_or(&serde_json::Value::Null));
    if found != TYPESCRIPT_TYPEDOC_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: TYPESCRIPT_TYPEDOC_PIN.to_owned(),
            found,
        });
    }
    let mut symbols = Vec::new();
    collect_typedoc("typescript", package, &value, &mut symbols)?;
    shard("typescript", package, symbols)
}

fn collect_typedoc(
    language: &str,
    package: &str,
    node: &serde_json::Value,
    out: &mut Vec<Symbol>,
) -> Result<(), AdapterError> {
    if let Some(children) = node.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            let name = text(child.get("name").unwrap_or(&serde_json::Value::Null));
            // TypeDoc kinds are numeric; map the common class/interface/method set.
            let kind_num = child
                .get("kind")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let kind = match kind_num {
                128 => "class",
                256 => "interface",
                512 => "method",
                1024 => "function",
                32 => "enum",
                _ => "function",
            };
            let doc = child
                .get("comment")
                .and_then(|comment| comment.get("summary"))
                .and_then(serde_json::Value::as_array)
                .map(|blocks| blocks.iter().map(text).collect::<Vec<_>>().join(""))
                .unwrap_or_default();
            let file = child
                .get("sources")
                .and_then(serde_json::Value::as_array)
                .and_then(|sources| sources.first())
                .and_then(|source| source.get("fileName"))
                .map(text)
                .unwrap_or_default();
            let line = child
                .get("sources")
                .and_then(serde_json::Value::as_array)
                .and_then(|sources| sources.first())
                .and_then(|source| source.get("line"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1);
            if !name.is_empty() && child.get("flags").is_some() {
                let params: Vec<String> = child
                    .get("signatures")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|sigs| sigs.first())
                    .and_then(|sig| sig.get("parameters"))
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .map(|param| {
                                param
                                    .get("type")
                                    .and_then(|ty| ty.get("name"))
                                    .map(text)
                                    .unwrap_or_default()
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let base = symbol_id(language, package, &name)?;
                let id = if params.is_empty() {
                    base
                } else {
                    overload_id(&base, &params)
                };
                out.push(Symbol {
                    id,
                    kind: kind_of(kind),
                    signature_text: format!("{kind} {name}"),
                    doc_markdown: doc,
                    params: params
                        .iter()
                        .map(|ty| Param {
                            name: String::new(),
                            r#type: ty.clone(),
                            doc: String::new(),
                        })
                        .collect(),
                    returns: None,
                    examples: vec![],
                    source: Some(SourceRef { file, line }),
                    visibility: Visibility::Public as i32,
                    relations: Some(Relations {
                        member_of: vec![],
                        implements: vec![],
                    }),
                    extensions: vec![],
                });
            }
            collect_typedoc(language, package, child, out)?;
        }
    }
    Ok(())
}

/// Java: owned Doclet JSON (specified-element count reconciles silently dropped APIs).
pub fn normalize_java(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = text(value.get("jdk").unwrap_or(&serde_json::Value::Null));
    if found != JAVA_JDK_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: JAVA_JDK_PIN.to_owned(),
            found,
        });
    }
    let elements = value
        .get("elements")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing elements".to_owned()))?;
    let mut symbols = Vec::new();
    for element in elements {
        let qualified = text(
            element
                .get("qualifiedName")
                .unwrap_or(&serde_json::Value::Null),
        );
        let kind = text(element.get("kind").unwrap_or(&serde_json::Value::Null));
        let doc = text(element.get("doc").unwrap_or(&serde_json::Value::Null));
        let file = text(element.get("file").unwrap_or(&serde_json::Value::Null));
        let line = element
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        symbols.push(make_symbol(SymbolSpec {
            language: "java",
            package,
            qualified: &qualified,
            kind_name: &kind.to_lowercase(),
            signature: format!("{kind} {qualified}"),
            doc,
            file,
            line,
        })?);
    }
    shard("java", package, symbols)
}

/// Kotlin: owned Dokka-plugin JSON (Dokka-to-Kotlin lockstep is pinned).
pub fn normalize_kotlin(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let dokka = text(value.get("dokka").unwrap_or(&serde_json::Value::Null));
    if dokka != KOTLIN_DOKKA_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: KOTLIN_DOKKA_PIN.to_owned(),
            found: dokka,
        });
    }
    let kotlin = text(value.get("kotlin").unwrap_or(&serde_json::Value::Null));
    if kotlin != KOTLIN_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: KOTLIN_PIN.to_owned(),
            found: kotlin,
        });
    }
    let declarations = value
        .get("declarations")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing declarations".to_owned()))?;
    let mut symbols = Vec::new();
    for declaration in declarations {
        let qualified = text(
            declaration
                .get("fqname")
                .unwrap_or(&serde_json::Value::Null),
        );
        let kind = text(declaration.get("kind").unwrap_or(&serde_json::Value::Null));
        let doc = text(declaration.get("doc").unwrap_or(&serde_json::Value::Null));
        let file = text(declaration.get("file").unwrap_or(&serde_json::Value::Null));
        let line = declaration
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        symbols.push(make_symbol(SymbolSpec {
            language: "kotlin",
            package,
            qualified: &qualified,
            kind_name: &kind.to_lowercase(),
            signature: format!("{kind} {qualified}"),
            doc,
            file,
            line,
        })?);
    }
    shard("kotlin", package, symbols)
}

/// Go: thin `go/packages` plus doc-comment AST JSON into IR.
pub fn normalize_go(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let toolchain = text(value.get("go").unwrap_or(&serde_json::Value::Null));
    if toolchain != GO_TOOLCHAIN_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: GO_TOOLCHAIN_PIN.to_owned(),
            found: toolchain,
        });
    }
    let xtools = text(value.get("xtools").unwrap_or(&serde_json::Value::Null));
    if xtools != GO_XTOOLS_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: GO_XTOOLS_PIN.to_owned(),
            found: xtools,
        });
    }
    let funcs = value
        .get("funcs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing funcs".to_owned()))?;
    let mut symbols = Vec::new();
    for func in funcs {
        let name = text(func.get("name").unwrap_or(&serde_json::Value::Null));
        let doc = text(func.get("doc").unwrap_or(&serde_json::Value::Null));
        let pos = text(func.get("pos").unwrap_or(&serde_json::Value::Null));
        let (file, line) = split_pos(&pos);
        symbols.push(make_symbol(SymbolSpec {
            language: "go",
            package,
            qualified: &name,
            kind_name: "function",
            signature: format!("func {name}"),
            doc,
            file,
            line,
        })?);
    }
    shard("go", package, symbols)
}

fn split_pos(pos: &str) -> (String, u64) {
    match pos.rsplit_once(':') {
        Some((file, line)) => (file.to_owned(), line.parse().unwrap_or(1)),
        None => (pos.to_owned(), 1),
    }
}

/// C/C++: Doxygen XML first candidate into IR (built-in versus Clang-assisted
/// comparison stays implementation work; the adapter owns the contract).
pub fn normalize_cpp(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    if !input.contains("<doxygen") {
        return Err(AdapterError::InvalidJson("missing doxygen root".to_owned()));
    }
    if !input.contains(&format!("version=\"{CPP_DOXYGEN_PIN}\""))
        && !input.contains(&format!("version='{CPP_DOXYGEN_PIN}'"))
        && !input.contains(CPP_DOXYGEN_PIN)
    {
        return Err(AdapterError::VersionMismatch {
            expected: CPP_DOXYGEN_PIN.to_owned(),
            found: "unpinned".to_owned(),
        });
    }
    let mut symbols = Vec::new();
    for member in split_members(input) {
        let kind = tag_attr(&member, "memberdef", "kind").unwrap_or_else(|| "function".to_owned());
        let name = tag_text(&member, "name").unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let brief = tag_text(&member, "briefdescription").unwrap_or_default();
        let file = location_file(&member).unwrap_or_default();
        let line = location_line(&member).unwrap_or(1);
        symbols.push(make_symbol(SymbolSpec {
            language: "cpp",
            package,
            qualified: &name,
            kind_name: &kind,
            signature: format!("{kind} {name}"),
            doc: brief,
            file,
            line,
        })?);
    }
    shard("cpp", package, symbols)
}

fn split_members(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<memberdef") {
        let tail = &rest[start..];
        if let Some(end) = tail.find("</memberdef>") {
            out.push(tail[..end + "</memberdef>".len()].to_owned());
            rest = &tail[end + "</memberdef>".len()..];
        } else {
            break;
        }
    }
    out
}

fn tag_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = block.find(&open)?;
    let content_start = block[start..].find('>')? + start + 1;
    let end = block[content_start..].find(&close)? + content_start;
    Some(block[content_start..end].trim().to_owned())
}

fn tag_attr(block: &str, tag: &str, attr: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = block.find(&open)?;
    let end = block[start..].find('>')? + start;
    let header = &block[start..end];
    let needle = format!("{attr}=\"");
    let attr_start = header.find(&needle)? + needle.len();
    let attr_end = header[attr_start..].find('"')? + attr_start;
    Some(header[attr_start..attr_end].to_owned())
}

fn location_file(block: &str) -> Option<String> {
    let start = block.find("<location")?;
    let end = block[start..].find('>')? + start;
    tag_attr(&block[start..end + 1], "location", "file")
}

fn location_line(block: &str) -> Option<u64> {
    let start = block.find("<location")?;
    let end = block[start..].find('>')? + start;
    tag_attr(&block[start..end + 1], "location", "line")?
        .parse()
        .ok()
}

/// C#: assembly metadata joined with `/doc` XML into IR.
/// XML alone is not the model; unresolved IDs keep an empty doc, never drop.
pub fn normalize_csharp(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = text(value.get("sdk").unwrap_or(&serde_json::Value::Null));
    if found != CSHARP_DOTNET_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: CSHARP_DOTNET_PIN.to_owned(),
            found,
        });
    }
    let members = value
        .get("members")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing members".to_owned()))?;
    let docs = value
        .get("docs")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut symbols = Vec::new();
    for member in members {
        let id = text(member.get("id").unwrap_or(&serde_json::Value::Null));
        let kind = text(member.get("kind").unwrap_or(&serde_json::Value::Null));
        let name = text(member.get("name").unwrap_or(&serde_json::Value::Null));
        let file = text(member.get("file").unwrap_or(&serde_json::Value::Null));
        let line = member
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let doc = docs.get(&id).map(text).unwrap_or_default();
        symbols.push(make_symbol(SymbolSpec {
            language: "csharp",
            package,
            qualified: &name,
            kind_name: &kind.to_lowercase(),
            signature: format!("{kind} {name}"),
            doc,
            file,
            line,
        })?);
        let _ = id;
    }
    shard("csharp", package, symbols)
}

/// F#: compiler-service signatures joined with XML docs into IR.
pub fn normalize_fsharp(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let sdk = text(value.get("sdk").unwrap_or(&serde_json::Value::Null));
    if sdk != FSHARP_DOTNET_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: FSHARP_DOTNET_PIN.to_owned(),
            found: sdk,
        });
    }
    let service = text(value.get("fcs").unwrap_or(&serde_json::Value::Null));
    if service != FSHARP_SERVICE_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: FSHARP_SERVICE_PIN.to_owned(),
            found: service,
        });
    }
    let entities = value
        .get("entities")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing entities".to_owned()))?;
    let docs = value
        .get("docs")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut symbols = Vec::new();
    for entity in entities {
        let signature = text(entity.get("signature").unwrap_or(&serde_json::Value::Null));
        let kind = text(entity.get("kind").unwrap_or(&serde_json::Value::Null));
        let name = text(entity.get("name").unwrap_or(&serde_json::Value::Null));
        let file = text(entity.get("file").unwrap_or(&serde_json::Value::Null));
        let line = entity
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let doc = docs.get(&signature).map(text).unwrap_or_default();
        symbols.push(make_symbol(SymbolSpec {
            language: "fsharp",
            package,
            qualified: &name,
            kind_name: &kind.to_lowercase(),
            signature: format!("{kind} {name}"),
            doc,
            file,
            line,
        })?);
    }
    shard("fsharp", package, symbols)
}

/// Vue: `vue-docgen-api` `parseMulti` JSON into IR (arrays-only).
pub fn normalize_vue(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = text(value.get("docgen").unwrap_or(&serde_json::Value::Null));
    if found != VUE_DOCGEN_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: VUE_DOCGEN_PIN.to_owned(),
            found,
        });
    }
    let components = value
        .get("components")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing components".to_owned()))?;
    let mut symbols = Vec::new();
    for component in components {
        // Arrays-only contract: object-shaped props/events/slots/methods fail.
        for key in ["props", "events", "slots", "methods"] {
            if let Some(node) = component.get(key) {
                if !node.is_array() {
                    return Err(AdapterError::InvalidJson(format!("{key} must be an array")));
                }
            }
        }
        let export = text(
            component
                .get("exportName")
                .unwrap_or(&serde_json::Value::Null),
        );
        let description = text(
            component
                .get("description")
                .unwrap_or(&serde_json::Value::Null),
        );
        let file = text(component.get("file").unwrap_or(&serde_json::Value::Null));
        symbols.push(make_symbol(SymbolSpec {
            language: "vue",
            package,
            qualified: &export,
            kind_name: "class",
            signature: format!("component {export}"),
            doc: description,
            file,
            line: 1,
        })?);
        for prop in component
            .get("props")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let name = text(prop.get("name").unwrap_or(&serde_json::Value::Null));
            let doc = text(prop.get("description").unwrap_or(&serde_json::Value::Null));
            symbols.push(make_symbol(SymbolSpec {
                language: "vue",
                package,
                qualified: &format!("{export}.{name}"),
                kind_name: "property",
                signature: format!("prop {name}"),
                doc,
                file: String::new(),
                line: 1,
            })?);
        }
    }
    shard("vue", package, symbols)
}

/// Svelte: `sveld` JSON into IR (runes plus legacy, snippets versus slots).
pub fn normalize_svelte(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let found = text(value.get("sveld").unwrap_or(&serde_json::Value::Null));
    if found != SVELTE_SVELD_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: SVELTE_SVELD_PIN.to_owned(),
            found,
        });
    }
    let components = value
        .get("components")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AdapterError::InvalidJson("missing components".to_owned()))?;
    let mut symbols = Vec::new();
    for component in components {
        let name = text(component.get("name").unwrap_or(&serde_json::Value::Null));
        let doc = text(
            component
                .get("description")
                .unwrap_or(&serde_json::Value::Null),
        );
        let file = text(component.get("file").unwrap_or(&serde_json::Value::Null));
        symbols.push(make_symbol(SymbolSpec {
            language: "svelte",
            package,
            qualified: &name,
            kind_name: "class",
            signature: format!("component {name}"),
            doc,
            file,
            line: 1,
        })?);
        for prop in component
            .get("props")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let prop_name = text(prop.get("name").unwrap_or(&serde_json::Value::Null));
            let prop_doc = text(prop.get("description").unwrap_or(&serde_json::Value::Null));
            symbols.push(make_symbol(SymbolSpec {
                language: "svelte",
                package,
                qualified: &format!("{name}.{prop_name}"),
                kind_name: "property",
                signature: format!("prop {prop_name}"),
                doc: prop_doc,
                file: String::new(),
                line: 1,
            })?);
        }
    }
    shard("svelte", package, symbols)
}

/// Scala: TASTy Inspector spike into IR.
/// Missing or incompatible TASTy never inventories; it fails closed.
pub fn normalize_scala(input: &str, package: &str) -> Result<DocIr, AdapterError> {
    if input.trim().is_empty() {
        return Err(AdapterError::EmptyInput);
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|err| AdapterError::InvalidJson(err.to_string()))?;
    let scala = text(value.get("scala").unwrap_or(&serde_json::Value::Null));
    if scala != SCALA_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: SCALA_PIN.to_owned(),
            found: scala,
        });
    }
    let tasty = value.get("tasty").ok_or(AdapterError::MissingTasty)?;
    if tasty
        .get("missing")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Err(AdapterError::MissingTasty);
    }
    let tasty_version = text(tasty.get("version").unwrap_or(&serde_json::Value::Null));
    if tasty_version != SCALA_PIN {
        return Err(AdapterError::VersionMismatch {
            expected: SCALA_PIN.to_owned(),
            found: tasty_version,
        });
    }
    let symbols_json = tasty
        .get("symbols")
        .and_then(serde_json::Value::as_array)
        .ok_or(AdapterError::MissingTasty)?;
    if symbols_json.is_empty() {
        return Err(AdapterError::MissingTasty);
    }
    let mut symbols = Vec::new();
    for node in symbols_json {
        let qualified = text(node.get("fqname").unwrap_or(&serde_json::Value::Null));
        let kind = text(node.get("kind").unwrap_or(&serde_json::Value::Null));
        let doc = text(node.get("doc").unwrap_or(&serde_json::Value::Null));
        let file = text(node.get("file").unwrap_or(&serde_json::Value::Null));
        let line = node
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        symbols.push(make_symbol(SymbolSpec {
            language: "scala",
            package,
            qualified: &qualified,
            kind_name: &kind.to_lowercase(),
            signature: format!("{kind} {qualified}"),
            doc,
            file,
            line,
        })?);
    }
    shard("scala", package, symbols)
}

/// Astro/MDX: prose-only confirmation (no extractor; authored markdown flows straight through).
pub fn confirm_prose_only(package: &str, files: &[&str]) -> Result<DocIr, AdapterError> {
    if package.is_empty() {
        return Err(AdapterError::EmptyIdentity);
    }
    for file in files {
        if !(file.ends_with(".md")
            || file.ends_with(".mdx")
            || file.ends_with(".astro")
            || file.is_empty())
        {
            return Err(AdapterError::NonMarkdown((*file).to_owned()));
        }
        if file.starts_with('/') {
            return Err(AdapterError::AbsolutePath {
                id: format!("markdown:{package}"),
                path: (*file).to_owned(),
            });
        }
    }
    shard("markdown", package, Vec::new())
}

/// Same-producer rebuilds stay byte-identical; cross-version compares decoded semantics.
/// See: `docs/documentation/site.md`.
pub fn encode_ir(shard: &DocIr) -> Result<Vec<u8>, AdapterError> {
    documentation_ir::encode_shard(shard).map_err(|err| AdapterError::InvalidIr(format!("{err:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        match name {
            "rust/input.json" => include_str!("../testdata/rust/input.json").to_owned(),
            "python/input.json" => include_str!("../testdata/python/input.json").to_owned(),
            "typescript/input.json" => include_str!("../testdata/typescript/input.json").to_owned(),
            "java/input.json" => include_str!("../testdata/java/input.json").to_owned(),
            "kotlin/input.json" => include_str!("../testdata/kotlin/input.json").to_owned(),
            "go/input.json" => include_str!("../testdata/go/input.json").to_owned(),
            "cpp/input.xml" => include_str!("../testdata/cpp/input.xml").to_owned(),
            "csharp/input.json" => include_str!("../testdata/csharp/input.json").to_owned(),
            "fsharp/input.json" => include_str!("../testdata/fsharp/input.json").to_owned(),
            "vue/input.json" => include_str!("../testdata/vue/input.json").to_owned(),
            "svelte/input.json" => include_str!("../testdata/svelte/input.json").to_owned(),
            "scala/input.json" => include_str!("../testdata/scala/input.json").to_owned(),
            _ => panic!("missing fixture {name}"),
        }
    }

    #[test]
    fn rust_normalizes_pinned_index_with_overloads() {
        let shard = normalize_rust(&fixture("rust/input.json"), "demo").unwrap();
        assert_eq!(shard.language, "rust");
        assert_eq!(shard.symbols.len(), 4);
        assert!(shard.symbols.windows(2).all(|pair| pair[0].id < pair[1].id));
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn rust_rejects_unpinned_producer() {
        let bad = fixture("rust/input.json").replace(RUST_RUSTDOC_PIN, "nightly-2020-01-01");
        assert!(matches!(
            normalize_rust(&bad, "demo"),
            Err(AdapterError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn python_normalizes_griffe_members() {
        let shard = normalize_python(&fixture("python/input.json"), "mylib").unwrap();
        assert_eq!(shard.symbols.len(), 4);
        assert!(shard.symbols.windows(2).all(|pair| pair[0].id < pair[1].id));
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn typescript_normalizes_typedoc_reflections() {
        let shard = normalize_typescript(&fixture("typescript/input.json"), "web").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn java_normalizes_doclet_elements() {
        let shard = normalize_java(&fixture("java/input.json"), "example").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn kotlin_normalizes_dokka_declarations() {
        let shard = normalize_kotlin(&fixture("kotlin/input.json"), "example").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn go_normalizes_packages_with_positions() {
        let shard = normalize_go(&fixture("go/input.json"), "example").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn cpp_normalizes_doxygen_xml() {
        let shard = normalize_cpp(&fixture("cpp/input.xml"), "native").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn csharp_joins_metadata_with_docs() {
        let shard = normalize_csharp(&fixture("csharp/input.json"), "Example").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        // Unresolved IDs keep an empty doc, never drop the symbol.
        assert!(shard
            .symbols
            .iter()
            .any(|symbol| symbol.doc_markdown.is_empty()));
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn fsharp_joins_service_with_docs() {
        let shard = normalize_fsharp(&fixture("fsharp/input.json"), "Example").unwrap();
        assert_eq!(shard.symbols.len(), 2);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn vue_requires_arrays_only() {
        let shard = normalize_vue(&fixture("vue/input.json"), "web").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bad = fixture("vue/input.json").replace("\"props\": [", "\"props\": {\"oops\": ");
        assert!(normalize_vue(&bad, "web").is_err());
    }

    #[test]
    fn svelte_normalizes_components() {
        let shard = normalize_svelte(&fixture("svelte/input.json"), "web").unwrap();
        assert_eq!(shard.symbols.len(), 3);
        let bytes = encode_ir(&shard).unwrap();
        assert_eq!(encode_ir(&shard).unwrap(), bytes);
    }

    #[test]
    fn scala_spike_fails_closed_without_tasty() {
        let shard = normalize_scala(&fixture("scala/input.json"), "example").unwrap();
        assert_eq!(shard.symbols.len(), 2);
        let missing = r#"{"scala": "3.3.6", "tasty": {"missing": true}}"#;
        assert_eq!(
            normalize_scala(missing, "example"),
            Err(AdapterError::MissingTasty)
        );
        let empty = r#"{"scala": "3.3.6", "tasty": {"version": "3.3.6", "symbols": []}}"#;
        assert_eq!(
            normalize_scala(empty, "example"),
            Err(AdapterError::MissingTasty)
        );
    }

    #[test]
    fn astromdx_confirms_prose_only() {
        let shard =
            confirm_prose_only("site", &["docs/guide.md", "blog/post.mdx", "pages/a.astro"])
                .unwrap();
        assert!(shard.symbols.is_empty());
        assert_eq!(
            confirm_prose_only("site", &["src/main.rs"]),
            Err(AdapterError::NonMarkdown("src/main.rs".to_owned()))
        );
    }

    #[test]
    fn adapters_reject_absolute_paths() {
        let bad = fixture("python/input.json").replace("src/account.py", "/abs/account.py");
        assert!(matches!(
            normalize_python(&bad, "mylib"),
            Err(AdapterError::AbsolutePath { .. })
        ));
    }

    #[test]
    fn thirteen_scopes_stay_pinned() {
        assert_eq!(ADAPTER_SCOPES.len(), 13);
        assert!(ADAPTER_SCOPES.contains(&"scala"));
        assert!(ADAPTER_SCOPES.contains(&"astromdx"));
    }
}

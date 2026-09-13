//! Shard writer binary: thin CLI shim over the shard codec library.
//!
//! The `dx_codegen_shard` Starlark rule invokes this writer with one
//! contributor's validated record fields; the binary re-validates and emits
//! the binary `DxCodegenShard` protobuf. All record semantics live in the
//! library and are unit-tested there.
//!
//! Usage:
//! ```text
//! codegen_shard_writer --producer LABEL --language LANG \
//!   --entry LOGICAL_PATH|IMPORT_ROOT|NAMESPACE [--entry ...] \
//!   --output OUT.dxcodegen.pb
//! ```

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and shard-emission execution, not unit coverage.
use std::path::PathBuf;

use codegen_shard::{
    decode_validated, encode_validated,
    proto::{DxCodegenEntry, DxCodegenShard},
};

fn usage() -> String {
    "usage: codegen_shard_writer --producer LABEL --language LANG --entry LOGICAL|ROOT|NAMESPACE [--entry ...] --output OUT".into()
}

fn parse_entry(raw: &str) -> Result<DxCodegenEntry, String> {
    let parts: Vec<&str> = raw.split('|').collect();
    if parts.len() != 3 {
        return Err(format!(
            "bad --entry {raw:?}: want LOGICAL_PATH|IMPORT_ROOT|NAMESPACE"
        ));
    }
    Ok(DxCodegenEntry {
        logical_path: parts[0].into(),
        import_root: parts[1].into(),
        namespace: parts[2].into(),
        read_only: true,
    })
}

fn run(args: &[String]) -> Result<(), String> {
    let mut producer: Option<String> = None;
    let mut language: Option<String> = None;
    let mut entries = Vec::new();
    let mut output: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--producer" => {
                i += 1;
                producer = Some(args.get(i).ok_or_else(usage)?.clone());
            }
            "--language" => {
                i += 1;
                language = Some(args.get(i).ok_or_else(usage)?.clone());
            }
            "--entry" => {
                i += 1;
                entries.push(parse_entry(args.get(i).ok_or_else(usage)?)?);
            }
            "--output" => {
                i += 1;
                output = Some(PathBuf::from(args.get(i).ok_or_else(usage)?));
            }
            other => return Err(format!("unknown argument {other:?}\n{}", usage())),
        }
        i += 1;
    }
    let shard = DxCodegenShard {
        producer: producer.ok_or_else(usage)?,
        language: language.ok_or_else(usage)?,
        entries,
    };
    let bytes = encode_validated(&shard).map_err(|error| error.to_string())?;
    // Read back before writing so a codec regression fails the action
    // instead of emitting bytes the CLI would reject.
    decode_validated(&bytes).map_err(|error| error.to_string())?;
    std::fs::write(output.ok_or_else(usage)?, bytes).map_err(|error| error.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("codegen_shard_writer: {error}");
        std::process::exit(1);
    }
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

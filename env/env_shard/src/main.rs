//! Shard writer binary: thin CLI shim over the shard codec library.
//!
//! The `dx_env_shard` Starlark rule invokes this writer with one
//! contributor's validated record fields; the binary re-validates and emits
//! the binary `DxEnvShard` protobuf. All record semantics live in the
//! library and are unit-tested there.
//!
//! Usage:
//! ```text
//! env_shard_writer --producer LABEL --integration LANG \
//!   --entry KEY|VALUE [--entry ...] \
//!   --entry KEY|VALUE|EXEC_PATH [--entry ...] \
//!   --output OUT.dxenv.pb
//! ```
//!
//! The two-part entry form declares a logical-only identity input (empty
//! exec path, requiring no materialized artifact). The three-part form
//! declares the BEP-matching exec-path suffix for the backing artifact.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and shard-emission execution, not unit coverage.
use std::path::PathBuf;

use env_shard::{
    decode_validated, encode_validated,
    proto::{DxEnvEntry, DxEnvShard},
};

fn usage() -> String {
    "usage: env_shard_writer --producer LABEL --integration LANG --entry KEY|VALUE[|EXEC] [--entry ...] --output OUT".into()
}

fn parse_entry(raw: &str) -> Result<DxEnvEntry, String> {
    let parts: Vec<&str> = raw.split('|').collect();
    match parts.len() {
        2 => Ok(DxEnvEntry {
            key: parts[0].into(),
            value: parts[1].into(),
            exec_path: String::new(),
        }),
        3 => Ok(DxEnvEntry {
            key: parts[0].into(),
            value: parts[1].into(),
            exec_path: parts[2].into(),
        }),
        _ => Err(format!("bad --entry {raw:?}: want KEY|VALUE[|EXEC_PATH]")),
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let mut producer: Option<String> = None;
    let mut integration: Option<String> = None;
    let mut entries = Vec::new();
    let mut output: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--producer" => {
                i += 1;
                producer = Some(args.get(i).ok_or_else(usage)?.clone());
            }
            "--integration" => {
                i += 1;
                integration = Some(args.get(i).ok_or_else(usage)?.clone());
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
    let shard = DxEnvShard {
        producer: producer.ok_or_else(usage)?,
        integration: integration.ok_or_else(usage)?,
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
        eprintln!("env_shard_writer: {error}");
        std::process::exit(1);
    }
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

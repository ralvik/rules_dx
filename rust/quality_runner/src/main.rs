//! M03 WP2b pipeline runner binary: thin CLI shim over the runner library.
//! All pipeline semantics live in the library and are unit-tested there.
//!
//! Usage:
//! ```text
//! quality_runner --producer LABEL --capability CAP --output OUT.pb \
//!   --stage TOOL;class,class;path,path [--stage ...] \
//!   --source WORKSPACE_PATH=EXEC_PATH [--source ...]
//! ```
//! Stages run in argument order. Each `--source` maps one workspace path
//! to the action-local file holding its bytes. Failures exit nonzero with
//! a message on stderr and write no output.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and WP2c aspect execution, not unit coverage.
use quality_result::encode_validated;
use quality_runner::{run_pipeline, FileInput, StageSpec};

fn main() {
    if let Err(message) = run() {
        eprintln!("quality_runner: {message}");
        std::process::exit(1);
    }
}

fn flag_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_stage(spec: &str) -> Result<StageSpec, String> {
    let (tool_id, rest) = spec
        .split_once(';')
        .ok_or_else(|| format!("malformed --stage {spec:?}, want TOOL;classes;paths"))?;
    let (classes, paths) = rest
        .split_once(';')
        .ok_or_else(|| format!("malformed --stage {spec:?}, want TOOL;classes;paths"))?;
    Ok(StageSpec {
        tool_id: tool_id.to_owned(),
        class_ids: classes.split(',').map(str::to_owned).collect(),
        source_paths: paths.split(',').map(str::to_owned).collect(),
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut producer: Option<String> = None;
    let mut capability: Option<String> = None;
    let mut output: Option<String> = None;
    let mut stages: Vec<StageSpec> = Vec::new();
    let mut sources: Vec<(String, String)> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--producer" => producer = Some(flag_value(&args, &mut index, "--producer")?),
            "--capability" => capability = Some(flag_value(&args, &mut index, "--capability")?),
            "--output" => output = Some(flag_value(&args, &mut index, "--output")?),
            "--stage" => stages.push(parse_stage(&flag_value(&args, &mut index, "--stage")?)?),
            "--source" => {
                let mapping = flag_value(&args, &mut index, "--source")?;
                let (workspace, exec) = mapping
                    .split_once('=')
                    .ok_or_else(|| format!("malformed --source {mapping:?}, want WS_PATH=EXEC"))?;
                sources.push((workspace.to_owned(), exec.to_owned()));
            }
            other => return Err(format!("unknown flag {other:?}")),
        }
        index += 1;
    }
    let producer = producer.ok_or("--producer is required")?;
    let capability = capability.ok_or("--capability is required")?;
    let output = output.ok_or("--output is required")?;
    let mut files = Vec::with_capacity(sources.len());
    for (workspace, exec) in &sources {
        let bytes = std::fs::read(exec).map_err(|e| format!("cannot read {workspace:?}: {e}"))?;
        files.push(FileInput {
            path: workspace.clone(),
            bytes,
        });
    }
    let result = run_pipeline(&producer, &capability, &stages, &files)
        .map_err(|e| format!("pipeline failed: {e}"))?;
    let bytes = encode_validated(&result).map_err(|e| format!("invalid result: {e}"))?;
    std::fs::write(&output, bytes).map_err(|e| format!("cannot write {output:?}: {e}"))?;
    Ok(())
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

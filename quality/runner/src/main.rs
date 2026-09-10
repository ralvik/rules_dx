//! M03 WP2b pipeline runner binary: thin CLI shim over the runner library.
//! All pipeline semantics live in the library and are unit-tested there.
//!
//! Usage:
//! ```text
//! quality_runner --producer LABEL --capability CAP --output OUT.pb \
//!   --stage TOOL;class,class;path,path [--stage ...] \
//!   --source WORKSPACE_PATH=EXEC_PATH [--source ...] \
//!   [--sibling WORKSPACE_PATH=EXEC_PATH [--sibling ...]] \
//!   [--real --tool-binary TOOL=ABS_PATH [--tool-binary ...] \
//!    [--tool-config TOOL=MIRROR_REL] [--tool-file TOOL=MIRROR_REL=EXEC_PATH] \
//!    [--tool-env TOOL=KEY=VALUE] [--scratch-parent PATH]]
//! ```
//! Stages run in argument order. Each `--source` maps one workspace path
//! to the action-local file holding its bytes. Each `--sibling` maps one
//! unclassified link-resolution file (Markdown `--sibling` inputs): sibling
//! bytes are never linted and never enter snapshots. Without `--real` the
//! synthetic M03 pipeline runs. With `--real` the M04 real backend runs
//! `run_real_pipeline` over the resolved tools: each stage tool needs one
//! `--tool-binary`, configs are mirror-relative `--tool-config` paths whose
//! bytes arrive via `--tool-file`, and extra hermetic env entries arrive
//! via `--tool-env`. Scratch trees default under `TMPDIR`. Failures exit
//! nonzero with a message on stderr and write no output.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and WP2c aspect execution, not unit coverage.
use std::collections::BTreeMap;
use std::path::PathBuf;

use quality_result::encode_validated;
use quality_runner::{
    real::{RealBackend, RealTool},
    run_pipeline, FileInput, StageSpec,
};

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

fn parse_tool_binary(spec: &str) -> Result<(String, PathBuf), String> {
    let (tool, path) = spec
        .split_once('=')
        .ok_or_else(|| format!("malformed --tool-binary {spec:?}, want TOOL=ABS_PATH"))?;
    if tool.is_empty() || path.is_empty() {
        return Err(format!(
            "malformed --tool-binary {spec:?}, want TOOL=ABS_PATH"
        ));
    }
    Ok((tool.to_owned(), PathBuf::from(path)))
}

fn parse_tool_config(spec: &str) -> Result<(String, String), String> {
    let (tool, rel) = spec
        .split_once('=')
        .ok_or_else(|| format!("malformed --tool-config {spec:?}, want TOOL=MIRROR_REL"))?;
    if tool.is_empty() || rel.is_empty() {
        return Err(format!(
            "malformed --tool-config {spec:?}, want TOOL=MIRROR_REL"
        ));
    }
    Ok((tool.to_owned(), rel.to_owned()))
}

fn parse_tool_file(spec: &str) -> Result<(String, String, String), String> {
    let (tool, rest) = spec
        .split_once('=')
        .ok_or_else(|| format!("malformed --tool-file {spec:?}, want TOOL=MIRROR_REL=EXEC"))?;
    let (rel, exec) = rest
        .split_once('=')
        .ok_or_else(|| format!("malformed --tool-file {spec:?}, want TOOL=MIRROR_REL=EXEC"))?;
    if tool.is_empty() || rel.is_empty() || exec.is_empty() {
        return Err(format!(
            "malformed --tool-file {spec:?}, want TOOL=MIRROR_REL=EXEC"
        ));
    }
    Ok((tool.to_owned(), rel.to_owned(), exec.to_owned()))
}

fn parse_tool_env(spec: &str) -> Result<(String, String, String), String> {
    let (tool, rest) = spec
        .split_once('=')
        .ok_or_else(|| format!("malformed --tool-env {spec:?}, want TOOL=KEY=VALUE"))?;
    let (key, value) = rest
        .split_once('=')
        .ok_or_else(|| format!("malformed --tool-env {spec:?}, want TOOL=KEY=VALUE"))?;
    if tool.is_empty() || key.is_empty() {
        return Err(format!(
            "malformed --tool-env {spec:?}, want TOOL=KEY=VALUE"
        ));
    }
    Ok((tool.to_owned(), key.to_owned(), value.to_owned()))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut producer: Option<String> = None;
    let mut capability: Option<String> = None;
    let mut output: Option<String> = None;
    let mut stages: Vec<StageSpec> = Vec::new();
    let mut sources: Vec<(String, String)> = Vec::new();
    let mut siblings: Vec<(String, String)> = Vec::new();
    let mut real = false;
    let mut scratch_parent: Option<String> = None;
    let mut binaries: Vec<(String, PathBuf)> = Vec::new();
    let mut configs: Vec<(String, String)> = Vec::new();
    let mut tool_files: Vec<(String, String, String)> = Vec::new();
    let mut tool_env: Vec<(String, String, String)> = Vec::new();
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
            "--sibling" => {
                let mapping = flag_value(&args, &mut index, "--sibling")?;
                let (workspace, exec) = mapping
                    .split_once('=')
                    .ok_or_else(|| format!("malformed --sibling {mapping:?}, want WS_PATH=EXEC"))?;
                siblings.push((workspace.to_owned(), exec.to_owned()));
            }
            "--real" => real = true,
            "--scratch-parent" => {
                scratch_parent = Some(flag_value(&args, &mut index, "--scratch-parent")?);
            }
            "--tool-binary" => {
                binaries.push(parse_tool_binary(&flag_value(
                    &args,
                    &mut index,
                    "--tool-binary",
                )?)?);
            }
            "--tool-config" => {
                configs.push(parse_tool_config(&flag_value(
                    &args,
                    &mut index,
                    "--tool-config",
                )?)?);
            }
            "--tool-file" => {
                tool_files.push(parse_tool_file(&flag_value(
                    &args,
                    &mut index,
                    "--tool-file",
                )?)?);
            }
            "--tool-env" => {
                tool_env.push(parse_tool_env(&flag_value(
                    &args,
                    &mut index,
                    "--tool-env",
                )?)?);
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
    let mut sibling_files = Vec::with_capacity(siblings.len());
    for (workspace, exec) in &siblings {
        let bytes =
            std::fs::read(exec).map_err(|e| format!("cannot read sibling {workspace:?}: {e}"))?;
        sibling_files.push(FileInput {
            path: workspace.clone(),
            bytes,
        });
    }
    if !real {
        let result = run_pipeline(&producer, &capability, &stages, &files)
            .map_err(|e| format!("pipeline failed: {e}"))?;
        let bytes = encode_validated(&result).map_err(|e| format!("invalid result: {e}"))?;
        std::fs::write(&output, bytes).map_err(|e| format!("cannot write {output:?}: {e}"))?;
        return Ok(());
    }
    let mut tools: BTreeMap<String, RealTool> = BTreeMap::new();
    for (tool_id, binary) in binaries {
        if tools.contains_key(&tool_id) {
            return Err(format!("duplicate --tool-binary for {tool_id:?}"));
        }
        // Bazel actions pass exec-root-relative tool paths while the backend
        // spawns from scratch trees under TMPDIR, so resolve relatives against
        // the startup working directory (the action exec root) now. Absolute
        // paths pass through unchanged.
        let absolute = if binary.is_absolute() {
            binary
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(&binary))
                .unwrap_or(binary)
        };
        tools.insert(
            tool_id,
            RealTool {
                binary: absolute,
                extra_env: Vec::new(),
                config_rel: None,
                tool_files: Vec::new(),
            },
        );
    }
    for (tool_id, rel) in configs {
        let tool = tools.get_mut(&tool_id).ok_or_else(|| {
            format!("--tool-config for unknown tool {tool_id:?}: pass --tool-binary first")
        })?;
        if tool.config_rel.is_some() {
            return Err(format!("duplicate --tool-config for {tool_id:?}"));
        }
        tool.config_rel = Some(rel);
    }
    for (tool_id, rel, exec) in tool_files {
        let bytes = std::fs::read(&exec)
            .map_err(|e| format!("cannot read tool file {rel:?} for {tool_id:?}: {e}"))?;
        let tool = tools.get_mut(&tool_id).ok_or_else(|| {
            format!("--tool-file for unknown tool {tool_id:?}: pass --tool-binary first")
        })?;
        tool.tool_files.push((rel, bytes));
    }
    for (tool_id, key, value) in tool_env {
        let tool = tools.get_mut(&tool_id).ok_or_else(|| {
            format!("--tool-env for unknown tool {tool_id:?}: pass --tool-binary first")
        })?;
        tool.extra_env.push((key, value));
    }
    let scratch_parent = scratch_parent
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let backend = RealBackend::new(tools, scratch_parent);
    let result = quality_runner::real::run_real_pipeline_with_siblings(
        &producer,
        &capability,
        &stages,
        &files,
        &sibling_files,
        &backend,
    )
    .map_err(|e| format!("pipeline failed: {e}"))?;
    let bytes = encode_validated(&result).map_err(|e| format!("invalid result: {e}"))?;
    std::fs::write(&output, bytes).map_err(|e| format!("cannot write {output:?}: {e}"))?;
    Ok(())
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

//! M03 WP3 evaluator binary: thin CLI shim over the evaluator library.
//! Threshold semantics live in the library and are unit-tested there.
//!
//! Usage:
//! ```text
//! quality_evaluator --result RESULT.pb --fail_on info|warning|error --output MARKER
//! ```
//! The result is decoded and validated before any threshold comparison, so
//! malformed results fail at every threshold. A passing evaluation writes a
//! deterministic validation marker; a failing one exits nonzero with reasons
//! on stderr and writes no output.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and WP3 evaluator execution, not unit coverage.
use quality_evaluator::{evaluate, parse_threshold};
use quality_result::decode_validated;

fn main() {
    if let Err(message) = run() {
        eprintln!("quality_evaluator: {message}");
        std::process::exit(1);
    }
}

fn flag_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut result_path: Option<String> = None;
    let mut fail_on: Option<String> = None;
    let mut output: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--result" => result_path = Some(flag_value(&args, &mut index, "--result")?),
            "--fail_on" => fail_on = Some(flag_value(&args, &mut index, "--fail_on")?),
            "--output" => output = Some(flag_value(&args, &mut index, "--output")?),
            other => return Err(format!("unknown flag {other:?}")),
        }
        index += 1;
    }
    let result_path = result_path.ok_or("--result is required")?;
    let fail_on = fail_on.ok_or("--fail_on is required")?;
    let output = output.ok_or("--output is required")?;
    let bytes =
        std::fs::read(&result_path).map_err(|e| format!("cannot read {result_path:?}: {e}"))?;
    let result =
        decode_validated(&bytes).map_err(|e| format!("invalid result {result_path:?}: {e:?}"))?;
    let threshold = parse_threshold(&fail_on)?;
    let evaluation = evaluate(&result, threshold);
    if !evaluation.passed {
        return Err(evaluation.reasons.join("; "));
    }
    let marker = format!(
        "validated {} {} fail_on={}\n",
        result.producer,
        result.capability,
        threshold.name()
    );
    std::fs::write(&output, marker).map_err(|e| format!("cannot write {output:?}: {e}"))?;
    Ok(())
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

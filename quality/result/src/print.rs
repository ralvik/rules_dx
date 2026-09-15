//! Layer-2 matrix harness printer (#59): decodes one validated
//! `QualityResult` protobuf into deterministic text for golden diffing.
//!
//! Test-only tooling: prints the producer, capability, stages,
//! convergence, initial/terminal diagnostics, and replacements. Snapshots
//! and digests are omitted (input-identity noise, not behavior).

use quality_result::{decode_validated, proto};

fn severity_name(value: i32) -> &'static str {
    match value {
        1 => "INFO",
        2 => "WARNING",
        3 => "ERROR",
        _ => "UNKNOWN",
    }
}

fn capability_name(value: i32) -> &'static str {
    match value {
        1 => "LINT",
        2 => "TYPECHECK",
        3 => "FORMAT",
        4 => "AUDIT",
        _ => "UNKNOWN",
    }
}

fn convergence_name(value: i32) -> &'static str {
    match value {
        1 => "STABLE",
        2 => "OSCILLATION",
        3 => "ITERATION_LIMIT",
        _ => "UNKNOWN",
    }
}

fn opt_number(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".to_owned())
}

fn rule_name(rule: &str) -> &str {
    if rule.is_empty() {
        "-"
    } else {
        rule
    }
}

fn print_diagnostics(prefix: &str, diagnostics: &[proto::Diagnostic]) {
    println!("{prefix} {}", diagnostics.len());
    for diagnostic in diagnostics {
        println!(
            "{prefix} {} {} {} {} {} {} fixable={} {:?}",
            severity_name(diagnostic.severity),
            diagnostic.tool_id,
            rule_name(&diagnostic.rule_id),
            diagnostic.path,
            opt_number(diagnostic.start_byte),
            opt_number(diagnostic.end_byte),
            diagnostic.fixable,
            diagnostic.message,
        );
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: print_result OUT.pb");
    let bytes = std::fs::read(&path).expect("cannot read result protobuf");
    let result = decode_validated(&bytes).expect("invalid result protobuf");
    println!("producer {}", result.producer);
    println!("capability {}", capability_name(result.capability));
    println!("stages {}", result.stages.len());
    for stage in &result.stages {
        println!(
            "stage {} classes={} sources={}",
            stage.tool_id,
            stage.class_ids.join(","),
            stage.source_paths.join(","),
        );
    }
    println!("completed_rounds {}", result.completed_rounds);
    println!("convergence {}", convergence_name(result.convergence));
    print_diagnostics("initial", &result.initial_diagnostics);
    print_diagnostics("terminal", &result.terminal_diagnostics);
    println!("replacements {}", result.replacements.len());
    for file in &result.replacements {
        for edit in &file.edits {
            println!(
                "replacement {} {} {} {:?}",
                file.path,
                edit.start_byte,
                edit.end_byte,
                String::from_utf8_lossy(&edit.replacement),
            );
        }
    }
}

use serde_json::Value;
use tempfile::tempdir;

fn main() {
    let _ = Value::Null;
    tempdir().unwrap();
}

#[test]
fn demo_runs() {
    main();
}

//! Direct `tests/common.rs`: a test root regardless of basename.
#[test]
fn common_file_is_still_a_root() {
    assert_eq!(api::digest("worker"), api::digest("worker"));
}

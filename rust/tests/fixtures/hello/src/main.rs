//! Thin wrapper over the hello library.
//! All branching logic lives in the library and is unit-tested there.

// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
fn main() -> anyhow::Result<()> {
    let greeting = hello::greet("world")?;
    println!("{greeting}");
    Ok(())
}
// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage

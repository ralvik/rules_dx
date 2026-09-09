//! M00 seed binary: thin wrapper over the hello library.
//! All branching logic lives in the library and is unit-tested there.

// LCOV_EXCL_START - reason: thin binary shim with no branches; behavior verified by build and binary smoke run.
fn main() -> anyhow::Result<()> {
    let greeting = hello::greet("world")?;
    println!("{greeting}");
    Ok(())
}
// LCOV_EXCL_STOP

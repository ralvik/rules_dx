//! M00 coverage gate CLI: thin entry point over the gate library.
//! All branching logic lives in the library and is unit-tested there.

// LCOV_EXCL_START - reason: thin binary shim with no branches; behavior verified by build and gate runs.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(coverage_gate::run(
        &args,
        &|path| std::fs::read_to_string(path).map_err(|err| err.to_string()),
        &mut |line| println!("{line}"),
    ));
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.

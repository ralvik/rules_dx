// Edition-2015 fixture (issue #468): `async` stays a plain identifier
// only under `--edition 2015`, so this file typechecks the per-crate
// edition flow (a single-edition 2021 rustfmt would reject it).
pub fn edition_gate() -> i32 {
    let async = 1;
    async
}

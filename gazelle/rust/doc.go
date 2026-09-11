// Package rust implements the first-party Rust Gazelle extension
// (ADR 0015, M09). It is implemented directly against Gazelle and the
// selected public Rust ruleset APIs. There is no universal language model
// or shared helper layer: helpers are extracted only after a later
// language implementation proves concrete reuse.
package rust

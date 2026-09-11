// Package python implements the first-party Python Gazelle extension
// (ADR 0010, M14). It is implemented directly against Gazelle and the
// selected public Python ruleset APIs. There is no universal language model
// or shared helper layer: helpers are extracted only after a later
// language implementation proves concrete reuse.
package python

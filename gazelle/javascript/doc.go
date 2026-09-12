// Package javascript implements the first-party JavaScript Gazelle extension
// (ADR 0013, M16). It is implemented directly against Gazelle and the
// selected public JavaScript ruleset APIs. There is no universal language model
// or shared helper layer: helpers are extracted only after a later
// language implementation proves concrete reuse.
package javascript

// Package golang implements the first-party Go Gazelle extension (ADR 0019).
// It is implemented directly against Gazelle over the pinned rules_go
// package boundary. There is no shared helper layer: helpers are extracted
// only after a later native foundation proves concrete reuse.
package golang

// Package ruby implements the first-party Ruby Gazelle extension (ADR 0032).
// It is implemented directly against Gazelle over the pinned rules_ruby
// package boundary. There is no shared helper layer: helpers are extracted
// only after a later managed foundation proves concrete reuse.
package ruby

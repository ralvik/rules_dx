// Package greet diff helpers over the pinned go-cmp module (v0.6.0).
// Proves the Go lock wiring: go.mod require plus go.sum plus the shared
// go_deps hub label below via an exact resolve mapping.
package greet

import "github.com/google/go-cmp/cmp"

// Diff returns the cmp.Diff of two strings (empty when equal).
func Diff(x, y string) string {
	return cmp.Diff(x, y)
}

// Equal reports whether x and y are equal via cmp.
func Equal(x, y string) bool {
	return cmp.Diff(x, y) == ""
}

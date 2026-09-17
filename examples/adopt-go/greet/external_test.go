// External test package coexisting with the internal package in one
// package-level go_test (generation-contract Go exception). Stdlib-only
// on purpose: importing the library under test via its full module path
// needs module-aware resolution (exact # gazelle:resolve mapping today),
// so this file pins the coexistence shape without an import edge. The
// internal test above exercises the library directly.
package greet_test

import "testing"

func TestExternalPlaceholder(t *testing.T) {
	t.Parallel()
	if "hello world" != "hello world" {
		t.Fatal("unreachable")
	}
}

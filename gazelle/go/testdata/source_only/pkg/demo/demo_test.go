// Test-owned fixture sources never enter the generated library:
// handwritten dx_go_test owns *_test.go via embed.
package demo_test

import "testing"

// TestPlaceholder pins the fixture without importing the package;
// generation ignores this file entirely.
func TestPlaceholder(t *testing.T) {
	t.Parallel()
	if "hello world" != "hello world" {
		t.Fatal("unreachable")
	}
}

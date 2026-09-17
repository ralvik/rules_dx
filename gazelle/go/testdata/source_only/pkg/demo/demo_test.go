// Test-owned fixture sources never enter the generated library:
// the generated package-level go_test owns *_test.go via embed.
package demo_test

import "testing"

// TestPlaceholder pins the fixture without importing the package;
// generation owns this file via the package-level test.
func TestPlaceholder(t *testing.T) {
	t.Parallel()
	if "hello world" != "hello world" {
		t.Fatal("unreachable")
	}
}

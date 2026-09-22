// Test-owned entry fixture sources never enter the generated library:
// the generated package-level go_test owns *_test.go via embed.
package demo

import "testing"

// TestHelper pins the entry fixture without importing the package;
// generation owns this file via the package-level test.
func TestHelper(t *testing.T) {
	t.Parallel()
	if "hello world" != "hello world" {
		t.Fatal("unreachable")
	}
}

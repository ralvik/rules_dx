// Helper routines for the merge generation fixture.
package demo

import "fmt"

// Announce formats a greeting using the standard library.
func Announce(name string) string {
	return fmt.Sprintf("hello %s", name)
}

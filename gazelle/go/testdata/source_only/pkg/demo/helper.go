// Helper routines for the source-only generation fixture.
package demo

import "fmt"

// Announce formats a greeting using the standard library.
func Announce(name string) string {
	return fmt.Sprintf("hello %s", name)
}

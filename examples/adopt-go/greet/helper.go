// Helper routines for the adopted Go package (stdlib-only).
package greet

import "strings"

// Shout returns s in upper case with an exclamation mark.
func Shout(s string) string {
	return strings.ToUpper(s) + "!"
}

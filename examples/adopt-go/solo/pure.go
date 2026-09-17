// Package solo is the manifest-free-style stdlib-only Go package: no
// imports beyond the standard library, so generation needs no lockfile
// scope and no exact mappings.
package solo

// Double returns twice n.
func Double(n int) int {
	return 2 * n
}

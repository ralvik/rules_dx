// Go from_file fixture library; consumer of go_library with an external dep.
package godeps

import "github.com/google/go-cmp/cmp"

// Diff wraps cmp.Diff for strings (proves the external dep wiring).
func Diff(x, y string) string {
	return cmp.Diff(x, y)
}

// Equal reports whether x and y are equal via cmp.
func Equal(x, y string) bool {
	return cmp.Diff(x, y) == ""
}

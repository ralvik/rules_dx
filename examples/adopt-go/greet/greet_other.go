//go:build !linux

// Non-linux fallback: selected only off linux. Kept tiny so the
// linux-seed CI still type-checks the constraint shape via generation
// (the toolchain drops this file on linux).
package greet

// Platform reports the selected platform family.
func Platform() string {
	return "other"
}

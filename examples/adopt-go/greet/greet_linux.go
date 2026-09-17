//go:build linux

// Linux-selected source: the pinned rules_go toolchain picks this file on
// linux and drops greet_other.go. Generation includes every source and
// never emits select() for build constraints.
package greet

// Platform reports the selected platform family.
func Platform() string {
	return "linux"
}

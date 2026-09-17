// Package greet is the foreign Go package adopted without upstream changes:
// stdlib-only sources plus platform-selected files. Generation owns the
// Bazel package-level library; the Go toolchain selects per platform.
package greet

import "fmt"

// Hello returns a greeting for name.
func Hello(name string) string {
	return fmt.Sprintf("hello %s", name)
}

// Handwritten cgo scope fixture: cgo stays handwritten, never generated.
package cgo

import "C"

// Hello returns a greeting for name.
func Hello(name string) string {
	return "hello " + name
}

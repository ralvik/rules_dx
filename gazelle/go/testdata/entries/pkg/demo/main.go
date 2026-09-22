// Entry fixture: a `main.go` basename in a library package is an ordinary
// library source, never a thin binary. `package main` stays handwritten.
package demo

import "fmt"

// MainGreet returns a greeting for name.
func MainGreet(name string) string {
	return fmt.Sprintf("entry hello %s", name)
}

// Package demo is the source-only generation fixture: one Go package,
// one generated dx_go_library, standard-library imports only.
package demo

// Greet returns a greeting for name.
func Greet(name string) string {
	return "hello " + name
}

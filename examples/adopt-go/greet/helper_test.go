// Shared test helper for the package-level Go test: internal test
// sources coexist in one go_test via embed (generation-contract Go
// exception), so this helper needs no separate target.
package greet

// greetHelper joins s with itself for test assertions.
func greetHelper(s string) string {
	return s + "-" + s
}

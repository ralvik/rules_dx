// Handwritten cgo scope test: embeds the cgo library, exercises race scope.
package cgo

import "testing"

func TestHello(t *testing.T) {
	t.Parallel()
	if got, want := Hello("world"), "hello world"; got != want {
		t.Fatalf("Hello(world) = %q, want %q", got, want)
	}
}

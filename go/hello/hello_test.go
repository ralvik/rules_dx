// M22 seed Go test; consumer of dx_go_test.
package hello

import "testing"

func TestHello(t *testing.T) {
	t.Parallel()
	if got, want := Hello("world"), "hello world"; got != want {
		t.Fatalf("Hello(world) = %q, want %q", got, want)
	}
}

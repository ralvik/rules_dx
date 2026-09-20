// Go from_file fixture test; consumer of go_test with an external dep.
package godeps

import (
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestDiff(t *testing.T) {
	t.Parallel()
	if got := Diff("hello", "hello"); got != "" {
		t.Fatalf("Diff(equal) = %q, want empty", got)
	}
	if got := Diff("hello", "world"); got == "" {
		t.Fatal("Diff(unequal) = empty, want non-empty")
	}
}

func TestEqual(t *testing.T) {
	t.Parallel()
	if !Equal("a", "a") {
		t.Fatal("Equal(a,a) = false, want true")
	}
	if Equal("a", "b") {
		t.Fatal("Equal(a,b) = true, want false")
	}
	// Direct cmp usage proves the external import resolves in tests too.
	if diff := cmp.Diff([]int{1, 2}, []int{1, 3}); diff == "" {
		t.Fatal("cmp.Diff = empty, want non-empty")
	}
}

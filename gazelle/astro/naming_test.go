package astro

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := []struct{ in, want string }{
		{"hello", "hello"},
		{"Hello", "Hello"},
		{"_private", "private"},
		{"trailing_", "trailing"},
		{"my-mod", "my_mod"},
		{"my--mod", "my_mod"},
		{"my mod", "my_mod"},
		{"a.b+c", "a_b_c"},
		{"UPPER123", "UPPER123"},
		{"123", "123"},
		{"déjà", "d_j"},
	}
	for _, tc := range cases {
		if got, err := Normalize(tc.in); err != nil || got != tc.want {
			t.Errorf("Normalize(%q) = %q, %v; want %q", tc.in, got, err, tc.want)
		}
	}
}

func TestNormalizeEmptyFails(t *testing.T) {
	for _, in := range []string{"", "---", "___", "…"} {
		if got, err := Normalize(in); err == nil {
			t.Errorf("Normalize(%q) = %q, want error", in, got)
		}
	}
}

func TestTargetName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.astro", "demo"},
		{"pkg/my-mod.astro", "my_mod"},
		{"Hello.astro", "Hello"},
		{"plain", "plain"},
	}
	for _, tc := range cases {
		if got, err := TargetName(tc.name); err != nil || got != tc.want {
			t.Errorf("TargetName(%q) = %q, %v; want %q", tc.name, got, err, tc.want)
		}
	}
	if _, err := TargetName("---.astro"); err == nil {
		t.Error("TargetName(---.astro) succeeded, want error")
	}
}

func TestModuleName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.astro", "demo"},
		{"pkg/my-mod.astro", "my-mod"},
		{"Hello.astro", "Hello"},
		{"archive.tar.astro", "archive.tar"},
		{"plain", "plain"},
	}
	for _, tc := range cases {
		if got := ModuleName(tc.name); got != tc.want {
			t.Errorf("ModuleName(%q) = %q, want %q", tc.name, got, tc.want)
		}
	}
}

func TestCheckCollisions(t *testing.T) {
	if err := CheckCollisions(nil); err != nil {
		t.Errorf("empty collisions = %v", err)
	}
	single := []Claimant{{Name: "a", Source: "a.astro"}, {Name: "b", Source: "b.astro"}}
	if err := CheckCollisions(single); err != nil {
		t.Errorf("unique collisions = %v", err)
	}
	dupes := []Claimant{{Name: "a_b", Source: "a-b.astro"}, {Name: "a_b", Source: "a_b.astro"}}
	err := CheckCollisions(dupes)
	collision, ok := err.(*CollisionError)
	if !ok {
		t.Fatalf("collisions = %v (%T), want *CollisionError", err, err)
	}
	if collision.Name != "a_b" || len(collision.Claimants) != 2 {
		t.Errorf("collision = %+v", collision)
	}
	message := collision.Error()
	if !strings.Contains(message, "a-b.astro") || !strings.Contains(message, "a_b.astro") {
		t.Errorf("collision message omits a claimant: %s", message)
	}
}

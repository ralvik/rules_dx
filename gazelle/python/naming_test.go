package python

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := []struct{ in, want string }{
		{"hello", "hello"},
		{"hello_test", "hello_test"},
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

func TestIsTestFile(t *testing.T) {
	cases := []struct {
		name string
		want bool
	}{
		{"helper_test.py", true},
		{"pkg/helper_test.py", true},
		{"helper.py", false},
		{"test_helper.py", false},
		{"tests/helper.py", false},
		{"helper_test.pyi", false},
		{"helper_test", false},
		{"_test.py", true},
	}
	for _, tc := range cases {
		if got := IsTestFile(tc.name); got != tc.want {
			t.Errorf("IsTestFile(%q) = %v, want %v", tc.name, got, tc.want)
		}
	}
}

func TestTargetName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.py", "demo"},
		{"pkg/my-mod.py", "my_mod"},
		{"helper_test.py", "helper_test"},
		{"stub.pyi", "stub"},
		{"plain", "plain"},
	}
	for _, tc := range cases {
		if got, err := TargetName(tc.name); err != nil || got != tc.want {
			t.Errorf("TargetName(%q) = %q, %v; want %q", tc.name, got, err, tc.want)
		}
	}
	if _, err := TargetName("---.py"); err == nil {
		t.Error("TargetName(---.py) succeeded, want error")
	}
}

func TestModuleName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.py", "demo"},
		{"pkg/my-mod.py", "my-mod"},
		{"stub.pyi", "stub"},
		{"plain", "plain"},
		{"archive.tar.gz", "archive.tar.gz"},
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
	single := []Claimant{{Name: "a", Source: "a.py"}, {Name: "b", Source: "b.py"}}
	if err := CheckCollisions(single); err != nil {
		t.Errorf("unique collisions = %v", err)
	}
	dupes := []Claimant{{Name: "a_b", Source: "a-b.py"}, {Name: "a_b", Source: "a_b.py"}}
	err := CheckCollisions(dupes)
	collision, ok := err.(*CollisionError)
	if !ok {
		t.Fatalf("collisions = %v (%T), want *CollisionError", err, err)
	}
	if collision.Name != "a_b" || len(collision.Claimants) != 2 {
		t.Errorf("collision = %+v", collision)
	}
	message := collision.Error()
	if !strings.Contains(message, "a-b.py") || !strings.Contains(message, "a_b.py") {
		t.Errorf("collision message omits a claimant: %s", message)
	}
}

func TestIsEntryFile(t *testing.T) {
	cases := []struct {
		name string
		want bool
	}{
		{"main.py", true},
		{"pkg/main.py", true},
		{"main_test.py", false},
		{"helper.py", false},
		{"__main__.py", false},
		{"main.pyi", false},
		{"main", false},
	}
	for _, tc := range cases {
		if got := IsEntryFile(tc.name); got != tc.want {
			t.Errorf("IsEntryFile(%q) = %v, want %v", tc.name, got, tc.want)
		}
	}
}

func TestEntryBinaryName(t *testing.T) {
	if got := EntryBinaryName("main"); got != "main_bin" {
		t.Errorf("EntryBinaryName(main) = %q, want main_bin", got)
	}
	if got := EntryBinaryName("a_b"); got != "a_b_bin" {
		t.Errorf("EntryBinaryName(a_b) = %q, want a_b_bin", got)
	}
}

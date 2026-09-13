package fsharp

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := map[string]string{
		"demo":      "demo",
		"demo-pkg":  "demo_pkg",
		"demo.pkg":  "demo_pkg",
		"demo  pkg": "demo_pkg",
		"a--b..c":   "a_b_c",
		"Ab3":       "Ab3",
		"under_ok":  "under_ok",
	}
	for in, want := range cases {
		got, err := Normalize(in)
		if err != nil || got != want {
			t.Errorf("Normalize(%q) = %q, %v; want %q", in, got, err, want)
		}
	}
	for _, in := range []string{"", "---", "...", "___"} {
		if _, err := Normalize(in); err == nil {
			t.Errorf("Normalize(%q) succeeded, want failure", in)
		}
	}
}

func TestIsTestSource(t *testing.T) {
	if !IsTestSource("DemoTest.fs") || !IsTestSource("pkg/DemoTest.fs") {
		t.Fatal("IsTestSource missed a *Test.fs file")
	}
	for _, name := range []string{"Demo.fs", "TestHelper.fs", "Contest.fs", "DemoTest.fs.bak"} {
		if IsTestSource(name) {
			t.Errorf("IsTestSource(%q) = true, want false", name)
		}
	}
}

func TestDirTargetName(t *testing.T) {
	got, err := DirTargetName("pkg/demo-pkg")
	if err != nil || got != "demo_pkg" {
		t.Errorf("DirTargetName = %q, %v; want demo_pkg", got, err)
	}
}

func TestClassIdentity(t *testing.T) {
	if got := ClassIdentity("pkg/Demo.fs"); got != "Demo" {
		t.Errorf("ClassIdentity = %q, want Demo", got)
	}
}

func TestCheckCollisions(t *testing.T) {
	if err := CheckCollisions([]Claimant{{Name: "a", Source: "A.fs"}, {Name: "b", Source: "B.fs"}}); err != nil {
		t.Errorf("CheckCollisions distinct = %v", err)
	}
	err := CheckCollisions([]Claimant{{Name: "a", Source: "A.fs"}, {Name: "a", Source: "A2.fs"}})
	if err == nil || !strings.Contains(err.Error(), `"a"`) || !strings.Contains(err.Error(), "A.fs") || !strings.Contains(err.Error(), "A2.fs") {
		t.Errorf("CheckCollisions collision = %v, want every claimant", err)
	}
}

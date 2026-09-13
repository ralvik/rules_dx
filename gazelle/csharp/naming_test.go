package csharp

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
	if !IsTestSource("DemoTest.cs") || !IsTestSource("pkg/DemoTest.cs") {
		t.Fatal("IsTestSource missed a *Test.cs file")
	}
	for _, name := range []string{"Demo.cs", "TestHelper.cs", "Contest.cs", "DemoTest.cs.bak"} {
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
	if got := ClassIdentity("pkg/Demo.cs"); got != "Demo" {
		t.Errorf("ClassIdentity = %q, want Demo", got)
	}
}

func TestCheckCollisions(t *testing.T) {
	if err := CheckCollisions([]Claimant{{Name: "a", Source: "A.cs"}, {Name: "b", Source: "B.cs"}}); err != nil {
		t.Errorf("CheckCollisions distinct = %v", err)
	}
	err := CheckCollisions([]Claimant{{Name: "a", Source: "A.cs"}, {Name: "a", Source: "A2.cs"}})
	if err == nil || !strings.Contains(err.Error(), `"a"`) || !strings.Contains(err.Error(), "A.cs") || !strings.Contains(err.Error(), "A2.cs") {
		t.Errorf("CheckCollisions collision = %v, want every claimant", err)
	}
}

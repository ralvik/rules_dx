package python

import (
	"fmt"
	"path"
	"strings"
)

func Normalize(base string) (string, error) {
	var b strings.Builder
	b.Grow(len(base))
	pending := false
	for i := 0; i < len(base); i++ {
		c := base[i]
		if c <= 0x7F && (c == '_' ||
			(c >= 'a' && c <= 'z') ||
			(c >= 'A' && c <= 'Z') ||
			(c >= '0' && c <= '9')) {
			if pending && b.Len() > 0 {
				b.WriteByte('_')
			}
			pending = false
			b.WriteByte(c)
			continue
		}
		pending = true
	}
	out := strings.Trim(b.String(), "_")
	if out == "" {
		return "", fmt.Errorf("naming: %q normalizes to an empty target name", base)
	}
	return out, nil
}

func IsTestFile(name string) bool {
	if !strings.HasSuffix(name, ".py") {
		return false
	}
	stem := strings.TrimSuffix(path.Base(name), ".py")
	return strings.HasSuffix(stem, "_test")
}

func TargetName(name string) (string, error) {
	base := path.Base(name)
	if strings.HasSuffix(base, ".py") {
		base = strings.TrimSuffix(base, ".py")
	} else if strings.HasSuffix(base, ".pyi") {
		base = strings.TrimSuffix(base, ".pyi")
	}
	return Normalize(base)
}

func ModuleName(name string) string {
	base := path.Base(name)
	if strings.HasSuffix(base, ".py") {
		return strings.TrimSuffix(base, ".py")
	}
	if strings.HasSuffix(base, ".pyi") {
		return strings.TrimSuffix(base, ".pyi")
	}
	return base
}

func IsEntryFile(name string) bool {
	if IsTestFile(name) {
		return false
	}
	return path.Base(name) == "main.py"
}

func EntryBinaryName(lib string) string {
	return lib + "_bin"
}

type Claimant struct {
	Name   string
	Source string
	Kind   string
}

type CollisionError struct {
	Name      string
	Claimants []string
}

func (e *CollisionError) Error() string {
	return fmt.Sprintf("naming: normalized name %q claimed by %s; rename a source or keep one target handwritten",
		e.Name, strings.Join(e.Claimants, ", "))
}

func CheckCollisions(claimants []Claimant) error {
	byName := make(map[string][]string, len(claimants))
	order := make([]string, 0, len(claimants))
	for _, c := range claimants {
		if _, ok := byName[c.Name]; !ok {
			order = append(order, c.Name)
		}
		byName[c.Name] = append(byName[c.Name], c.Source)
	}
	for _, name := range order {
		if sources := byName[name]; len(sources) > 1 {
			return &CollisionError{Name: name, Claimants: append([]string(nil), sources...)}
		}
	}
	return nil
}

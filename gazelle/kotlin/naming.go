package kotlin

import (
	"fmt"
	"path"
	"strings"
)

var SupportedExts = []string{".kt"}

const LibraryKind = "kotlin_library"

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

func IsTestSource(name string) bool {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".kt")
	return strings.HasSuffix(stem, "Test")
}

func DirTargetName(dir string) (string, error) {
	return Normalize(path.Base(dir))
}

func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".kt")
	return Normalize(stem)
}

func ClassIdentity(name string) string {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".kt")
	if i := strings.LastIndex(stem, "."); i >= 0 {
		return stem[i+1:]
	}
	return stem
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

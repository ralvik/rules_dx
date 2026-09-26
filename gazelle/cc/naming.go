package cc

import (
	"fmt"
	"path"
	"strings"
)

var SupportedSrcExts = []string{".c", ".cc", ".cpp", ".cxx"}

var SupportedHdrExts = []string{".h", ".hh", ".hpp", ".hxx"}

const LibraryKind = "cc_library"

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
	stem := base
	if i := strings.LastIndexByte(base, '.'); i >= 0 {
		stem = base[:i]
	}
	return strings.HasSuffix(stem, "_test")
}

func isSupportedExt(name string, exts []string) bool {
	for _, ext := range exts {
		if strings.HasSuffix(name, ext) {
			return true
		}
	}
	return false
}

func IsSource(name string) bool {
	return isSupportedExt(path.Base(name), SupportedSrcExts)
}

func IsHeader(name string) bool {
	return isSupportedExt(path.Base(name), SupportedHdrExts)
}

func DirTargetName(dir string) (string, error) {
	return Normalize(path.Base(dir))
}

func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem := base
	if i := strings.LastIndexByte(base, '.'); i >= 0 {
		stem = base[:i]
	}
	return Normalize(stem)
}

func HeaderIdentity(name string) string {
	return path.Base(name)
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

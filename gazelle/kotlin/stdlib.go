// Stdlib lists the Kotlin/JDK import roots treated as standard library by the
// Kotlin Gazelle extension (ADR 0019). Generation treats these imports as
// Kotlin/JDK-provided without an edge, a manifest, or a lockfile. The set covers
// the Kotlin `kotlin.*`/`kotlinx.*` plus the stable `java.*`, `javax.*`, `javafx.*`, `jdk.*`, plus the JDK-bundled
// `org.w3c.*`/`org.xml.*` packages; it is revisited when ADR 0019 freezes the
// JDK baseline.
package kotlin

import "strings"

// stdlibRoots are the dotted-path roots treated as Kotlin/JDK-provided.
var stdlibRoots = []string{
	"kotlin.",
	"kotlinx.",
	"java.",
	"javax.",
	"javafx.",
	"jdk.",
	"org.w3c.",
	"org.xml.",
	"com.sun.",
	"sun.",
}

// IsStdLib reports whether a dotted import path is Kotlin/JDK-provided.
func IsStdLib(spec string) bool {
	for _, root := range stdlibRoots {
		if strings.HasPrefix(spec, root) {
			return true
		}
	}
	return false
}

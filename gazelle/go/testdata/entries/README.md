# Entry naming and package-level generation

Verifies a `main.go` source in a library package stays an ordinary
library source (thin `go_binary` entries are never inferred; `package main`
stays handwritten per `gazelle/go/lang.go`), and `helper_test.go` is owned
by the package-level `go_test` via `embed`. The `go_library` `importpath`
derives from the nearest enclosing `go.mod` (`example.com/entries` plus
the subpath).

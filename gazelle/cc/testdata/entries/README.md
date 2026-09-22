# Entry naming and package-level generation

Verifies a `main.cc` source without a `main` definition stays an ordinary
library source (thin `cc_binary` entries are never inferred; `main`
stays handwritten per `gazelle/cc/lang.go`), and `helper_test.cc` is
test-owned and never enters the library.

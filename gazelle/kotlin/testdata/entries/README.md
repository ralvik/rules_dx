# Entry naming and package-level generation

Verifies a `Main.kt` source without a `main` function stays an ordinary
library source (thin `kotlin_binary` entries are never inferred; `main`
stays handwritten per `gazelle/kotlin/lang.go`), and `HelperTest.kt` is
test-owned and never enters the library.

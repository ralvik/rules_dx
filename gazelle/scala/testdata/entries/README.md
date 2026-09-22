# Entry naming and package-level generation

Verifies a `Main.scala` source without a `main` method stays an ordinary
library source (thin `scala_binary` entries are never inferred; `main`
stays handwritten per `gazelle/scala/lang.go`), and `HelperTest.scala` is
test-owned and never enters the library.

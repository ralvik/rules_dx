# Entry naming and package-level generation

Verifies a `Main.java` source without a `main` method stays an ordinary
library source (thin `java_binary` entries are never inferred; `main`
stays handwritten per `gazelle/java/lang.go`), and `HelperTest.java` is
test-owned and never enters the library.

# Entry naming and package-level generation

Verifies a `Main.cs` source without a `Main` method stays an ordinary
library source (thin `csharp_binary` entries are never inferred; `Main`
stays handwritten per `gazelle/csharp/lang.go`), and `HelperTest.cs` is
test-owned and never enters the library.

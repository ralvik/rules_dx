# Entry naming and package-level generation

Verifies a `Main.fs` source without an `EntryPoint` stays an ordinary
library source (thin `fsharp_binary` entries are never inferred; entry
points stay handwritten per `gazelle/fsharp/lang.go`), and `HelperTest.fs`
is test-owned and never enters the library.

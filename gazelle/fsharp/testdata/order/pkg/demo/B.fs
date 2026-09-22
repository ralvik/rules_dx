namespace Demo

open C
open System.Collections.Generic

type B =
    static member Value() = C.Value() + "b"

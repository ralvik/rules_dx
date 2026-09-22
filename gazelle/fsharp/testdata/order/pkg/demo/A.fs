namespace Demo

open B
open System.Collections.Generic

type A =
    static member Value() = B.Value() + "a"

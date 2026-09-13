// M23 seed F# binary; consumer of dx_fsharp_binary.
module Program

[<EntryPoint>]
let main _ =
    printfn "%s" (Hello.greet "world")
    0

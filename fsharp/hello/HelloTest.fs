// M23 seed F# test; consumer of fsharp_test (plain executable: exit
// code is the verdict; the xUnit/NUnit runner selection stays open under
// O31).
module HelloTest

[<EntryPoint>]
let main _ =
    let got = Hello.greet "world"
    if got <> "hello world" then
        printfn "FAIL: got '%s'" got
        1
    else
        printfn "PASS"
        0

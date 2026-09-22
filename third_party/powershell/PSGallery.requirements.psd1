@{
    # Provisional Gallery requirements (consumed, never written).
    # Exact versions live in PSGallery.lock.json; this manifest records
    # the required modules so depcheck can prove manifest-vs-lock
    # consistency offline. Consumer builds never run Install-Module.
    Pester = '5.7.1'
    PSScriptAnalyzer = '1.25.0'
}

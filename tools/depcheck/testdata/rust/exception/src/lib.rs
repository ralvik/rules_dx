use anyhow::Result;

pub fn greet(name: &str) -> Result<String> {
    if name.is_empty() {
        anyhow::bail!("empty");
    }
    Ok(format!("Hello, {name}!"))
}

// build-plugin is invoked by build.rs codegen; no direct `use` here
// by design (non-import use covered by the explained exception).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

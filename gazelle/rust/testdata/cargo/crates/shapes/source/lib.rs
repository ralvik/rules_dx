use serde_json::Value;

pub fn value() -> Value {
    Value::Null
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    #[test]
    fn creates_tempdir() {
        tempdir().unwrap();
    }
}

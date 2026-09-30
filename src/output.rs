use serde_json::{Value, json};

pub fn success(data: Value) {
    println!(
        "{}",
        serde_json::to_string(&json!({"ok": true, "data": data}))
            .expect("JSON output serialization should succeed")
    );
}

pub fn failure(error: &str) {
    println!(
        "{}",
        serde_json::to_string(&json!({"ok": false, "error": error}))
            .expect("JSON output serialization should succeed")
    );
}

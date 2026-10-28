use serde_json::{json, Value};
use std::io::{self, Read, Write};
mod protocol;

fn action(input: &[u8]) -> Value {
    let result = (|| {
        let envelope: Value = serde_json::from_slice(input).ok()?;
        let text = protocol::command(&envelope, "transform", "text")?.as_str()?;
        let value: Value = serde_json::from_str(text).ok()?;
        let content = serde_json::to_string_pretty(&value).ok()?;
        Some(protocol::success("text", json!(content)))
    })();
    result.unwrap_or_else(protocol::declined)
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    serde_json::to_writer(io::stdout().lock(), &action(&input))?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_json_and_rejects_invalid_text() {
        let request = protocol::test_command("transform", "text", json!("{\"a\":1}"));
        assert!(action(request.to_string().as_bytes())["payload"]["data"]
            .as_str()
            .unwrap()
            .contains("\"a\": 1"));
        let bad = protocol::test_command("transform", "text", json!("not json"));
        assert_eq!(
            action(bad.to_string().as_bytes())["payload"]["kind"],
            "none"
        );
    }
    #[test]
    fn preserves_large_numbers_and_key_order() {
        let request = protocol::test_command(
            "transform",
            "text",
            json!("{\"z\":123456789012345678901234567890,\"a\":1}"),
        );
        let output = action(request.to_string().as_bytes());
        let text = output["payload"]["data"].as_str().unwrap();
        assert!(text.contains("123456789012345678901234567890"));
        assert!(text.find("\"z\"").unwrap() < text.find("\"a\"").unwrap());
    }
    #[test]
    fn legacy_requests_are_not_accepted() {
        let old = json!({"api_version":1,"type":"action","content_type":"text","content":"{}"});
        assert_eq!(
            action(old.to_string().as_bytes())["payload"]["kind"],
            "none"
        );
    }
}

use serde_json::{json, Value};
use std::io::{self, Read, Write};

fn action(input: &[u8]) -> Value {
    let result = (|| {
        let request: Value = serde_json::from_slice(input).ok()?;
        if request["api_version"] != 1
            || request["type"] != "action"
            || request["content_type"] != "text"
        {
            return None;
        }
        let value: Value = serde_json::from_str(request["content"].as_str()?).ok()?;
        let content = serde_json::to_string_pretty(&value).ok()?;
        Some(json!({"api_version":1,"kind":"text","content":content}))
    })();
    result.unwrap_or_else(|| json!({"api_version":1,"supported":false}))
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
        let request =
            json!({"api_version":1,"type":"action","content_type":"text","content":"{\"a\":1}"});
        assert!(action(request.to_string().as_bytes())["content"]
            .as_str()
            .unwrap()
            .contains("\"a\": 1"));
        let bad =
            json!({"api_version":1,"type":"action","content_type":"text","content":"not json"});
        assert_eq!(action(bad.to_string().as_bytes())["supported"], false);
    }
    #[test]
    fn preserves_large_numbers_and_key_order() {
        let request = json!({"api_version":1,"type":"action","content_type":"text","content":"{\"z\":123456789012345678901234567890,\"a\":1}"});
        let output = action(request.to_string().as_bytes());
        let text = output["content"].as_str().unwrap();
        assert!(text.contains("123456789012345678901234567890"));
        assert!(text.find("\"z\"").unwrap() < text.find("\"a\"").unwrap());
    }
}

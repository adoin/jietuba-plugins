use serde_json::{json, Value};
use std::io::{self, Read, Write};

fn preview(input: &[u8]) -> Value {
    serde_json::from_slice::<Value>(input)
        .ok()
        .filter(|request| {
            request["api_version"] == 1
                && request["type"] == "preview"
                && request["content_type"] == "text"
        })
        .and_then(|request| {
            request["content"]
                .as_str()
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
        })
        .filter(|value| value.is_object() || value.is_array())
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .map(|content| json!({"api_version": 1, "kind": "text", "content": content}))
        .unwrap_or_else(|| json!({"api_version": 1, "supported": false}))
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    let response = preview(&input);
    serde_json::to_writer(io::stdout().lock(), &response)?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_json_and_declines_other_text() {
        let input = json!({"api_version": 1, "type": "preview", "content_type": "text", "content": "{\"a\":1}"});
        let result = preview(input.to_string().as_bytes());
        assert_eq!(result["kind"], "text");
        assert!(result["content"]
            .as_str()
            .unwrap()
            .contains("\n  \"a\": 1\n"));
        let other = json!({"api_version": 1, "type": "preview", "content_type": "text", "content": "hello"});
        assert_eq!(preview(other.to_string().as_bytes())["supported"], false);
    }
}

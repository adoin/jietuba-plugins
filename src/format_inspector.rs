use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
mod protocol;

fn preview(input: &[u8]) -> Value {
    let result = (|| {
        let envelope: Value = serde_json::from_slice(input).ok()?;
        let request = protocol::command(&envelope, "render", "clipboard")?;
        if request["mode"] != "formats" {
            return None;
        }
        let names = request["format_names"].as_array()?;
        if names.is_empty() {
            return None;
        }
        let unit = match request["language"].as_str() {
            Some("zh") | Some("zh_TW") => "字节",
            Some("ja") => "バイト",
            Some("ko") => "바이트",
            _ => "bytes",
        };
        let mut output = String::new();
        let selected = request["selected_format"].as_str().unwrap_or("");
        for name in names
            .iter()
            .filter_map(Value::as_str)
            .filter(|name| selected.is_empty() || *name == selected)
        {
            if !output.is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(name);
            if let Some(bytes) = request["formats_base64"][name]
                .as_str()
                .and_then(|s| STANDARD.decode(s).ok())
            {
                output.push_str(&format!("  ({} {})\n", bytes.len(), unit));
                for (line, chunk) in bytes[..bytes.len().min(256)].chunks(16).enumerate() {
                    output.push_str(&format!("{:04x}  ", line * 16));
                    for byte in chunk {
                        output.push_str(&format!("{byte:02x} "));
                    }
                    output.push('\n');
                }
                if bytes.len() > 256 {
                    output.push_str("…");
                }
            }
        }
        Some(protocol::success(
            "preview",
            json!({"kind":"text","content":output}),
        ))
    })();
    result.unwrap_or_else(protocol::declined)
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    serde_json::to_writer(io::stdout().lock(), &preview(&input))?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lists_formats_and_bounded_hex_without_changing_the_data() {
        let input = protocol::test_command(
            "render",
            "clipboard",
            json!({"mode":"formats","format_names":["Example","Large"],"formats_base64":{"Example":STANDARD.encode([0,1,255])}}),
        );
        let response = preview(input.to_string().as_bytes());
        let text = response["payload"]["data"]["content"].as_str().unwrap();
        assert!(text.contains("Example  (3 bytes)"));
        assert!(text.contains("00 01 ff"));
        assert!(text.contains("Large"));
    }
}

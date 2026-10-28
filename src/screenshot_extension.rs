use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::io::{self, Read, Write};

fn respond(input: &[u8]) -> Value {
    let result = (|| {
        let request: Value = serde_json::from_slice(input).ok()?;
        if request["api_version"] != 2
            || request["type"] != "extension"
            || request["event"] != "command"
        {
            return None;
        }
        let payload = &request["payload"];
        let text = match request["command"].as_str()? {
            "dimensions" if payload["kind"] == "image" => {
                let png = STANDARD.decode(payload["data"].as_str()?).ok()?;
                if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
                    return None;
                }
                let width = u32::from_be_bytes(png[16..20].try_into().ok()?);
                let height = u32::from_be_bytes(png[20..24].try_into().ok()?);
                if width == 0 || height == 0 {
                    return None;
                }
                format!("{width}x{height}")
            }
            "format" if payload["kind"] == "text" => {
                let (width, height) = payload["data"].as_str()?.split_once('x')?;
                let (width, height) = (width.parse::<u32>().ok()?, height.parse::<u32>().ok()?);
                format!("{width} × {height} px")
            }
            _ => return None,
        };
        Some(json!({"api_version":2,"ok":true,"payload":{"kind":"text","data":text}}))
    })();
    result.unwrap_or_else(|| json!({"api_version":2,"ok":false}))
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    serde_json::to_writer(io::stdout().lock(), &respond(&input))?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chains_png_dimensions_into_formatted_text() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\x07\x80\0\0\x048";
        let req = json!({"api_version":2,"type":"extension","event":"command","command":"dimensions", "payload":{"kind":"image","data":STANDARD.encode(png)}});
        let first = respond(req.to_string().as_bytes());
        assert_eq!(first["payload"]["data"], "1920x1080");
        let second = json!({"api_version":2,"type":"extension","event":"command","command":"format", "payload":first["payload"]});
        assert_eq!(
            respond(second.to_string().as_bytes())["payload"]["data"],
            "1920 × 1080 px"
        );
        assert_eq!(respond(b"{}")["ok"], false);
    }
}

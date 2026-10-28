use serde_json::{json, Value};
mod protocol;
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

fn markdown(request: &Value) -> Option<String> {
    if request["content_type"] != "file" {
        return None;
    }
    let paths = request["paths"].as_array()?;
    if paths.len() != 1 {
        return None;
    }
    let path = paths[0].as_str()?;
    if path.starts_with("\\\\") || path.starts_with("//") {
        return None;
    }
    let path = Path::new(path);
    if !path.extension()?.eq_ignore_ascii_case("md") {
        return None;
    }
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 1024 * 1024 || bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    let response = serde_json::from_slice::<Value>(&input)
        .ok()
        .and_then(|request| protocol::command(&request, "render", "clipboard").and_then(markdown))
        .map(|content| protocol::success("preview", json!({"kind":"markdown","content":content})))
        .unwrap_or_else(protocol::declined);
    serde_json::to_writer(io::stdout().lock(), &response)?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_network_and_non_markdown_files() {
        for path in ["\\\\server\\share\\doc.md", "C:\\note.txt"] {
            let request = json!({"content_type":"file","paths":[path]});
            assert!(markdown(&request).is_none());
        }
    }
}

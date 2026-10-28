use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
mod protocol;

fn sensitive(text: &str) -> bool {
    if [
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "-----BEGIN DSA PRIVATE KEY-----",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        return true;
    }
    for token in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '"' | '\'' | '(' | ')' | ',' | ';' | '=' | ':')
    }) {
        let t = token.trim();
        if t.len() == 20
            && (t.starts_with("AKIA") || t.starts_with("ASIA"))
            && t.bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return true;
        }
        if ((t.starts_with("sk-") && t.len() >= 24)
            || (t.starts_with("github_pat_") && t.len() >= 30)
            || (["ghp_", "gho_", "ghu_", "ghs_", "ghr_"]
                .iter()
                .any(|prefix| t.starts_with(prefix))
                && t.len() >= 30))
            && t.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return true;
        }
        let parts: Vec<_> = t.split('.').collect();
        if parts.len() == 3
            && parts.iter().all(|part| part.len() >= 8)
            && parts[0].starts_with("eyJ")
        {
            if let Ok(header) = URL_SAFE_NO_PAD.decode(parts[0]) {
                if serde_json::from_slice::<Value>(&header)
                    .ok()
                    .and_then(|v| v.get("alg").and_then(Value::as_str).map(str::to_owned))
                    .is_some()
                {
                    return true;
                }
            }
        }
    }
    false
}

fn filter(input: &[u8]) -> Value {
    let allow = serde_json::from_slice::<Value>(input)
        .ok()
        .and_then(|v| {
            protocol::hook(&v, "clipboard.before_capture", "clipboard")
                .and_then(|data| data["content"].as_str().map(|s| !sensitive(s)))
        })
        .unwrap_or(false);
    protocol::success("decision", json!(allow))
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    serde_json::to_writer(io::stdout().lock(), &filter(&input))?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catches_high_confidence_secrets_and_keeps_ordinary_text() {
        assert!(sensitive("-----BEGIN OPENSSH PRIVATE KEY-----\nabc"));
        assert!(sensitive("AKIA1234567890ABCDEF"));
        assert!(sensitive("AWS_ACCESS_KEY_ID=AKIA1234567890ABCDEF"));
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256"}"#);
        assert!(sensitive(&format!("{header}.YWJjZGVmZ2g.YWJjZGVmZ2g")));
        assert!(!sensitive("The status is okay and the key is below."));
        assert!(!sensitive("AKIA123 is an incomplete example"));
    }
    #[test]
    fn invalid_request_fails_closed() {
        assert_eq!(filter(b"not json")["payload"]["data"], false);
        let good = json!({"api_version":2,"type":"extension","event":"clipboard.before_capture","command":"","payload":{"kind":"clipboard","data":{"content":"ordinary text"}}});
        assert_eq!(filter(good.to_string().as_bytes())["payload"]["data"], true);
    }
}

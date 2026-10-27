use serde_json::{json, Value};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

fn app_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name).trim()
}

fn allowed(source: &str, config: &Value) -> bool {
    let source = app_name(source);
    if source.is_empty() {
        return true;
    }
    !config["ignored_apps"]
        .as_str()
        .unwrap_or("")
        .lines()
        .map(app_name)
        .filter(|line| !line.is_empty())
        .any(|line| line.eq_ignore_ascii_case(source))
}

fn read_config() -> io::Result<Value> {
    let Some(path) = std::env::var_os("JIETUBA_PLUGIN_CONFIG") else {
        return Ok(json!({}));
    };
    let path = Path::new(&path);
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(json!({})),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "configuration too large",
        ));
    }
    let config: Value = serde_json::from_slice(&bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if !config.is_object() || config.get("ignored_apps").is_some_and(|v| !v.is_string()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid configuration",
        ));
    }
    Ok(config)
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    let request: Value = serde_json::from_slice(&input)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if request["api_version"] != 1 || request["type"] != "capture_filter" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid request",
        ));
    }
    let response = json!({"api_version":1,"allow":allowed(request["source_app"].as_str().unwrap_or(""), &read_config()?)});
    serde_json::to_writer(io::stdout().lock(), &response)?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignores_exact_executable_names_without_matching_other_apps() {
        let config = json!({"ignored_apps":"PasswordManager.exe\nC:\\Tools\\secret.exe\n"});
        assert!(!allowed("passwordmanager.EXE", &config));
        assert!(!allowed("C:\\Tools\\secret.exe", &config));
        assert!(allowed("my-passwordmanager.exe", &config));
        assert!(allowed("", &config));
        assert!(allowed("browser.exe", &json!({})));
    }
}

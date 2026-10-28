use rhai::{
    packages::{Package, StandardPackage},
    Engine, ImmutableString, Scope,
};
use serde_json::{json, Value};
mod protocol;
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

fn script_from_config() -> io::Result<String> {
    let Some(path) = std::env::var_os("JIETUBA_PLUGIN_CONFIG") else {
        return Ok("content.to_upper()".into());
    };
    let file = match fs::File::open(Path::new(&path)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok("content.to_upper()".into())
        }
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
    if !config.is_object() || config.get("script").is_some_and(|v| !v.is_string()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid configuration",
        ));
    }
    Ok(config["script"]
        .as_str()
        .unwrap_or("content.to_upper()")
        .to_owned())
}

fn transform(content: &str, source_app: &str, script: &str) -> Option<String> {
    let mut engine = Engine::new_raw();
    StandardPackage::new().register_into_engine(&mut engine);
    engine
        .set_max_operations(100_000)
        .set_max_call_levels(32)
        .set_max_string_size(1024 * 1024);
    let mut scope = Scope::new();
    scope.push("content", content.to_owned());
    scope.push("source_app", source_app.to_owned());
    match engine.eval_with_scope::<ImmutableString>(&mut scope, script) {
        Ok(value) => Some(value.to_string()),
        Err(_error) => {
            #[cfg(test)]
            eprintln!("{_error}");
            None
        }
    }
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    let request: Value = serde_json::from_slice(&input)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let content = protocol::command(&request, "transform", "text")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid request"))?;
    let result = transform(
        content.as_str().unwrap_or(""),
        request["context"]["source_app"].as_str().unwrap_or(""),
        &script_from_config()?,
    );
    let response = result
        .map(|content| protocol::success("text", json!(content)))
        .unwrap_or_else(protocol::declined);
    serde_json::to_writer(io::stdout().lock(), &response)?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_user_scripts_and_limits_unbounded_work() {
        assert_eq!(
            transform(
                "  hello  ",
                "x.exe",
                "let text = content; text.trim(); text.to_upper()"
            ),
            Some("HELLO".into())
        );
        assert_eq!(
            transform("hello", "x.exe", "source_app + ':' + content"),
            Some("x.exe:hello".into())
        );
        assert!(transform("hello", "", "loop {} ").is_none());
        assert!(transform("hello", "", "import \"x\" as x; content").is_none());
    }
}

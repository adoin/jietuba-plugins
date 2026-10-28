#![allow(dead_code)] // Each plugin uses the relevant parts of the shared wire protocol.
use serde_json::{json, Value};

pub fn command<'a>(request: &'a Value, command: &str, kind: &str) -> Option<&'a Value> {
    (request["api_version"] == 2
        && request["type"] == "extension"
        && request["event"] == "command"
        && request["command"] == command
        && request["payload"]["kind"] == kind)
        .then_some(&request["payload"]["data"])
}

pub fn hook<'a>(request: &'a Value, event: &str, kind: &str) -> Option<&'a Value> {
    (request["api_version"] == 2
        && request["type"] == "extension"
        && request["event"] == event
        && request["command"] == ""
        && request["payload"]["kind"] == kind)
        .then_some(&request["payload"]["data"])
}

pub fn success(kind: &str, data: Value) -> Value {
    json!({"api_version":2,"ok":true,"payload":{"kind":kind,"data":data}})
}

pub fn declined() -> Value {
    json!({"api_version":2,"ok":true,"payload":{"kind":"none"}})
}

#[cfg(test)]
pub fn test_command(command: &str, kind: &str, data: Value) -> Value {
    json!({"api_version":2,"type":"extension","event":"command","command":command,"payload":{"kind":kind,"data":data},"context":{}})
}

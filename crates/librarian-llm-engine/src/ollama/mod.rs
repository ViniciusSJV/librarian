//! Ollama adapter. Model selection is explicit; no domain policy is embedded.
pub mod client;
use serde_json::{json, Value};

pub fn request(query: &str, model: &str) -> Result<Value, String> {
    request_with(query, model, true)
}

/// `structured` false leaves the response schema out of the request (no `format`): a model that
/// reasons in a separate channel can answer nothing when the schema constrains its output. The
/// local checker validates the answer either way.
pub fn request_with(query: &str, model: &str, structured: bool) -> Result<Value, String> {
    if model.trim().is_empty() {
        return Err("Modelo deve ser explícito".into());
    }
    let value: Value = serde_json::from_str(query).map_err(|e| e.to_string())?;
    if !value["question"]
        .as_str()
        .is_some_and(|q| !q.trim().is_empty())
        || !value["evidence"].is_object()
        || !value["instructions"].as_array().is_some_and(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty()))
        })
    {
        return Err("Consulta deve conter question, evidence e instructions válidos".into());
    }
    let mut body = json!({"model": model, "prompt": query, "stream": false});
    if let Some(schema) = value.get("response_schema").filter(|_| structured) {
        if !schema.is_object() || schema["type"] != "object" {
            return Err("response_schema deve ser um JSON Schema de objeto".into());
        }
        // Optional provider mapping; legacy queries keep their exact request shape.
        // The local checker still validates the actual output, irrespective of format.
        body["format"] = schema.clone();
    }
    Ok(body)
}

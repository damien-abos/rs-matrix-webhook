use std::collections::HashMap;

use anyhow::Result;
use mlua::{Lua, Value as LuaValue};
use serde_json::Value;

/// Executes `script`, calls its `format(data, headers)` function, and returns the result.
///
/// A fresh Lua state is created per call so formatters are stateless and thread-safe.
pub fn call_formatter(
    script: &str,
    data: Value,
    headers: &HashMap<String, String>,
) -> Result<Value> {
    run(script, data, headers).map_err(|e| anyhow::anyhow!("Lua error: {}", e))
}

fn run(script: &str, data: Value, headers: &HashMap<String, String>) -> mlua::Result<Value> {
    let lua = Lua::new();

    lua.load(script).exec()?;

    let format_fn: mlua::Function = lua.globals().get("format").map_err(|_| {
        mlua::Error::RuntimeError(
            "Lua formatter must define a top-level `format` function".to_string(),
        )
    })?;

    let data_table = json_to_lua(&lua, &data)?;

    let headers_table = lua.create_table()?;
    for (k, v) in headers {
        headers_table.set(k.as_str(), v.as_str())?;
    }

    let result: LuaValue = format_fn.call((data_table, LuaValue::Table(headers_table)))?;
    lua_to_json(result)
}

// ---------------------------------------------------------------------------
// JSON ↔ Lua conversions (use mlua::Result to avoid Send+Sync constraints)
// ---------------------------------------------------------------------------

fn json_to_lua(lua: &Lua, value: &Value) -> mlua::Result<LuaValue> {
    match value {
        Value::Null => Ok(LuaValue::Nil),
        Value::Bool(b) => Ok(LuaValue::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else {
                Ok(LuaValue::Number(n.as_f64().unwrap_or(0.0)))
            }
        }
        Value::String(s) => Ok(LuaValue::String(lua.create_string(s.as_bytes())?)),
        Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        Value::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), json_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

fn lua_to_json(value: LuaValue) -> mlua::Result<Value> {
    match value {
        LuaValue::Nil => Ok(Value::Null),
        LuaValue::Boolean(b) => Ok(Value::Bool(b)),
        LuaValue::Integer(i) => Ok(Value::Number(i.into())),
        LuaValue::Number(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| {
                mlua::Error::RuntimeError(format!("Cannot represent float {} as JSON number", f))
            }),
        LuaValue::String(s) => Ok(Value::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => {
            let len = t.raw_len();

            // Treat as array when keys 1..=len are all present.
            if len > 0 {
                let mut arr: Vec<Value> = Vec::with_capacity(len);
                let mut is_array = true;
                for i in 1..=(len as i64) {
                    match t.get::<LuaValue>(i) {
                        Ok(v) => arr.push(lua_to_json(v)?),
                        Err(_) => {
                            is_array = false;
                            break;
                        }
                    }
                }
                if is_array {
                    return Ok(Value::Array(arr));
                }
            }

            // Otherwise treat as object.
            let mut map = serde_json::Map::new();
            for pair in t.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;
                let key = match k {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    LuaValue::Integer(i) => i.to_string(),
                    LuaValue::Number(f) => f.to_string(),
                    _ => continue,
                };
                map.insert(key, lua_to_json(v)?);
            }
            Ok(Value::Object(map))
        }
        _ => Ok(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sets_body_field() {
        let script = r#"
            function format(data, headers)
                data.body = "hello from lua"
                return data
            end
        "#;
        let result = call_formatter(script, json!({}), &HashMap::new()).unwrap();
        assert_eq!(result["body"], "hello from lua");
    }

    #[test]
    fn reads_data_fields() {
        let script = r#"
            function format(data, headers)
                data.body = "value=" .. data.input
                return data
            end
        "#;
        let result = call_formatter(script, json!({"input": "test"}), &HashMap::new()).unwrap();
        assert_eq!(result["body"], "value=test");
    }

    #[test]
    fn reads_headers() {
        let script = r#"
            function format(data, headers)
                data.body = headers["x-event"]
                return data
            end
        "#;
        let mut headers = HashMap::new();
        headers.insert("x-event".to_string(), "push".to_string());
        let result = call_formatter(script, json!({}), &headers).unwrap();
        assert_eq!(result["body"], "push");
    }

    #[test]
    fn returns_array_as_json_array() {
        let script = r#"
            function format(data, headers)
                data.items = {1, 2, 3}
                data.body = "ok"
                return data
            end
        "#;
        let result = call_formatter(script, json!({}), &HashMap::new()).unwrap();
        assert_eq!(result["items"], json!([1, 2, 3]));
    }

    #[test]
    fn missing_format_function_is_error() {
        let script = "-- no format function defined";
        let err = call_formatter(script, json!({}), &HashMap::new());
        assert!(err.is_err());
    }

    #[test]
    fn syntax_error_is_error() {
        let script = "function format(data end";
        let err = call_formatter(script, json!({}), &HashMap::new());
        assert!(err.is_err());
    }
}

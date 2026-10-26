use std::collections::BTreeMap;

use serde_yaml_ng::Value;

pub fn parse(text: &str) -> Result<BTreeMap<String, String>, ()> {
    let value =
        serde_yaml_ng::from_str::<Value>(text.trim_start_matches('\u{feff}')).map_err(|_| ())?;
    let mut out = BTreeMap::new();
    flatten(value, "", &mut out);
    Ok(out)
}

fn child(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

fn key(value: Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Null => Some("null".to_owned()),
        Value::Tagged(tagged) => key(tagged.value),
        Value::Sequence(_) | Value::Mapping(_) => None,
    }
}

fn flatten(value: Value, path: &str, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Mapping(map) => {
            for (raw_key, value) in map {
                if let Some(key) = key(raw_key) {
                    flatten(value, &child(path, &key), out);
                }
            }
        }
        Value::Sequence(items) => {
            for (index, value) in items.into_iter().enumerate() {
                flatten(value, &format!("{path}[{index}]"), out);
            }
        }
        Value::Tagged(tagged) => flatten(tagged.value, path, out),
        Value::String(value) => {
            out.insert(path.to_owned(), value);
        }
        Value::Bool(value) => {
            out.insert(path.to_owned(), value.to_string());
        }
        Value::Number(value) => {
            out.insert(path.to_owned(), value.to_string());
        }
        Value::Null => {
            out.insert(path.to_owned(), "null".to_owned());
        }
    }
}

//! Consuming flatteners avoid retaining a second plaintext config tree.

use std::collections::BTreeMap;

fn child(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

pub fn json(value: serde_json::Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    flatten_json(value, "", &mut out);
    out
}

fn flatten_json(value: serde_json::Value, path: &str, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                flatten_json(value, &child(path, &key), out);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, value) in items.into_iter().enumerate() {
                flatten_json(value, &format!("{path}[{index}]"), out);
            }
        }
        serde_json::Value::String(value) => {
            out.insert(path.to_owned(), value);
        }
        scalar => {
            out.insert(path.to_owned(), scalar.to_string());
        }
    }
}

pub fn toml(value: toml::Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    flatten_toml(value, "", &mut out);
    out
}

fn flatten_toml(value: toml::Value, path: &str, out: &mut BTreeMap<String, String>) {
    match value {
        toml::Value::Table(map) => {
            for (key, value) in map {
                flatten_toml(value, &child(path, &key), out);
            }
        }
        toml::Value::Array(items) => {
            for (index, value) in items.into_iter().enumerate() {
                flatten_toml(value, &format!("{path}[{index}]"), out);
            }
        }
        toml::Value::String(value) => {
            out.insert(path.to_owned(), value);
        }
        scalar => {
            out.insert(path.to_owned(), scalar.to_string());
        }
    }
}

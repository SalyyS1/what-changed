use std::collections::BTreeMap;

pub fn parse(bytes: &[u8]) -> Result<BTreeMap<String, String>, ()> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let value = serde_json::from_slice(bytes).map_err(|_| ())?;
    Ok(super::flatten::json(value))
}

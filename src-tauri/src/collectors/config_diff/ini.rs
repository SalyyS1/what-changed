use std::collections::BTreeMap;

pub fn parse(text: &str, sections: bool) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut section = String::new();
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if sections && line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_owned();
            continue;
        }
        let pair = line.split_once('=').or_else(|| line.split_once(':'));
        let Some((key, value)) = pair else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let path = if sections && !section.is_empty() {
            format!("{section}.{key}")
        } else {
            key.to_owned()
        };
        out.insert(path, value.trim().to_owned());
    }
    out
}

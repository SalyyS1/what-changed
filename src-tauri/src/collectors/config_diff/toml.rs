use std::collections::BTreeMap;

pub fn parse(text: &str) -> Result<BTreeMap<String, String>, ()> {
    let value =
        toml::from_str::<toml::Value>(text.trim_start_matches('\u{feff}')).map_err(|_| ())?;
    Ok(super::flatten::toml(value))
}

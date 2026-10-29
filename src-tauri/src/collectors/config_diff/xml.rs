use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

pub fn parse(bytes: &[u8]) -> Result<BTreeMap<String, String>, ()> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut out = BTreeMap::new();
    let mut stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event().map_err(|_| ())? {
            Event::Start(start) => {
                let path = element_path(&stack, &start)?;
                attributes(&reader, &start, &path, &mut out)?;
                stack.push(path);
            }
            Event::Empty(start) => {
                let path = element_path(&stack, &start)?;
                attributes(&reader, &start, &path, &mut out)?;
            }
            Event::Text(text) => {
                let Some(path) = stack.last() else {
                    continue;
                };
                let decoded = text.decode().map_err(|_| ())?;
                let value = quick_xml::escape::unescape(&decoded).map_err(|_| ())?;
                if !value.trim().is_empty() {
                    let key = if path.is_empty() {
                        "#text".to_owned()
                    } else {
                        format!("{path}.#text")
                    };
                    out.insert(key, value.trim().to_owned());
                }
            }
            Event::CData(text) => {
                let Some(path) = stack.last() else {
                    continue;
                };
                let value = text.decode().map_err(|_| ())?;
                if !value.trim().is_empty() {
                    let key = if path.is_empty() {
                        "#text".to_owned()
                    } else {
                        format!("{path}.#text")
                    };
                    out.insert(key, value.trim().to_owned());
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

fn element_path(stack: &[String], start: &BytesStart<'_>) -> Result<String, ()> {
    let name = std::str::from_utf8(start.name().as_ref())
        .map_err(|_| ())?
        .to_owned();
    Ok(match stack.last() {
        None => String::new(),
        Some(parent) if parent.is_empty() => name,
        Some(parent) => format!("{parent}.{name}"),
    })
}

fn attributes(
    reader: &Reader<&[u8]>,
    start: &BytesStart<'_>,
    path: &str,
    out: &mut BTreeMap<String, String>,
) -> Result<(), ()> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|_| ())?;
        let key = std::str::from_utf8(attribute.key.as_ref()).map_err(|_| ())?;
        let value = attribute
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|_| ())?;
        let field = if path.is_empty() {
            format!("@{key}")
        } else {
            format!("{path}.@{key}")
        };
        out.insert(field, value.into_owned());
    }
    Ok(())
}

//! Friendly, data-driven names for Windows device-driver changes.

/// Windows device class to the noun shown in the timeline.
///
/// The table is deliberately data: adding a class does not add another branch
/// to event phrasing, and unknown classes still get an honest fallback.
const CLASS_NOUNS: &[(&str, &str)] = &[
    ("Display", "graphics driver"),
    ("Net", "network driver"),
    ("AudioEndpoint", "audio driver"),
    ("MEDIA", "audio driver"),
    ("USB", "USB driver"),
];

/// Human noun for a SetupAPI class name.
pub fn class_noun(class_name: &str) -> String {
    CLASS_NOUNS
        .iter()
        .find(|(class, _)| class.eq_ignore_ascii_case(class_name.trim()))
        .map(|(_, noun)| (*noun).to_owned())
        .unwrap_or_else(|| {
            let class_name = class_name.trim();
            if class_name.is_empty() {
                "device driver".to_owned()
            } else {
                format!("{class_name} driver")
            }
        })
}

/// Manufacturer's first token, stripped of punctuation common in registry
/// publisher values. An empty manufacturer stays absent rather than becoming
/// an invented brand.
pub fn brand(manufacturer: &str) -> Option<String> {
    manufacturer
        .split_whitespace()
        .next()
        .map(|token| token.trim_matches(|character: char| character == ',' || character == ';'))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}

/// Stable display label shared by every driver event phrase.
pub fn friendly_name(manufacturer: &str, class_name: &str) -> String {
    let noun = class_noun(class_name);
    brand(manufacturer).map_or(noun.clone(), |brand| format!("{brand} {noun}"))
}

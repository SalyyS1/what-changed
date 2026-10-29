//! The two scrubbing entry points every sink goes through.
//!
//! `scrub_value` is for a value whose key is known — a config entry, an event
//! field — and applies the key rule first, because a value under
//! `DB_PASSWORD` is a secret whatever it looks like. `scrub_line` is for text
//! with no structure to lean on: a log line, a command line, a title. It finds
//! `key=value` and `"key": "value"` itself so a sensitive name inside free
//! text still hides its value.

use std::borrow::Cow;

use super::keys::is_sensitive_key;
use super::patterns::{redact_shapes, Kind, REDACTED};

/// Shortest key the pair scanner will accept. A single letter before a colon is
/// a drive letter or a time field, never a config key.
const MIN_KEY_LEN: usize = 2;

/// Characters that end an unquoted value. Whitespace is one of them: a log line
/// is mostly prose, and eating the words after a secret loses the sentence that
/// explains it. An unquoted value with a space in it is not a shape credentials
/// come in.
fn ends_unquoted_value(c: char) -> bool {
    c.is_whitespace() || matches!(c, ',' | ';' | '"' | '\'')
}

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// Where the value opened by `quote` ends, skipping any quote the writer
/// escaped.
///
/// Two rules, both learned from what a truncated value publishes. Only the
/// matching quote closes the value, so an apostrophe inside a double-quoted
/// password is part of it. And a backslash-escaped quote — which is how JSON
/// writes one — is part of it too. Getting either wrong ends the value early and
/// leaves the rest of the secret in the clear.
fn closing_quote(inner: &str, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (index, c) in inner.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            return Some(index);
        }
    }
    None
}

/// The key immediately to the left of `separator`, if there is one.
///
/// Walks back over optional spaces and one optional quote, then over the key
/// itself, and requires it to start like an identifier — which is what keeps
/// `C:` in a path and `14:22` in a timestamp from being read as keys.
fn key_before(line: &str, separator: usize) -> Option<(usize, &str)> {
    let head = line.get(..separator)?;
    let trimmed = head.trim_end_matches([' ', '\t']);
    let trimmed = trimmed.strip_suffix(['"', '\'']).unwrap_or(trimmed);
    let start = trimmed
        .char_indices()
        .rev()
        .take_while(|&(_, c)| is_key_char(c))
        .last()
        .map(|(i, _)| i)?;
    let key = trimmed.get(start..)?;
    let first = key.chars().next()?;
    if key.len() < MIN_KEY_LEN || !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    Some((start, key))
}

/// Where the value after `separator` starts and ends.
fn value_span(line: &str, separator: usize) -> Option<(usize, usize)> {
    let after = separator + 1;
    let rest = line.get(after..)?;
    let offset = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    let body = rest.get(offset..)?;
    let start = after + offset;

    if let Some(quote) = body.chars().next().filter(|&c| matches!(c, '"' | '\'')) {
        // A quoted value may contain spaces; it ends at the matching quote. With
        // no matching quote the line ended inside the value, so the value is the
        // rest of it: there is no sentence after an unterminated quote worth
        // preserving, and skipping the pair would leave the credential in the
        // clear. This branch is only reached for a key already judged sensitive,
        // so when the two readings disagree the safe one is to take more.
        let inner = body.get(1..)?;
        let end = closing_quote(inner, quote).unwrap_or(inner.len());
        return Some((start + 1, start + 1 + end));
    }

    let first = body.find(ends_unquoted_value).unwrap_or(body.len());
    if first == 0 {
        return None;
    }
    // Absorb an already-redacted tail. A shape rule may have replaced the
    // credential that followed the word this value stops at — an
    // `Authorization: Bearer …` header is the everyday case — and two adjacent
    // markers say nothing the first one did not.
    let mut end = first;
    loop {
        let tail = body.get(end..).unwrap_or_default();
        let skipped = tail.len() - tail.trim_start_matches([' ', '\t']).len();
        let Some(next) = tail.get(skipped..) else {
            break;
        };
        if next.starts_with(REDACTED) {
            end += skipped + REDACTED.len();
        } else {
            break;
        }
    }
    Some((start, start + end))
}

/// Replace the value of every `key=value` or `"key": "value"` whose key is
/// sensitive.
///
/// A hand-rolled scan rather than one regex: a regex value pattern is greedy
/// enough to swallow the next pair on the line, so `config: DB_PASSWORD=x`
/// would hide the inner pair from the rule that was meant to catch it.
fn scrub_pairs(line: &str) -> Cow<'_, str> {
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut cursor = 0;

    while let Some(offset) = line.get(cursor..).and_then(|rest| rest.find([':', '='])) {
        let separator = cursor + offset;
        cursor = separator + 1;

        let Some((_, key)) = key_before(line, separator) else {
            continue;
        };
        if !is_sensitive_key(key) {
            continue;
        }
        let Some((start, end)) = value_span(line, separator) else {
            continue;
        };
        // Nothing to do when a shape rule already replaced exactly this value.
        if line.get(start..end) == Some(REDACTED) {
            cursor = end;
            continue;
        }

        let buffer = out.get_or_insert_with(String::new);
        buffer.push_str(line.get(copied..start).unwrap_or_default());
        buffer.push_str(REDACTED);
        copied = end;
        cursor = end;
    }

    match out {
        Some(mut buffer) => {
            buffer.push_str(line.get(copied..).unwrap_or_default());
            Cow::Owned(buffer)
        }
        None => Cow::Borrowed(line),
    }
}

/// Outcome of scrubbing one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scrubbed<'a> {
    /// The value as it may be stored: borrowed when nothing changed.
    pub text: Cow<'a, str>,
    /// Rules that fired, in the order they were applied. Empty means the value
    /// passed through untouched.
    pub kinds: Vec<Kind>,
}

impl Scrubbed<'_> {
    /// Did anything get replaced?
    pub fn redacted(&self) -> bool {
        !self.kinds.is_empty()
    }
}

/// Scrub one value, optionally with the key it is filed under.
///
/// A sensitive key replaces the whole value: the shape rules would keep a
/// prefix or a host, and for a value the user already told us is a credential
/// there is nothing worth keeping.
pub fn scrub_value<'a>(key: Option<&str>, value: &'a str) -> Scrubbed<'a> {
    if key.is_some_and(is_sensitive_key) {
        return Scrubbed {
            text: Cow::Borrowed(REDACTED),
            kinds: vec![Kind::KeyName],
        };
    }

    let (text, mut kinds) = redact_shapes(value);
    let pair_scrubbed = scrub_pairs(&text);
    if pair_scrubbed != text {
        kinds.push(Kind::KeyName);
        return Scrubbed {
            text: Cow::Owned(pair_scrubbed.into_owned()),
            kinds,
        };
    }
    if kinds.is_empty() {
        Scrubbed {
            text: Cow::Borrowed(value),
            kinds,
        }
    } else {
        Scrubbed {
            text: Cow::Owned(text.into_owned()),
            kinds,
        }
    }
}

/// Scrub a line of free text.
///
/// Shape rules run first: some of them read the words around a credential —
/// `Bearer` before its token, a scheme before a URI's userinfo — and the pair
/// scan would remove that context. The pair scan then catches what has no
/// recognisable shape at all, which is most passwords.
pub fn scrub_line(line: &str) -> Cow<'_, str> {
    let (after_shapes, _) = redact_shapes(line);

    match scrub_pairs(&after_shapes) {
        Cow::Borrowed(_) => after_shapes,
        Cow::Owned(text) => Cow::Owned(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_key_immediately_left_of_the_separator() {
        assert_eq!(
            key_before("DB_PASSWORD=x", 11).map(|(_, k)| k),
            Some("DB_PASSWORD")
        );
        assert_eq!(
            key_before(r#"{"session_id": "x"}"#, 13).map(|(_, k)| k),
            Some("session_id")
        );
        // A drive letter and a clock field are not keys.
        assert_eq!(key_before(r"path C:\tools", 6), None);
        assert_eq!(key_before("2026-09-03T14:22:07Z", 13), None);
    }

    #[test]
    fn a_sensitive_key_replaces_the_whole_value() {
        let out = scrub_value(Some("DB_PASSWORD"), "hunter2");
        assert_eq!(out.text, REDACTED);
        assert_eq!(out.kinds, vec![Kind::KeyName]);
    }

    #[test]
    fn an_ordinary_key_leaves_an_ordinary_value_borrowed() {
        let out = scrub_value(Some("API_PORT"), "8080");
        assert_eq!(out.text, "8080");
        assert!(!out.redacted());
        assert!(
            matches!(out.text, Cow::Borrowed(_)),
            "an untouched value must not be copied"
        );
    }

    #[test]
    fn a_shape_fires_under_an_innocuous_key() {
        let out = scrub_value(Some("BUILD_SIGNATURE"), "sk-abcdefghijklmnopqrstuvwx");
        assert_eq!(out.text, REDACTED);
        assert_eq!(out.kinds, vec![Kind::ApiKey]);
    }

    #[test]
    fn scrubs_a_pair_inside_free_text() {
        let line = "loaded config: DB_PASSWORD=hunter2, API_PORT=8080";
        assert_eq!(
            scrub_line(line),
            format!("loaded config: DB_PASSWORD={REDACTED}, API_PORT=8080")
        );
    }

    #[test]
    fn scrub_value_without_a_key_also_scrubs_assignment_like_secrets() {
        for input in [
            "token=secret",
            "PASSWORD='two words'",
            "password=secret done",
        ] {
            let out = scrub_value(None, input);
            assert!(!out.text.contains("secret"), "{input} -> {}", out.text);
            assert!(out.redacted(), "{input} must be redacted");
        }
        assert_eq!(
            scrub_value(None, "Port 3000 now listening").text,
            "Port 3000 now listening"
        );
    }

    #[test]
    fn keeps_the_words_after_a_redacted_value() {
        let line = "config DB_PASSWORD=hunter2 loaded";
        assert_eq!(
            scrub_line(line),
            format!("config DB_PASSWORD={REDACTED} loaded")
        );
    }

    #[test]
    fn scrubs_every_sensitive_pair_on_one_line() {
        let line = "API_KEY=abc123 API_PORT=8080 SESSION_ID=zzz999";
        assert_eq!(
            scrub_line(line),
            format!("API_KEY={REDACTED} API_PORT=8080 SESSION_ID={REDACTED}")
        );
    }

    #[test]
    fn keeps_a_quoted_value_with_spaces_inside_the_quotes() {
        let line = r#"PASSWORD="two words" and more"#;
        assert_eq!(
            scrub_line(line),
            format!(r#"PASSWORD="{REDACTED}" and more"#)
        );
    }

    #[test]
    fn a_value_whose_quote_is_never_closed_is_still_hidden() {
        // A truncated log line, or a config file with a typo. The safe reading
        // is that the value runs to the end: skipping the pair would publish it.
        let line = r#"PASSWORD="hunter2 and then the line ended"#;
        let out = scrub_line(line);
        assert_eq!(out, format!(r#"PASSWORD="{REDACTED}"#));
        assert!(!out.contains("hunter2"), "{out}");
    }

    #[test]
    fn an_escaped_quote_does_not_end_the_value_early() {
        // How JSON writes a quote inside a string. Reading it as the closing one
        // hides the first half of the password and prints the second.
        let line = r#"{"password": "ab\"cdefghijklmnop"}"#;
        let out = scrub_line(line);
        assert!(!out.contains("cdefghij"), "{out}");
        assert_eq!(out, format!(r#"{{"password": "{REDACTED}"}}"#));
    }

    #[test]
    fn the_other_kind_of_quote_does_not_end_the_value() {
        // An apostrophe inside a double-quoted value is part of the value.
        let line = r#"PASSWORD="it's a long one" and the line goes on"#;
        let out = scrub_line(line);
        assert!(!out.contains("s a long"), "{out}");
        assert_eq!(
            out,
            format!(r#"PASSWORD="{REDACTED}" and the line goes on"#)
        );
    }

    #[test]
    fn scrubs_a_quoted_json_pair() {
        let line = r#"{"session_id": "abc123def456", "user": "salyy"}"#;
        let out = scrub_line(line);
        assert!(
            out.contains(&format!(r#""session_id": "{REDACTED}""#)),
            "{out}"
        );
        assert!(
            out.contains(r#""user": "salyy""#),
            "an ordinary field must survive: {out}"
        );
    }

    #[test]
    fn scrubs_a_shape_with_no_key_in_sight() {
        let line = "request failed: Authorization: Bearer abcdefghijklmnopqrstuvwxyz012345";
        assert_eq!(
            scrub_line(line),
            format!("request failed: Authorization: {REDACTED}")
        );
    }

    #[test]
    fn a_sensitive_key_cannot_leave_a_credential_behind_it() {
        // The pair scan stops an unquoted value at whitespace, so the token
        // after `Bearer` has to be caught by a shape rule and folded into the
        // same marker rather than left in the clear.
        let line = "AUTHORIZATION=Bearer abcdefghijklmnopqrstuvwxyz012345 done";
        let out = scrub_line(line);
        assert!(!out.contains("abcdefgh"), "{out}");
        assert_eq!(out, format!("AUTHORIZATION={REDACTED} done"));
    }

    #[test]
    fn leaves_an_ordinary_line_untouched_and_unallocated() {
        let line = "Port 3000 now listening (node.exe)";
        let out = scrub_line(line);
        assert_eq!(out, line);
        assert!(
            matches!(out, Cow::Borrowed(_)),
            "an untouched line must not be copied"
        );
    }

    #[test]
    fn leaves_a_path_that_contains_a_sensitive_word() {
        let line = r"watching C:\Users\dev\.ssh\id_rsa.pub";
        assert_eq!(scrub_line(line), line);
    }
}

//! Key-name redaction: deciding that a value is a secret from the name it is
//! filed under, before anything looks at the value itself.
//!
//! Two rules. A normalized whole-name match against a fixed list, and a
//! last-word match so that `STRIPE_SECRET_KEY` and `my_custom_token` are
//! caught without being listed. The last-word rule reads the *original* name,
//! split on separators and case transitions, because the alternative — a plain
//! suffix test on the normalized name — makes `MONKEY` a secret.

/// Lowercase and drop the characters people use to separate words in a key,
/// so `API_KEY`, `api-key`, `Api.Key` and `apiKey` all become `apikey`.
pub fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|c| !matches!(c, '_' | '-' | '.' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whole normalized names that are secrets on their own.
const SENSITIVE_NAMES: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "apikey",
    "authorization",
    // British spelling: config files in the wild carry both, and a redactor
    // that only knows one of them leaks on the other.
    "authorisation",
    "cookie",
    "session",
    // `SESSIONID` written glued has no word boundary for the rule below to
    // find, and a session id is a credential.
    "sessionid",
    "privatekey",
    "clientsecret",
    "accesskey",
    "refreshtoken",
    "bearer",
    "credential",
    "passphrase",
    // Connection URLs routinely embed credentials (`scheme://user:pass@…`).
    // Treat the whole value as secret even when its current sample does not.
    "url",
];

/// Words that make the value a secret when they are the last word of the key.
/// `key` is here rather than in a suffix test so that only a real word
/// boundary counts. Plurals are listed because `API_KEYS` is as sensitive as
/// `API_KEY`, while `HOTKEYS` and `MONKEYS` are whole words that match neither.
const SENSITIVE_WORDS: &[&str] = &[
    "password",
    "passwords",
    "passwd",
    "pwd",
    "secret",
    "secrets",
    "token",
    "tokens",
    "key",
    "keys",
    "cookie",
    "cookies",
    "session",
    "sessions",
    "credential",
    "credentials",
    "passphrase",
    "bearer",
    "authorization",
    "authorisation",
    "url",
    "urls",
];

/// Endings that make a *glued* last word sensitive: `PGPASSWORD`,
/// `NGROK_AUTHTOKEN`, `VAULT_ROLESECRET`. `key` is deliberately absent — no
/// English word ends in `password` or `authtoken` by accident, but plenty end
/// in `key`, and `MONKEY` is not a credential.
const GLUED_SUFFIXES: &[&str] = &[
    "password",
    "passwd",
    "passphrase",
    "token",
    "secret",
    "credential",
    "credentials",
];

/// Trailing words that qualify the thing before them without changing what it
/// is, so `AWS_ACCESS_KEY_ID` is still a key.
const QUALIFIERS: &[&str] = &["id", "ids", "value", "values"];

/// Substrings that every name, word and glued suffix above contains. A key with
/// none of them cannot be sensitive, so it never reaches the splitting path.
/// The pair scanner asks this question once per `:` or `=` on a line, which on a
/// Windows path or a timestamp is often, and the answer has to be cheap.
const STEMS: &[&str] = &[
    "key",
    "token",
    "secret",
    "pass",
    "pwd",
    "cookie",
    "session",
    "credential",
    "bearer",
    "auth",
    "url",
];

/// Longest key the stem filter lowercases on the stack. Anything longer is
/// unusual enough that the full rule can have it.
const STACK_KEY_LEN: usize = 96;

/// Could this key be sensitive at all? False is a definite no.
fn could_be_sensitive(key: &str) -> bool {
    let bytes = key.as_bytes();
    if bytes.len() > STACK_KEY_LEN {
        return true;
    }
    let mut buffer = [0u8; STACK_KEY_LEN];
    for (slot, &byte) in buffer.iter_mut().zip(bytes) {
        *slot = byte.to_ascii_lowercase();
    }
    // Lowercasing ASCII bytes leaves any multi-byte sequence untouched, so the
    // slice is still valid UTF-8; the fallback keeps that assumption honest
    // rather than assuming it.
    match std::str::from_utf8(&buffer[..bytes.len()]) {
        Ok(lower) => STEMS.iter().any(|stem| lower.contains(stem)),
        Err(_) => true,
    }
}

/// Split a key into words on separators and on case transitions, lowercased.
///
/// `AWSAccessKey` splits as `aws`, `access`, `key`: an uppercase run that runs
/// into a lowercase letter belongs to the following word, which is what keeps
/// acronyms readable.
fn words(key: &str) -> Vec<String> {
    let chars: Vec<char> = key.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();

    for (i, &c) in chars.iter().enumerate() {
        if matches!(c, '_' | '-' | '.' | ' ') {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            continue;
        }

        let previous = if i == 0 {
            None
        } else {
            chars.get(i - 1).copied()
        };
        let next = chars.get(i + 1).copied();

        let starts_word = match (previous, next) {
            // lower/digit → upper is always a boundary: `apiKey`.
            (Some(p), _) if c.is_uppercase() && (p.is_lowercase() || p.is_numeric()) => true,
            // upper → upper followed by lower ends an acronym: `AWSAccess`.
            (Some(p), Some(n)) if c.is_uppercase() && p.is_uppercase() && n.is_lowercase() => true,
            _ => false,
        };

        if starts_word && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        current.extend(c.to_lowercase());
    }

    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Does the name alone mean the value must be redacted?
///
/// Deliberately over-redacts a few names whose values are not secrets —
/// `PUBLIC_KEY`, `PRIMARY_KEY`, `LICENSE_KEY` — because the cost of hiding a
/// value the user could have seen is a worse timeline entry, and the cost of
/// the opposite is a leaked credential.
pub fn is_sensitive_key(key: &str) -> bool {
    if !could_be_sensitive(key) {
        return false;
    }
    if SENSITIVE_NAMES.contains(&normalize_key(key).as_str()) {
        return true;
    }

    let words = words(key);
    let mut tail = words.as_slice();
    while let Some((last, rest)) = tail.split_last() {
        if QUALIFIERS.contains(&last.as_str()) && !rest.is_empty() {
            tail = rest;
        } else {
            return SENSITIVE_WORDS.contains(&last.as_str())
                || GLUED_SUFFIXES.iter().any(|suffix| last.ends_with(suffix));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cheap_filter_never_rejects_something_the_rules_would_catch() {
        // The filter is a fast path in front of a security rule: it may only
        // ever say "definitely not", so every term the rules match on has to
        // survive it.
        for term in SENSITIVE_NAMES
            .iter()
            .chain(SENSITIVE_WORDS)
            .chain(GLUED_SUFFIXES)
        {
            assert!(
                could_be_sensitive(term),
                "{term} was rejected before the rules ran"
            );
            assert!(
                could_be_sensitive(&term.to_uppercase()),
                "{term} was rejected in upper case"
            );
        }
    }

    #[test]
    fn the_cheap_filter_rejects_an_ordinary_name() {
        for key in [
            "API_PORT",
            "hostname",
            "displayName",
            "version",
            "path",
            "pid",
        ] {
            assert!(
                !could_be_sensitive(key),
                "{key} should not reach the full rule"
            );
        }
    }

    #[test]
    fn normalizes_every_spelling_of_the_same_key() {
        for key in [
            "API_KEY", "api-key", "Api.Key", "apiKey", "api key", "ApiKey",
        ] {
            assert_eq!(normalize_key(key), "apikey", "{key}");
        }
    }

    #[test]
    fn splits_on_separators_and_case_transitions() {
        assert_eq!(words("AWS_ACCESS_KEY_ID"), ["aws", "access", "key", "id"]);
        assert_eq!(words("apiKey"), ["api", "key"]);
        assert_eq!(words("AWSAccessKey"), ["aws", "access", "key"]);
        assert_eq!(words("MONKEY"), ["monkey"]);
        assert_eq!(words("db.password"), ["db", "password"]);
        assert_eq!(words("x2Token"), ["x2", "token"]);
    }

    #[test]
    fn redacts_the_documented_sensitive_names() {
        for key in [
            "password",
            "PASSWD",
            "pwd",
            "secret",
            "TOKEN",
            "API_KEY",
            "api-key",
            "Api.Key",
            "apiKey",
            "Authorization",
            "authorisation",
            "Cookie",
            "SESSION",
            "PRIVATE_KEY",
            "private.key",
            "CLIENT_SECRET",
            "ACCESS_KEY",
            "REFRESH_TOKEN",
            "bearer",
            "CREDENTIAL",
            "PASSPHRASE",
        ] {
            assert!(is_sensitive_key(key), "{key} must be treated as a secret");
        }
    }

    #[test]
    fn redacts_by_last_word_whatever_the_prefix() {
        for key in [
            "X_AUTH_TOKEN",
            "db_password",
            "DB_PASSWD",
            "STRIPE_SECRET_KEY",
            "my_custom_token",
            "GPG_PASSPHRASE",
            "AWS_SECRET_ACCESS_KEY",
            "githubToken",
            "USER_SESSION",
            "vault.client.secret",
            "DATABASE_URL",
            "redisUrl",
            "UPSTREAM_URLS",
        ] {
            assert!(is_sensitive_key(key), "{key} must be treated as a secret");
        }
    }

    #[test]
    fn treats_a_trailing_qualifier_as_part_of_the_same_thing() {
        for key in ["SESSION_ID", "ACCESS_KEY_ID", "API_KEY_VALUE", "sessionId"] {
            assert!(is_sensitive_key(key), "{key} must be treated as a secret");
        }
        // The qualifier only qualifies; it cannot make an ordinary name secret.
        for key in ["USER_ID", "REQUEST_ID", "EVENT_IDS"] {
            assert!(!is_sensitive_key(key), "{key} must survive");
        }
    }

    #[test]
    fn catches_a_glued_name_with_no_word_boundary() {
        for key in [
            "PGPASSWORD",
            "MYSQL_ROOT_PASSWORD",
            "NGROK_AUTHTOKEN",
            "SESSIONID",
            "sessionid",
            "VAULT_ROLESECRET",
            "API_KEYS",
            "KEYS",
        ] {
            assert!(is_sensitive_key(key), "{key} must be treated as a secret");
        }
    }

    #[test]
    fn leaves_names_that_only_look_sensitive() {
        for key in [
            "API_PORT",
            "KEYBOARD_LAYOUT",
            "TOKENIZER_PATH",
            "SESSION_TIMEOUT_MS",
            "TOKEN_BUCKET_SIZE",
            "SECRETS_DIR",
            "PASSWORD_MIN_LENGTH",
            "HOSTKEY_ALGORITHM",
            "KEY_VAULT_NAME",
            "CREDENTIAL_PATH",
            "BEARER_PREFIX",
            "MONKEY",
            "DONKEY",
            "TURKEY",
            "HOTKEYS",
            "MONKEYS",
            "OLDPWD",
            "CURL_TIMEOUT",
            "URL_RETRY_COUNT",
        ] {
            assert!(!is_sensitive_key(key), "{key} must survive redaction");
        }
    }

    #[test]
    fn accepts_the_documented_over_redaction() {
        // Not secrets, but the last word is `key` and the rule cannot tell.
        // Recorded here so a future change to the rule shows up as a diff in
        // this list rather than as a silent behaviour change.
        for key in [
            "PUBLIC_KEY",
            "SSH_PUBLIC_KEY",
            "PRIMARY_KEY",
            "FOREIGN_KEY",
            "SORT_KEY",
            "PARTITION_KEY",
            "CACHE_KEY",
            "IDEMPOTENCY_KEY",
            "LICENSE_KEY",
            "PRODUCT_KEY",
            "KEY",
        ] {
            assert!(is_sensitive_key(key), "{key} is over-redacted on purpose");
        }
    }
}

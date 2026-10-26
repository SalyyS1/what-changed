//! Value-shape redaction rules.
//!
//! Credentials that announce themselves — a vendor prefix, a JWT's three
//! segments, a PEM header, a URI's userinfo — are matched by shape, whatever
//! key they sit under, because secrets get pasted into innocuously named
//! fields. The last rule is the generic one and defers to the entropy gate.
//!
//! Patterns are literals in this file and compiled once. A literal that fails
//! to compile is skipped rather than panicking at the first event: a change
//! recorder that crashes when it meets a secret is worse than one running with
//! eight rules, and `every_pattern_compiles` below turns the missing rule into
//! a failed build.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::{Regex, RegexSet};

use super::entropy;

/// What replaces a secret. Nothing derived from the value is kept: no digest,
/// no prefix, no length-preserving mask.
pub const REDACTED: &str = "[REDACTED]";

/// Which rule fired, recorded alongside the redaction so a user can see *why*
/// a value is hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The key name alone was enough.
    KeyName,
    /// Vendor API key with a recognisable prefix.
    ApiKey,
    /// The token part of an `Authorization: Bearer` value.
    Bearer,
    /// JSON web token.
    Jwt,
    /// AWS access key identifier, long-term or temporary.
    AwsAccessKey,
    /// PEM private key block.
    PrivateKey,
    /// Source-host personal or OAuth token.
    HostToken,
    /// Chat platform token.
    ChatToken,
    /// Credentials in a URI's userinfo position.
    UriUserinfo,
    /// Long opaque string that passed the entropy gate.
    HighEntropy,
}

impl Kind {
    /// Stable name used in redaction metadata and test assertions.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeyName => "key_name",
            Self::ApiKey => "api_key",
            Self::Bearer => "bearer",
            Self::Jwt => "jwt",
            Self::AwsAccessKey => "aws_access_key",
            Self::PrivateKey => "private_key",
            Self::HostToken => "host_token",
            Self::ChatToken => "chat_token",
            Self::UriUserinfo => "uri_userinfo",
            Self::HighEntropy => "high_entropy",
        }
    }
}

/// How much of a match to replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Replace the entire match.
    Whole,
    /// Replace one capture group and keep the rest, so `Bearer` and a URI's
    /// scheme and host stay readable.
    Group(usize),
    /// Replace the entire match only if the entropy gate agrees.
    WholeIfDense,
}

/// A compiled shape rule.
pub struct Rule {
    /// Reason recorded when this rule fires.
    pub kind: Kind,
    /// Compiled pattern.
    pub regex: Regex,
    /// Extent of the replacement.
    pub action: Action,
}

/// The rule table, most specific first. The generic entropy rule is last so a
/// recognisable shape is always attributed to its own kind.
const PATTERNS: &[(Kind, &str, Action)] = &[
    (
        Kind::PrivateKey,
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
        Action::Whole,
    ),
    (
        Kind::Jwt,
        r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
        Action::Whole,
    ),
    (Kind::ApiKey, r"sk-[A-Za-z0-9_-]{16,}", Action::Whole),
    (
        Kind::AwsAccessKey,
        r"(?:AKIA|ASIA)[0-9A-Z]{16}",
        Action::Whole,
    ),
    (
        Kind::HostToken,
        r"gh[pousr]_[A-Za-z0-9]{20,}",
        Action::Whole,
    ),
    (
        Kind::ChatToken,
        r"xox[baprs]-[A-Za-z0-9-]{10,}",
        Action::Whole,
    ),
    (Kind::Bearer, r"(?i)bearer\s+(\S{12,})", Action::Group(1)),
    // Scheme and host survive; only the userinfo is replaced. Covers
    // postgres, mysql, mongodb+srv, redis and amqp without naming them.
    //
    // Deliberately not anchored on a word boundary. A formatter may write the
    // value straight after a word character — the console layer's ANSI reset
    // ends in `m` — and `\b` there stops the rule from ever starting, which let
    // `postgres://user:pass@host` through while `mongodb+srv://` survived only
    // because its `+` happened to offer a legal start position. The scheme is
    // not what gets replaced, so nothing is lost by matching it loosely.
    (
        Kind::UriUserinfo,
        r"(?i)[a-z][a-z0-9+.-]*://([^\s/@:]+:[^\s/@]+)@",
        Action::Group(1),
    ),
    (
        Kind::HighEntropy,
        r"[A-Za-z0-9+/]{40,}={0,2}",
        Action::WholeIfDense,
    ),
];

/// Compiled rules, built once per process.
pub static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .filter_map(|&(kind, pattern, action)| {
            Regex::new(pattern).ok().map(|regex| Rule {
                kind,
                regex,
                action,
            })
        })
        .collect()
});

/// The eight rules that begin with, or contain, a literal — as one automaton,
/// used only to answer "could any of them match this text?" in a single pass.
/// Almost every line the app sees carries no credential at all, and this is what
/// makes that case cost one scan and no allocation instead of nine of each.
///
/// The generic entropy rule is deliberately **not** in here. `[A-Za-z0-9+/]{40,}`
/// contains no literal, and one such pattern in a set removes the regex engine's
/// prefilter for the whole set: instead of skipping ahead to the next `sk-` or
/// `://` with a memory-scan instruction, the union automaton has to visit every
/// byte. `has_dense_run` below answers the same question for that rule at a
/// fraction of the cost.
static LITERAL_RULE_SET: LazyLock<Option<RegexSet>> = LazyLock::new(|| {
    RegexSet::new(
        PATTERNS
            .iter()
            .filter(|&&(_, _, action)| !matches!(action, Action::WholeIfDense))
            .map(|&(_, pattern, _)| pattern),
    )
    .ok()
});

/// Necessary condition for the generic rule: a run of at least `MIN_LEN`
/// characters it could match. A byte loop rather than an automaton, because that
/// rule's whole problem is having nothing for an automaton to skip ahead to.
///
/// It may over-answer — a long hex hash has such a run and is not a secret — and
/// costs only a full pass of the ordered rules when it does. It may never
/// under-answer: this is the only thing standing between the entropy rule and a
/// line it never gets to see.
fn has_dense_run(input: &str) -> bool {
    let mut run = 0usize;
    for &byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'+' || byte == b'/' {
            run += 1;
            if run >= entropy::MIN_LEN {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// Replace every secret shape in `input`, reporting which kinds fired.
///
/// Rules are applied in table order, so a JWT is attributed to `jwt` rather
/// than to the generic rule that would also have matched its segments. Once the
/// pre-filter says some rule can match, every rule runs: skipping the ones it
/// did not name would let one replacement join two halves of a shape that
/// nothing afterwards looks at again.
pub fn redact_shapes(input: &str) -> (Cow<'_, str>, Vec<Kind>) {
    // Both filters have to agree there is nothing here. A set that failed to
    // compile answers "not sure", which means the rules run.
    let no_literal = LITERAL_RULE_SET
        .as_ref()
        .is_some_and(|set| !set.is_match(input));
    if no_literal && !has_dense_run(input) {
        return (Cow::Borrowed(input), Vec::new());
    }

    let mut current = Cow::Borrowed(input);
    let mut fired = Vec::new();

    for rule in RULES.iter() {
        let mut hit = false;
        let replaced = rule
            .regex
            .replace_all(&current, |caps: &regex::Captures<'_>| {
                let Some(whole) = caps.get(0) else {
                    return String::new();
                };
                match rule.action {
                    Action::Whole => {
                        hit = true;
                        REDACTED.to_owned()
                    }
                    Action::WholeIfDense => {
                        if entropy::looks_high_entropy(whole.as_str()) {
                            hit = true;
                            REDACTED.to_owned()
                        } else {
                            whole.as_str().to_owned()
                        }
                    }
                    Action::Group(index) => match caps.get(index) {
                        Some(group) => {
                            hit = true;
                            let start = group.start() - whole.start();
                            let end = group.end() - whole.start();
                            let text = whole.as_str();
                            format!("{}{REDACTED}{}", &text[..start], &text[end..])
                        }
                        None => whole.as_str().to_owned(),
                    },
                }
            })
            .into_owned();

        if hit {
            fired.push(rule.kind);
        }
        current = Cow::Owned(replaced);
    }

    (current, fired)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_compiles() {
        // The rule table is a security control: a pattern lost to a typo must
        // fail the build, not thin out redaction at runtime.
        assert_eq!(RULES.len(), PATTERNS.len());
    }

    #[test]
    fn the_two_pre_filters_cover_the_whole_table_between_them() {
        // The literal set holds every rule but the generic one, and the generic
        // one is covered by the run scan. If a future rule with no literal is
        // added to the table without a gate of its own, this fails.
        let literal_count = LITERAL_RULE_SET
            .as_ref()
            .map_or(0, |set| set.patterns().len());
        let dense_count = PATTERNS
            .iter()
            .filter(|&&(_, _, action)| matches!(action, Action::WholeIfDense))
            .count();
        assert_eq!(dense_count, 1, "only the entropy rule may lack a literal");
        assert_eq!(literal_count + dense_count, PATTERNS.len());
    }

    #[test]
    fn the_run_scan_finds_what_the_generic_rule_needs() {
        // Sound in the direction that matters: anything the entropy rule could
        // fire on has a run this finds.
        assert!(has_dense_run(
            "Zm9vYmFyYmF6cXV1eEhlbGxvV29ybGREZWFkQmVlZjEyMzQ1Ng=="
        ));
        assert!(has_dense_run(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ));
        // A separator resets the run, which is why config lines are cheap.
        assert!(!has_dense_run(
            "PATH=C:\\Program Files\\nodejs;API_PORT=8080;BUILD=10.0.26200.1234"
        ));
        assert!(!has_dense_run("short"));
        // Exactly at the threshold, and one short of it.
        assert!(has_dense_run(&"a".repeat(40)));
        assert!(!has_dense_run(&"a".repeat(39)));
    }

    #[test]
    fn kind_names_are_unique_and_stable() {
        // The names end up in stored metadata, so two kinds must never share
        // one and a rename must be a deliberate, visible change.
        let mut names = [
            Kind::KeyName.as_str(),
            Kind::ApiKey.as_str(),
            Kind::Bearer.as_str(),
            Kind::Jwt.as_str(),
            Kind::AwsAccessKey.as_str(),
            Kind::PrivateKey.as_str(),
            Kind::HostToken.as_str(),
            Kind::ChatToken.as_str(),
            Kind::UriUserinfo.as_str(),
            Kind::HighEntropy.as_str(),
        ];
        let count = names.len();
        names.sort_unstable();
        let mut distinct = names.to_vec();
        distinct.dedup();
        assert_eq!(distinct.len(), count);
    }

    #[test]
    fn redacts_each_shape_and_names_the_rule() {
        let cases = [
            ("sk-abcdefghijklmnopqrstuvwx", Kind::ApiKey),
            (
                "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dQw4w9WgXcQdQw4w9WgXcQ",
                Kind::Jwt,
            ),
            ("AKIAIOSFODNN7EXAMPLE", Kind::AwsAccessKey),
            ("ASIAIOSFODNN7EXAMPLE", Kind::AwsAccessKey),
            ("-----BEGIN RSA PRIVATE KEY-----", Kind::PrivateKey),
            ("ghp_abcdefghijklmnopqrstuvwxyz0123456789", Kind::HostToken),
            ("xoxb-123456789012-abcdefghijkl", Kind::ChatToken),
        ];
        for (input, kind) in cases {
            let (out, fired) = redact_shapes(input);
            assert_eq!(out, REDACTED, "{input} must be replaced whole");
            assert!(
                fired.contains(&kind),
                "{input} must be attributed to {}",
                kind.as_str()
            );
        }
    }

    #[test]
    fn keeps_the_readable_part_of_a_bearer_header() {
        let (out, fired) = redact_shapes("Authorization: Bearer abcdefghijklmnopqrstuvwxyz012345");
        assert_eq!(out, format!("Authorization: Bearer {REDACTED}"));
        assert!(fired.contains(&Kind::Bearer));
    }

    #[test]
    fn keeps_the_host_of_a_uri_and_drops_the_credentials() {
        let (out, fired) = redact_shapes("postgres://appuser:h0rr1bl3pass@db.internal:5432/events");
        assert_eq!(
            out,
            format!("postgres://{REDACTED}@db.internal:5432/events")
        );
        assert!(fired.contains(&Kind::UriUserinfo));
        assert!(
            out.contains("db.internal"),
            "the host is evidence and must survive"
        );

        let (srv, _) =
            redact_shapes("mongodb+srv://svc:Sup3rSecret@cluster0.abcd.mongodb.net/prod");
        assert_eq!(
            srv,
            format!("mongodb+srv://{REDACTED}@cluster0.abcd.mongodb.net/prod")
        );

        // The value written straight after a word character, which is what the
        // console formatter does: its ANSI reset ends in `m`. The credential
        // still has to go — this case is why the rule carries no `\b`.
        let (glued, fired) = redact_shapes(
            "payload=\u{1b}[0mpostgres://appuser:h0rr1bl3pass@db.internal:5432/events",
        );
        assert!(
            !glued.contains("h0rr1bl3pass"),
            "a coloured line kept its credential: {glued}"
        );
        assert!(fired.contains(&Kind::UriUserinfo));
        assert!(glued.contains("db.internal"), "{glued}");
    }

    #[test]
    fn leaves_a_uri_without_credentials_alone() {
        let input = "https://example.internal/api/v1/status";
        assert_eq!(redact_shapes(input).0, input);
    }

    #[test]
    fn defers_the_generic_rule_to_the_entropy_gate() {
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(
            redact_shapes(hash).0,
            hash,
            "a sha256 is evidence, not a secret"
        );

        let blob = "Zm9vYmFyYmF6cXV1eEhlbGxvV29ybGREZWFkQmVlZjEyMzQ1Ng==";
        let (out, fired) = redact_shapes(blob);
        assert_eq!(out, REDACTED);
        assert!(fired.contains(&Kind::HighEntropy));
    }

    #[test]
    fn redacts_every_occurrence_in_one_line() {
        let (out, _) = redact_shapes("a=sk-abcdefghijklmnopqrstuvwx b=sk-zyxwvutsrqponmlkjihgfe");
        assert_eq!(out, format!("a={REDACTED} b={REDACTED}"));
    }
}

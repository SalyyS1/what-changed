//! Secret-shaped fixtures for the redaction leak suite: one entry per shape the
//! scrubber has to catch, replayed through every sink that can persist or print
//! a value. Every value here is synthetic and belongs to no real account — only
//! the structure is real, because structure is what the pattern rules key on.

// Not every consumer asserts on every field.
#![allow(dead_code)]

/// A synthetic credential the redaction suite replays through every sink.
pub struct Secret {
    /// Human name of the shape, used in assertion messages.
    pub what: &'static str,
    /// Config key the value is filed under when the fixture is embedded in an event.
    pub key: &'static str,
    /// The full value as it would appear in a config file or log line.
    pub value: &'static str,
    /// The substring that must never survive redaction in any artifact. For
    /// most shapes this is the whole value; for a URI it is only the
    /// credential, because the host is legitimately kept.
    pub must_vanish: &'static str,
    /// Redaction rule expected to fire, as its stable string name.
    pub kind: &'static str,
}

pub const SECRETS: &[Secret] = &[
    Secret {
        what: "OpenAI-style vendor API key",
        key: "OPENAI_API_KEY",
        value: "sk-syntheticFIXTUREvalue00notreal01",
        must_vanish: "sk-syntheticFIXTUREvalue00notreal01",
        kind: "api_key",
    },
    Secret {
        what: "HTTP Authorization bearer value",
        key: "AUTHORIZATION",
        value: "Bearer fixtureNOTaREALtoken0011",
        must_vanish: "Bearer fixtureNOTaREALtoken0011",
        kind: "bearer",
    },
    // Each segment stays under 40 characters so the JWT rule is what fires here
    // and not the generic base64 rule.
    Secret {
        what: "JWT with three base64url segments",
        key: "ID_TOKEN",
        value: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJmaXh0dXJlLXVzZXIifQ.c2lnbmF0dXJlLWZpeHR1cmU",
        must_vanish: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJmaXh0dXJlLXVzZXIifQ.c2lnbmF0dXJlLWZpeHR1cmU",
        kind: "jwt",
    },
    Secret {
        what: "AWS long-term access key id",
        key: "AWS_ACCESS_KEY_ID",
        value: "AKIAFIXTURE0NOTREAL0",
        must_vanish: "AKIAFIXTURE0NOTREAL0",
        kind: "aws_access_key",
    },
    // `ASIA` marks a short-lived STS credential, and the dotted key is how a
    // credentials-file section arrives once the config collector flattens it.
    Secret {
        what: "AWS temporary access key id",
        key: "profile.ci.aws_access_key_id",
        value: "ASIAFIXTURE0TEMP0000",
        must_vanish: "ASIAFIXTURE0TEMP0000",
        kind: "aws_access_key",
    },
    // The BEGIN marker is the whole value on purpose: the rule keys on the
    // marker, so a trailing key body would be a survivor the rule never claimed.
    Secret {
        what: "PEM private key header",
        key: "SSH_PRIVATE_KEY",
        value: "-----BEGIN RSA PRIVATE KEY-----",
        must_vanish: "-----BEGIN RSA PRIVATE KEY-----",
        kind: "private_key",
    },
    Secret {
        what: "GitHub personal access token",
        key: "GITHUB_TOKEN",
        value: concat!("gh", "p_FIXTUREnotarealtoken0000000000000000"),
        must_vanish: concat!("gh", "p_FIXTUREnotarealtoken0000000000000000"),
        kind: "host_token",
    },
    Secret {
        what: "GitHub refresh token",
        key: "GH_REFRESH_TOKEN",
        value: concat!("gh", "r_FIXTUREnotarealrefresh00000000000000"),
        must_vanish: concat!("gh", "r_FIXTUREnotarealrefresh00000000000000"),
        kind: "host_token",
    },
    Secret {
        what: "Slack bot token",
        key: "SLACK_BOT_TOKEN",
        value: concat!("xo", "xb-1111111111-2222222222-FIXTUREnotarealsecret"),
        must_vanish: concat!("xo", "xb-1111111111-2222222222-FIXTUREnotarealsecret"),
        kind: "chat_token",
    },
    // Host, port and database name are evidence worth keeping, so only the
    // password is required to vanish and it shares no substring with the host.
    Secret {
        what: "Postgres URI with inline credentials",
        key: "DATABASE_URL",
        value: "postgres://whatchanged_app:FixtureOnlyNotReal99@db.internal.example:5432/whatchanged",
        must_vanish: "FixtureOnlyNotReal99",
        kind: "uri_userinfo",
    },
    // The `+` in the scheme must not stop the userinfo rule; the cluster host
    // and the connection options survive alongside it.
    Secret {
        what: "mongodb+srv URI with inline credentials",
        key: "MONGO_URI",
        value: "mongodb+srv://ingest_svc:Synth3ticNotRealPw@cluster0.k7f2x.mongodb.net/whatchanged?retryWrites=true&w=majority",
        must_vanish: "Synth3ticNotRealPw",
        kind: "uri_userinfo",
    },
    // No key rule matches `BUILD_SIGNATURE`, so the value rule has to fire on
    // its own: 44 characters, mixed case, base64 symbols, neither hex nor digits.
    Secret {
        what: "High-entropy opaque secret under an innocuous key",
        key: "BUILD_SIGNATURE",
        value: "aG4Kp+9ZqR3vX7wLm2TsYb8NdQ1cE6uJ/fH0iOgV5rWx",
        must_vanish: "aG4Kp+9ZqR3vX7wLm2TsYb8NdQ1cE6uJ/fH0iOgV5rWx",
        kind: "high_entropy",
    },
];

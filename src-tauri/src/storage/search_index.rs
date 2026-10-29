//! Full-text search over events.
//!
//! One implementation ships: FTS5, which the vendored SQLite was measured to
//! contain and which a startup assertion guards. The trait exists so the
//! query service depends on "search" rather than on FTS5's SQL; if a
//! dependency bump ever removes the feature, the inverted-index fallback is
//! written then, behind this trait, against a test that constructs the other
//! schema — not shipped now as an unexercised second path.

use rusqlite::Connection;

use super::SqlContext;
use crate::error::AppResult;

/// Longest query accepted. Anything longer is not a search, it is a paste.
pub const MAX_QUERY_CHARS: usize = 128;
/// Search endpoints never return an unbounded candidate set.
pub const MAX_SEARCH_RESULTS: u16 = 100;

/// Search over the events a person can see.
pub trait SearchIndex {
    /// Event ids matching `text`, best first, at most `limit`.
    fn search(&self, conn: &Connection, text: &str, limit: u16) -> AppResult<Vec<i64>>;

    /// Rebuild the index from the events table.
    fn rebuild(&self, conn: &Connection) -> AppResult<()>;
}

/// The FTS5 external-content index created by the second migration.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fts5Index;

impl SearchIndex for Fts5Index {
    fn search(&self, conn: &Connection, text: &str, limit: u16) -> AppResult<Vec<i64>> {
        let Some(expression) = match_expression(text) else {
            return Ok(Vec::new());
        };
        let limit = limit.min(MAX_SEARCH_RESULTS);
        let numeric_port = text.trim().parse::<i64>().ok();
        let mut ids = Vec::with_capacity(usize::from(limit));
        for (importance_rank, importance_token) in [
            (3_i64, "critical"),
            (2_i64, "important"),
            (1_i64, "notable"),
            (0_i64, "informational"),
        ] {
            let remaining = limit.saturating_sub(u16::try_from(ids.len()).unwrap_or(limit));
            if remaining == 0 {
                break;
            }
            // The importance token is fixed by the enum, while the user's
            // text has already been quoted by `match_expression`.
            let bucket_expression = format!("{expression} AND importance:{importance_token}");
            let mut statement = conn
                .prepare_cached(
                    "SELECT search_hits.event_id FROM ( \
                        SELECT rowid AS event_id FROM events_search_fts \
                        WHERE events_search_fts MATCH ?1 \
                        UNION \
                        SELECT event_id FROM event_search_metadata \
                        WHERE ?2 IS NOT NULL AND port = ?2 \
                     ) AS search_hits \
                     JOIN event_search_order ON event_search_order.event_id = search_hits.event_id \
                     WHERE event_search_order.importance_rank = ?4 \
                     ORDER BY event_search_order.timestamp DESC, event_search_order.event_id DESC \
                     LIMIT ?3",
                )
                .during("prepare a search")?;
            let rows = statement
                .query_map(
                    (
                        bucket_expression,
                        numeric_port,
                        i64::from(remaining),
                        importance_rank,
                    ),
                    |row| row.get::<_, i64>(0),
                )
                .during("search events")?;
            ids.extend(
                rows.collect::<rusqlite::Result<Vec<i64>>>()
                    .during("search events")?,
            );
        }
        Ok(ids)
    }

    fn rebuild(&self, conn: &Connection) -> AppResult<()> {
        conn.execute_batch(
            "INSERT INTO events_fts (events_fts) VALUES ('rebuild'); \
             INSERT INTO events_search_fts (events_search_fts) VALUES ('rebuild')",
        )
        .during("rebuild the search index")
    }
}

/// Turn free text into an FTS5 expression that cannot be a syntax error.
///
/// Every whitespace-separated token becomes a quoted phrase, with inner
/// quotes doubled, and the last token also matches as a prefix so typing
/// `dock` finds Docker. A query with no tokens returns `None`.
pub fn match_expression(text: &str) -> Option<String> {
    let bounded: String = text.chars().take(MAX_QUERY_CHARS).collect();
    // A token with nothing alphanumeric in it tokenizes to nothing, and an
    // empty phrase is not a query.
    let tokens: Vec<&str> = bounded
        .split_whitespace()
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let last = tokens.len() - 1;
    let phrases: Vec<String> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            let quoted = format!("\"{}\"", token.replace('"', "\"\""));
            if index == last {
                format!("{quoted}*")
            } else {
                quoted
            }
        })
        .collect();
    Some(phrases.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_become_quoted_phrases_and_the_last_is_a_prefix() {
        assert_eq!(
            match_expression("port 3000"),
            Some("\"port\" \"3000\"*".into())
        );
        assert_eq!(match_expression("   "), None);
        assert_eq!(match_expression("( ) ..."), None);
        assert_eq!(
            match_expression("say \"hi\""),
            Some("\"say\" \"\"\"hi\"\"\"*".into())
        );
    }

    #[test]
    fn a_pasted_wall_of_text_is_cut_to_the_limit() {
        let long = "a".repeat(1000);
        let expression = match_expression(&long).unwrap_or_default();
        assert!(expression.len() <= MAX_QUERY_CHARS + 3);
    }
}

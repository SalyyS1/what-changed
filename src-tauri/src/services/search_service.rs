//! Full-text search joined back to the bounded event summary shape.

use crate::dto::SearchResultDto;
use crate::error::AppResult;
use crate::storage::search_index::{MAX_QUERY_CHARS, MAX_SEARCH_RESULTS};
use crate::storage::{checkpoint_repository, event_repository, Db, Fts5Index, SearchIndex};

/// Search visible event text, preserving FTS rank and returning no more than `limit`.
pub fn search(db: &Db, text: &str, limit: u16) -> AppResult<Vec<SearchResultDto>> {
    let bounded: String = text.chars().take(MAX_QUERY_CHARS).collect();
    let limit = limit.clamp(1, MAX_SEARCH_RESULTS);
    db.read(|conn| {
        let checkpoints = checkpoint_repository::search_labels(conn, &bounded, limit.min(20))?;
        let event_limit = limit.saturating_sub(u16::try_from(checkpoints.len()).unwrap_or(limit));
        let ids = Fts5Index.search(conn, &bounded, event_limit)?;
        let events = event_repository::summaries_by_ids(conn, &ids)?;
        let mut results: Vec<_> = events
            .into_iter()
            .map(|event| SearchResultDto::Event {
                category: event.category,
                importance: event.importance,
                event,
            })
            .collect();
        results.extend(
            checkpoints
                .into_iter()
                .map(|checkpoint| SearchResultDto::Checkpoint { checkpoint }),
        );
        Ok(results)
    })
}

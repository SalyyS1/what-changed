//! The churn episode: one summary instead of a flood.
//!
//! A program that starts and stops eight times in forty seconds is eight facts
//! worth keeping and one thing worth saying. The raw events stay append-only and
//! arrive promptly; the summary is appended beside them when the window closes,
//! never substituted for them. Substituting would mean rewriting history the
//! user has already been shown, and "collapse" cannot mean both.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::ids::ProcessId;
use crate::domain::Confidence;

/// The observation window, anchored at the episode's first completed pair.
pub const WINDOW_MS: i64 = 60_000;

/// Matched pairs in one window that earn a summary. The rule is *more than*
/// five, so six is the first count that publishes.
pub const PAIR_THRESHOLD: u32 = 5;

/// One image's open episode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Episode {
    /// The image name, for the summary's display text.
    pub display_name: String,
    /// When the first matched pair completed. Anchoring here rather than at a
    /// rolling "now" is what makes the window mean something: a process that
    /// starts and runs for an hour has not begun an episode.
    pub anchor_ms: i64,
    /// Matched pairs so far.
    pub pairs: u32,
    /// The weakest identity evidence among every pair in this episode.
    #[serde(default = "low_confidence")]
    pub confidence: Confidence,
}

/// Open episodes, keyed by the process identity that is churning.
pub type Episodes = BTreeMap<ProcessId, Episode>;

/// One closed window that earned a summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// The churning process.
    pub entity_id: ProcessId,
    /// Its image name.
    pub display_name: String,
    /// Matched pairs inside the window.
    pub pairs: u32,
    /// The weakest identity evidence supporting the summary.
    pub confidence: Confidence,
    /// When the window opened, epoch milliseconds UTC.
    pub window_start_ms: i64,
    /// When it closed, which is also the summary's timestamp.
    pub window_end_ms: i64,
}

/// Count one matched start/stop pair against its image's episode.
///
/// Only a stop whose start the collector also observed is a matched pair, so a
/// program that was already running when recording began cannot open an episode
/// on the strength of an exit nobody watched begin.
pub fn record_pair(
    episodes: &mut Episodes,
    entity_id: &ProcessId,
    display_name: &str,
    now_ms: i64,
    confidence: Confidence,
) {
    let episode = episodes
        .entry(entity_id.clone())
        .or_insert_with(|| Episode {
            display_name: display_name.to_owned(),
            anchor_ms: now_ms,
            pairs: 0,
            confidence,
        });
    episode.pairs = episode.pairs.saturating_add(1);
    episode.confidence = episode.confidence.min(confidence);
}

/// Close due episodes, returning at most `limit` summaries to publish.
///
/// An episode closes before its count is published, so a summary is a fact
/// about a finished window rather than a running total a later pair could
/// contradict. Episodes that never crossed the threshold are dropped: the
/// window is over and there is nothing to say.
/// Quiet due episodes consume no row and are discarded immediately. Loud due
/// episodes beyond the bound remain durable for a later commit; removing every
/// due episode before admission would either overflow the atomic limit or lose
/// summaries permanently.
pub fn close_due(episodes: &mut Episodes, now_ms: i64, limit: usize) -> Vec<Summary> {
    let due: Vec<ProcessId> = episodes
        .iter()
        .filter(|(_, episode)| now_ms.saturating_sub(episode.anchor_ms) >= WINDOW_MS)
        .map(|(entity_id, _)| entity_id.clone())
        .collect();
    let mut summaries = Vec::with_capacity(limit.min(due.len()));
    for entity_id in due {
        let Some(episode) = episodes.get(&entity_id) else {
            continue;
        };
        if episode.pairs <= PAIR_THRESHOLD {
            episodes.remove(&entity_id);
            continue;
        }
        if summaries.len() >= limit {
            continue;
        }
        let Some(episode) = episodes.remove(&entity_id) else {
            continue;
        };
        summaries.push(Summary {
            window_start_ms: episode.anchor_ms,
            window_end_ms: episode.anchor_ms.saturating_add(WINDOW_MS),
            entity_id,
            display_name: episode.display_name,
            pairs: episode.pairs,
            confidence: episode.confidence,
        });
    }
    summaries
}

const fn low_confidence() -> Confidence {
    Confidence::Low
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn id() -> ProcessId {
        ProcessId::from_executable("flaky.exe")
    }

    fn record(episodes: &mut Episodes, pairs: u32, now_ms: i64) {
        for offset in 0..pairs {
            record_pair(
                episodes,
                &id(),
                "flaky.exe",
                now_ms + i64::from(offset) * 1_000,
                Confidence::High,
            );
        }
    }

    #[test]
    fn the_window_is_anchored_at_the_first_pair_and_does_not_slide() {
        let mut episodes = Episodes::new();
        record_pair(&mut episodes, &id(), "flaky.exe", 10_000, Confidence::High);
        assert_eq!(episodes[&id()].anchor_ms, 10_000);
        record_pair(&mut episodes, &id(), "flaky.exe", 50_000, Confidence::High);
        assert_eq!(
            episodes[&id()].anchor_ms,
            10_000,
            "a later pair must not move the anchor"
        );
        assert_eq!(episodes[&id()].pairs, 2);
    }

    #[test]
    fn a_window_closes_on_time_and_not_before() {
        let mut episodes = Episodes::new();
        record(&mut episodes, 9, 10_000);
        assert!(close_due(&mut episodes, 10_000 + WINDOW_MS - 1, usize::MAX).is_empty());
        assert_eq!(episodes.len(), 1, "still open");

        let summaries = close_due(&mut episodes, 10_000 + WINDOW_MS, usize::MAX);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].pairs, 9);
        assert_eq!(summaries[0].window_start_ms, 10_000);
        assert_eq!(summaries[0].window_end_ms, 10_000 + WINDOW_MS);
        assert_eq!(summaries[0].display_name, "flaky.exe");
        assert!(episodes.is_empty(), "a closed episode is finished");
    }

    #[test]
    fn five_pairs_is_quiet_and_six_is_one_summary() {
        let mut quiet = Episodes::new();
        record(&mut quiet, PAIR_THRESHOLD, 0);
        assert!(close_due(&mut quiet, WINDOW_MS, usize::MAX).is_empty());
        assert!(
            quiet.is_empty(),
            "a quiet episode is dropped, not kept open"
        );

        let mut loud = Episodes::new();
        record(&mut loud, PAIR_THRESHOLD + 1, 0);
        let summaries = close_due(&mut loud, WINDOW_MS, usize::MAX);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].pairs, PAIR_THRESHOLD + 1);
    }

    #[test]
    fn each_image_gets_its_own_episode() {
        let mut episodes = Episodes::new();
        let other = ProcessId::from_executable("other.exe");
        record(&mut episodes, 9, 0);
        for _ in 0..9 {
            record_pair(&mut episodes, &other, "other.exe", 0, Confidence::High);
        }
        let summaries = close_due(&mut episodes, WINDOW_MS, usize::MAX);
        assert_eq!(summaries.len(), 2);
        assert!(summaries.iter().any(|row| row.entity_id == id()));
        assert!(summaries.iter().any(|row| row.entity_id == other));
    }

    #[test]
    fn an_episode_survives_serialization_so_a_recreation_resumes_it() {
        let mut episodes = Episodes::new();
        record(&mut episodes, 7, 10_000);
        let text = serde_json::to_string(&episodes).unwrap();
        let mut restored: Episodes = serde_json::from_str(&text).unwrap();
        assert_eq!(restored, episodes);
        let summaries = close_due(&mut restored, 10_000 + WINDOW_MS, usize::MAX);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].pairs, 7);
    }

    #[test]
    fn due_summaries_beyond_the_commit_bound_remain_for_later() {
        let mut episodes = Episodes::new();
        for index in 0..205u32 {
            let entity = ProcessId::from_executable(&format!("app-{index}.exe"));
            for _ in 0..(PAIR_THRESHOLD + 1) {
                record_pair(
                    &mut episodes,
                    &entity,
                    &format!("app-{index}.exe"),
                    0,
                    Confidence::High,
                );
            }
        }

        let first = close_due(&mut episodes, WINDOW_MS, 200);
        assert_eq!(first.len(), 200);
        assert_eq!(episodes.len(), 5, "undispatched summaries remain durable");
        let second = close_due(&mut episodes, WINDOW_MS, 200);
        assert_eq!(second.len(), 5);
        assert!(episodes.is_empty());
    }

    #[test]
    fn one_weak_pair_keeps_the_whole_summary_low_confidence() {
        let mut episodes = Episodes::new();
        for index in 0..(PAIR_THRESHOLD + 1) {
            let confidence = if index == 2 {
                Confidence::Low
            } else {
                Confidence::High
            };
            record_pair(&mut episodes, &id(), "flaky.exe", 0, confidence);
        }
        let summaries = close_due(&mut episodes, WINDOW_MS, usize::MAX);
        assert_eq!(summaries[0].confidence, Confidence::Low);
    }
}

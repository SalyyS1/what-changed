//! The settings table: string keys, JSON values, seeded by the first
//! migration so every key the product reads exists before it is read.
//!
//! The table is key/value because settings arrive over time and a schema
//! migration per preference would be absurd. The screen wants one object, so
//! `load` and `update` do that translation here — in the module allowed to
//! read the table — and a key name therefore exists in exactly one file.
//!
//! Every read falls back to the documented default rather than failing: a
//! settings row that will not decode must not stop the app from drawing its
//! Settings screen, which is the one place a user could fix it.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::{from_json, to_json, SqlContext};
use crate::domain::{
    now_ms, MonitoringProfile, RetentionPolicy, Settings, SettingsUpdate, Theme, VisualizationMode,
};
use crate::error::{AppError, AppResult};

/// Keys, exactly as seeded by the first migration.
const THEME_KEY: &str = "theme";
const VISUALIZATION_KEY: &str = "visualization";
/// Key under which the retention lifetimes live.
pub const RETENTION_KEY: &str = "retention";
const PROFILE_KEY: &str = "monitoring_profile";
const COLLECTORS_KEY: &str = "collectors";
const ONBOARDING_KEY: &str = "onboarding_completed";
const DEVELOPER_KEY: &str = "developer_mode";

/// Key of the flag that makes the hide-to-tray balloon a one-time event.
pub const FIRST_HIDE_NOTIFIED_KEY: &str = "first_hide_notified";

/// Longest a collector id may be. Ids are our own short words; anything longer
/// arriving from the frontend is not one.
const MAX_COLLECTOR_ID: usize = 40;

/// The raw JSON for a key, or `None` if the key does not exist.
pub fn get_json(conn: &Connection, key: &str) -> AppResult<Option<Value>> {
    let mut statement = conn
        .prepare_cached("SELECT value FROM settings WHERE key = ?1")
        .during("prepare a settings read")?;
    let mut rows = statement.query([key]).during("read a setting")?;
    let Some(row) = rows.next().during("read a setting")? else {
        return Ok(None);
    };
    let text: String = row.get(0).during("read a setting")?;
    from_json("value", &text).map(Some)
}

/// A typed setting, or `None` if the key does not exist.
pub fn get<T: DeserializeOwned>(conn: &Connection, key: &str) -> AppResult<Option<T>> {
    match get_json(conn, key)? {
        Some(value) => serde_json::from_value(value).map(Some).map_err(|_| {
            crate::error::DbError::Decoding {
                reason: format!("setting {key} has an unexpected shape"),
            }
            .into()
        }),
        None => Ok(None),
    }
}

/// Store a setting, replacing any previous value.
pub fn put<T: Serialize>(conn: &Connection, key: &str, value: &T) -> AppResult<()> {
    conn.prepare_cached(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .during("prepare a settings write")?
    .execute((key, to_json(value)?, now_ms()))
    .during("write a setting")?;
    Ok(())
}

/// Every setting, for the diagnostics view.
pub fn all(conn: &Connection) -> AppResult<Vec<(String, Value)>> {
    let mut statement = conn
        .prepare_cached("SELECT key, value FROM settings ORDER BY key")
        .during("prepare a settings listing")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .during("list settings")?;
    let mut out = Vec::new();
    for row in rows {
        let (key, text) = row.during("list settings")?;
        out.push((key, from_json("value", &text)?));
    }
    Ok(out)
}

/// The retention policy, falling back to the defaults if the row is missing
/// or unreadable — retention must never fail open into "keep forever".
pub fn retention(conn: &Connection) -> AppResult<RetentionPolicy> {
    Ok(get::<RetentionPolicy>(conn, RETENTION_KEY)?.unwrap_or_default())
}

/// Read a key, falling back to `default` when it is missing or unreadable.
///
/// The warning matters more than the value: a key that stopped decoding is a
/// bug in a migration or a hand-edited database, and it should be visible in
/// the log rather than silently papered over.
fn get_or<T: DeserializeOwned>(conn: &Connection, key: &str, default: T) -> T {
    match get::<T>(conn, key) {
        Ok(Some(value)) => value,
        Ok(None) => default,
        Err(error) => {
            tracing::warn!(key, %error, "a setting could not be read; using the default");
            default
        }
    }
}

/// Every setting, as one object.
pub fn load(conn: &Connection) -> AppResult<Settings> {
    Ok(Settings {
        theme: get_or(conn, THEME_KEY, Theme::System),
        visualization: get_or(conn, VISUALIZATION_KEY, VisualizationMode::Auto),
        retention: get_or(conn, RETENTION_KEY, RetentionPolicy::default()),
        monitoring_profile: get_or(conn, PROFILE_KEY, MonitoringProfile::Balanced),
        collectors: get_or(conn, COLLECTORS_KEY, BTreeMap::new()),
        onboarding_completed: get_or(conn, ONBOARDING_KEY, false),
        developer_mode: get_or(conn, DEVELOPER_KEY, false),
    })
}

/// Apply a partial update and return the settings as they now stand.
///
/// Fields the caller omitted are not rewritten, so two screens updating two
/// different preferences cannot overwrite each other with their stale copies.
pub fn update(conn: &Connection, update: &SettingsUpdate) -> AppResult<Settings> {
    // A partial update is one user action. If a later field is invalid, none of
    // the earlier fields may remain changed while the caller sees an error.
    let transaction = conn
        .unchecked_transaction()
        .during("begin a settings update")?;
    let settings = update_fields(&transaction, update)?;
    transaction.commit().during("commit a settings update")?;
    Ok(settings)
}

/// Apply one update on a connection that is already inside the transaction.
fn update_fields(conn: &Connection, update: &SettingsUpdate) -> AppResult<Settings> {
    if let Some(theme) = update.theme {
        put(conn, THEME_KEY, &theme)?;
    }
    if let Some(visualization) = update.visualization {
        put(conn, VISUALIZATION_KEY, &visualization)?;
    }
    if let Some(retention) = update.retention {
        if !retention.is_supported() {
            return Err(AppError::InvalidInput {
                what: "retention",
                reason: "lifetimes are between 1 and 3650 days".to_owned(),
            });
        }
        put(conn, RETENTION_KEY, &retention)?;
    }
    if let Some(profile) = update.monitoring_profile {
        put(conn, PROFILE_KEY, &profile)?;
    }
    if let Some(changes) = &update.collectors {
        put(conn, COLLECTORS_KEY, &merged_collectors(conn, changes)?)?;
    }
    if let Some(done) = update.onboarding_completed {
        put(conn, ONBOARDING_KEY, &done)?;
    }
    if let Some(developer) = update.developer_mode {
        put(conn, DEVELOPER_KEY, &developer)?;
    }
    load(conn)
}

/// The stored collector map with `changes` applied.
///
/// The seeded map is the list of collectors that exist. Accepting an unknown
/// id would let the frontend grow the settings object one call at a time.
fn merged_collectors(
    conn: &Connection,
    changes: &BTreeMap<String, bool>,
) -> AppResult<BTreeMap<String, bool>> {
    let mut current: BTreeMap<String, bool> = get_or(conn, COLLECTORS_KEY, BTreeMap::new());
    for (id, enabled) in changes {
        if !current.contains_key(id) {
            return Err(AppError::InvalidInput {
                what: "collector",
                reason: format!("there is no collector called '{}'", short_id(id)),
            });
        }
        current.insert(id.clone(), *enabled);
    }
    Ok(current)
}

/// A collector id is one of our own words, so echoing it back in an error
/// leaks nothing — but only after it has been bounded, since the value came
/// from the frontend and an unbounded one would land in the log.
fn short_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(MAX_COLLECTOR_ID)
        .collect()
}

/// Whether the one-time hide balloon has been shown.
///
/// A missing or unreadable flag means "not shown yet", and the worst
/// consequence of getting that wrong is one balloon.
pub fn first_hide_notified(conn: &Connection) -> AppResult<bool> {
    Ok(get_or(conn, FIRST_HIDE_NOTIFIED_KEY, false))
}

/// Record that the one-time hide balloon has been shown.
pub fn mark_first_hide_notified(conn: &Connection) -> AppResult<()> {
    put(conn, FIRST_HIDE_NOTIFIED_KEY, &true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::storage::Db;
    use crate::testing::temp_dir;

    fn db(label: &str) -> Db {
        Db::open_in(&temp_dir(label)).unwrap()
    }

    #[test]
    fn the_seeded_defaults_are_what_the_screen_first_renders() {
        let db = db("settings-defaults");
        let settings = db.read(load).unwrap();
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.visualization, VisualizationMode::Auto);
        assert_eq!(settings.retention, RetentionPolicy::default());
        assert_eq!(settings.monitoring_profile, MonitoringProfile::Balanced);
        assert!(!settings.onboarding_completed);
        assert!(!settings.developer_mode);
        assert_eq!(
            settings.collectors.len(),
            12,
            "the seeded map is the collector list"
        );
        assert_eq!(settings.collectors.get("docker"), Some(&false));
        assert_eq!(settings.collectors.get("process"), Some(&true));
    }

    #[test]
    fn an_update_touches_only_what_it_names_and_survives_a_reopen() {
        let dir = temp_dir("settings-partial");
        {
            let db = Db::open_in(&dir).unwrap();
            let before = db.read(load).unwrap();
            let changed = db
                .write(|conn| {
                    update(
                        conn,
                        &SettingsUpdate {
                            theme: Some(Theme::Light),
                            ..Default::default()
                        },
                    )
                })
                .unwrap();
            assert_eq!(changed.theme, Theme::Light);
            assert_eq!(
                changed.retention, before.retention,
                "an omitted field keeps its stored value"
            );
            assert_eq!(changed.collectors, before.collectors);
            db.flush().unwrap();
            db.close();
        }
        let db = Db::open_in(&dir).unwrap();
        assert_eq!(db.read(load).unwrap().theme, Theme::Light);
    }

    #[test]
    fn visualization_preference_round_trips_without_a_schema_key_seed() {
        let db = db("settings-visualization");
        let changed = db
            .write(|conn| {
                update(
                    conn,
                    &SettingsUpdate {
                        visualization: Some(VisualizationMode::TwoD),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(changed.visualization, VisualizationMode::TwoD);
        assert_eq!(
            db.read(load).unwrap().visualization,
            VisualizationMode::TwoD
        );
    }

    #[test]
    fn a_multi_field_update_rolls_back_when_a_later_field_is_invalid() {
        let db = db("settings-atomic");
        let before = db.read(load).unwrap();
        let invalid = RetentionPolicy {
            events_days: 0,
            snapshots_days: 7,
            important_days: 90,
        };

        let result = db.write(move |conn| {
            update(
                conn,
                &SettingsUpdate {
                    theme: Some(Theme::Light),
                    retention: Some(invalid),
                    ..Default::default()
                },
            )
        });
        assert!(result.is_err());

        let after = db.read(load).unwrap();
        assert_eq!(
            after, before,
            "an update that returned an error changed state"
        );
    }

    #[test]
    fn a_collector_flag_round_trips_and_an_unknown_id_is_refused() {
        let db = db("settings-collectors");
        let mut changes = BTreeMap::new();
        changes.insert("docker".to_owned(), true);
        let after = db
            .write(move |conn| {
                update(
                    conn,
                    &SettingsUpdate {
                        collectors: Some(changes),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(after.collectors.get("docker"), Some(&true));
        assert_eq!(
            after.collectors.len(),
            12,
            "an update must not grow the map"
        );

        let mut bogus = BTreeMap::new();
        bogus.insert("please_run_this".to_owned(), true);
        let error = db
            .write(move |conn| {
                update(
                    conn,
                    &SettingsUpdate {
                        collectors: Some(bogus),
                        ..Default::default()
                    },
                )
            })
            .unwrap_err();
        assert!(error.to_string().contains("no collector called"));
    }

    #[test]
    fn retention_is_bounded_so_zero_days_can_never_mean_delete_everything() {
        let db = db("settings-retention");
        let zero = RetentionPolicy {
            events_days: 0,
            snapshots_days: 7,
            important_days: 90,
        };
        assert!(db
            .write(move |conn| update(
                conn,
                &SettingsUpdate {
                    retention: Some(zero),
                    ..Default::default()
                }
            ))
            .is_err());

        let sane = RetentionPolicy {
            events_days: 7,
            snapshots_days: 3,
            important_days: 365,
        };
        let after = db
            .write(move |conn| {
                update(
                    conn,
                    &SettingsUpdate {
                        retention: Some(sane),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(after.retention, sane);
    }

    #[test]
    fn the_first_hide_flag_flips_once_and_stays_flipped() {
        let db = db("settings-first-hide");
        assert!(!db.read(first_hide_notified).unwrap());
        db.write(mark_first_hide_notified).unwrap();
        assert!(db.read(first_hide_notified).unwrap(), "the flag stays set");
    }

    #[test]
    fn a_garbled_row_falls_back_to_the_default_rather_than_failing_the_screen() {
        let db = db("settings-garbled");
        db.write(|conn| {
            conn.execute(
                "UPDATE settings SET value = ?2 WHERE key = ?1",
                (THEME_KEY, "\"not-a-theme\""),
            )
            .during("garble a setting")
        })
        .unwrap();
        assert_eq!(db.read(load).unwrap().theme, Theme::System);
    }
}

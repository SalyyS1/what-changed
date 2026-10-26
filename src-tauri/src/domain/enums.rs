//! The closed vocabularies of the event model.
//!
//! Each enum is stored as its `snake_case` text, so a row is readable in any
//! SQLite browser and a query can filter on the word a person would use.
//! That text is the storage contract: renaming a variant is a migration, and
//! the round-trip tests below pin the variant counts so a variant cannot be
//! added or removed without the tests noticing.

use std::fmt;

/// An enum value that was not one of the known words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVariant {
    /// The vocabulary the value was expected to belong to.
    pub kind: &'static str,
    /// The rejected text. Vocabularies are closed sets of public words, so
    /// echoing an unknown one leaks nothing.
    pub value: String,
}

impl fmt::Display for UnknownVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is not a {}", self.value, self.kind)
    }
}

impl std::error::Error for UnknownVariant {}

/// Declares a text-backed enum: the variants, their stored words, `as_str`,
/// `FromStr`, `Display` and an `ALL` slice for exhaustive tests. Variants are
/// ordered as declared, so a vocabulary that reads as a scale (importance)
/// compares the way it reads.
macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        $name:ident, $kind:literal {
            $( $(#[$variant_meta:meta])* $variant:ident => $text:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::ts_rs::TS,
        )]
        pub enum $name {
            $( $(#[$variant_meta])* #[serde(rename = $text)] $variant, )+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant, )+ ];

            /// The word stored in the database for this variant.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $text, )+
                }
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::domain::enums::UnknownVariant;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $text => Ok($name::$variant), )+
                    other => Err($crate::domain::enums::UnknownVariant {
                        kind: $kind,
                        value: other.to_owned(),
                    }),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

pub(crate) use text_enum;

text_enum! {
    /// What part of the machine an event is about. Fifteen, exactly as the
    /// brief lists them.
    Category, "category" {
        Application => "application",
        Process => "process",
        Filesystem => "filesystem",
        Configuration => "configuration",
        Network => "network",
        Service => "service",
        Driver => "driver",
        Update => "update",
        Docker => "docker",
        Git => "git",
        Security => "security",
        Runtime => "runtime",
        Startup => "startup",
        Hardware => "hardware",
        System => "system",
    }
}

text_enum! {
    /// What happened to the entity. Twenty-one: the nineteen the brief lists, plus
    /// `churn` — the one summary a collector writes when a thing started and
    /// stopped so often that the individual facts are noise. The individual
    /// facts are still recorded; the summary is appended beside them rather
    /// than replacing them.
    EventType, "event type" {
        Created => "created",
        Deleted => "deleted",
        Modified => "modified",
        Started => "started",
        Stopped => "stopped",
        Restarted => "restarted",
        Installed => "installed",
        Uninstalled => "uninstalled",
        Updated => "updated",
        Enabled => "enabled",
        Disabled => "disabled",
        Connected => "connected",
        Disconnected => "disconnected",
        Opened => "opened",
        Closed => "closed",
        Failed => "failed",
        Recovered => "recovered",
        ConfigurationChanged => "configuration_changed",
        VersionChanged => "version_changed",
        Churn => "churn",
        Renamed => "renamed",
    }
}

text_enum! {
    /// The kind of thing an entity id names. One variant per prefix of the
    /// identifier grammar in `ids`, plus the machine itself for boot and
    /// shutdown events, which are about nothing smaller.
    EntityType, "entity type" {
        Process => "process",
        Port => "port",
        Service => "service",
        Application => "app",
        Driver => "driver",
        File => "file",
        Adapter => "adapter",
        Container => "container",
        Image => "image",
        DockerVolume => "docker_volume",
        DockerNetwork => "docker_network",
        Project => "project",
        Update => "update",
        Task => "task",
        System => "system",
    }
}

text_enum! {
    /// How bad the thing itself was, independent of whether a person should
    /// care. A service failing is an error even when it is routine.
    Severity, "severity" {
        Info => "info",
        Warning => "warning",
        Error => "error",
    }
}

text_enum! {
    /// Whether a person should care. Most events are informational; the
    /// product is calm by classification, not by hiding rows.
    Importance, "importance" {
        Informational => "informational",
        Notable => "notable",
        Important => "important",
        Critical => "critical",
    }
}

text_enum! {
    /// How sure the collector is about the event's facts — for example whether
    /// it could read a process's executable path or only its name.
    Confidence, "confidence" {
        Low => "low",
        Medium => "medium",
        High => "high",
    }
}

text_enum! {
    /// How a process stopped, as far as the recorder can tell.
    ///
    /// `Unexpected` is a claim that something crashed, and a diff-poll cannot
    /// make it: the process collector only ever observes that a pid is gone.
    /// The word is reserved for an exit the event-log collector corroborates
    /// with an Application Error record for the same image a few seconds
    /// either side. Everything else stays `Unknown`, which is the honest
    /// answer rather than a euphemism for normal.
    ExitKind, "exit kind" {
        Normal => "normal",
        Unexpected => "unexpected",
        Unknown => "unknown",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::str::FromStr;

    use serde::Serialize;

    use super::*;

    fn round_trips<T>(all: &[T])
    where
        T: Copy
            + PartialEq
            + fmt::Debug
            + fmt::Display
            + FromStr
            + Serialize
            + serde::de::DeserializeOwned,
        <T as FromStr>::Err: fmt::Debug,
    {
        for variant in all {
            let text = variant.to_string();
            assert_eq!(
                text,
                text.to_lowercase(),
                "stored words are lowercase snake_case"
            );
            assert_eq!(&T::from_str(&text).unwrap(), variant);
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(json, format!("\"{text}\""), "serde and as_str must agree");
            assert_eq!(&serde_json::from_str::<T>(&json).unwrap(), variant);
        }
    }

    #[test]
    fn the_vocabularies_are_the_size_the_brief_fixes() {
        assert_eq!(Category::ALL.len(), 15);
        // The brief's nineteen, plus the one churn summary a noisy process
        // earns instead of a flood of individual rows.
        assert_eq!(EventType::ALL.len(), 21);
        assert_eq!(Severity::ALL.len(), 3);
        assert_eq!(Importance::ALL.len(), 4);
        assert_eq!(Confidence::ALL.len(), 3);
        assert_eq!(EntityType::ALL.len(), 15);
        assert_eq!(ExitKind::ALL.len(), 3);
    }

    #[test]
    fn every_variant_round_trips_through_text_and_json() {
        round_trips(Category::ALL);
        round_trips(EventType::ALL);
        round_trips(EntityType::ALL);
        round_trips(Severity::ALL);
        round_trips(Importance::ALL);
        round_trips(Confidence::ALL);
        round_trips(ExitKind::ALL);
    }

    #[test]
    fn an_unknown_word_is_an_error_that_names_the_vocabulary() {
        let error = Category::from_str("weather").unwrap_err();
        assert_eq!(error.kind, "category");
        assert_eq!(error.to_string(), "'weather' is not a category");
    }

    #[test]
    fn importance_orders_the_way_it_reads() {
        assert!(Importance::Informational < Importance::Notable);
        assert!(Importance::Notable < Importance::Important);
        assert!(Importance::Important < Importance::Critical);
    }
}

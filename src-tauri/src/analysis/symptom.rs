use std::collections::BTreeSet;

use crate::domain::{Category, Entity, EntityType, SymptomModel, SymptomTag};

pub struct LexiconRow {
    pub tag: SymptomTag,
    pub words: &'static [&'static str],
    pub categories: &'static [Category],
    pub entity_types: &'static [EntityType],
}

pub const LEXICON: &[LexiconRow] = &[
    LexiconRow {
        tag: SymptomTag::Audio,
        words: &["microphone", "audio", "sound", "mic"],
        categories: &[Category::Driver, Category::Application, Category::Service],
        entity_types: &[
            EntityType::Driver,
            EntityType::Application,
            EntityType::Service,
        ],
    },
    LexiconRow {
        tag: SymptomTag::Network,
        words: &[
            "internet", "network", "wifi", "dns", "vpn", "proxy", "offline",
        ],
        categories: &[Category::Network, Category::Driver, Category::Service],
        entity_types: &[EntityType::Adapter, EntityType::Driver, EntityType::Service],
    },
    LexiconRow {
        tag: SymptomTag::Port,
        words: &[
            "port",
            "localhost",
            "connect",
            "refused",
            "3000",
            "8080",
            "api",
        ],
        categories: &[
            Category::Network,
            Category::Process,
            Category::Configuration,
        ],
        entity_types: &[
            EntityType::Port,
            EntityType::Process,
            EntityType::File,
            EntityType::Service,
        ],
    },
    LexiconRow {
        tag: SymptomTag::Performance,
        words: &["slow", "freeze", "lag"],
        categories: &[
            Category::Driver,
            Category::Update,
            Category::Startup,
            Category::Service,
        ],
        entity_types: &[EntityType::Driver, EntityType::Task, EntityType::Service],
    },
    LexiconRow {
        tag: SymptomTag::Crash,
        words: &["crash", "closes", "quit", "error"],
        categories: &[Category::Application, Category::Update, Category::Driver],
        entity_types: &[
            EntityType::Application,
            EntityType::Update,
            EntityType::Driver,
        ],
    },
    LexiconRow {
        tag: SymptomTag::Build,
        words: &["build", "compile", "test"],
        categories: &[
            Category::Configuration,
            Category::Application,
            Category::Update,
        ],
        entity_types: &[
            EntityType::File,
            EntityType::Application,
            EntityType::Update,
        ],
    },
    LexiconRow {
        tag: SymptomTag::Docker,
        words: &["docker", "container"],
        categories: &[Category::Docker, Category::Service, Category::Application],
        entity_types: &[
            EntityType::Container,
            EntityType::Service,
            EntityType::Application,
        ],
    },
    LexiconRow {
        tag: SymptomTag::Login,
        words: &["login", "sign in", "password"],
        categories: &[
            Category::Configuration,
            Category::Network,
            Category::Application,
        ],
        entity_types: &[
            EntityType::File,
            EntityType::Adapter,
            EntityType::Application,
        ],
    },
];

pub fn extract(problem: &str, entities: &[Entity], symptom_time_ms: i64) -> SymptomModel {
    let normalized = normalize(problem);
    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    let mut categories = BTreeSet::new();
    let mut entity_types = BTreeSet::new();
    let mut tags = BTreeSet::new();
    let mut matched_words = BTreeSet::new();
    for row in LEXICON {
        let row_matches = row
            .words
            .iter()
            .filter(|word| contains_phrase(&normalized, &tokens, word))
            .copied()
            .collect::<Vec<_>>();
        if !row_matches.is_empty() {
            tags.insert(row.tag);
            categories.extend(row.categories.iter().copied());
            entity_types.extend(row.entity_types.iter().copied());
            matched_words.extend(row_matches);
        }
    }

    let mut entity_ids = entities
        .iter()
        .filter(|entity| {
            let name = normalize(&entity.display_name);
            let id_tail = normalize(entity.id.split(':').next_back().unwrap_or(&entity.id));
            [name, id_tail].iter().any(|candidate| {
                candidate.len() >= 4
                    && (normalized.contains(candidate.as_str())
                        || tokens.iter().any(|token| {
                            (token.len() >= 4 || matched_words.contains(token))
                                && candidate.contains(token)
                        }))
            })
        })
        .map(|entity| entity.id.clone())
        .collect::<Vec<_>>();
    entity_ids.sort();
    entity_ids.dedup();

    SymptomModel {
        entity_ids,
        categories: categories.into_iter().collect(),
        entity_types: entity_types.into_iter().collect(),
        tags: tags.into_iter().collect(),
        symptom_time_ms,
    }
}

fn contains_phrase(normalized: &str, tokens: &[&str], word: &str) -> bool {
    if word.contains(' ') {
        normalized.contains(word)
    } else {
        tokens.contains(&word)
    }
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

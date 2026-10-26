use what_changed_lib::analysis::symptom::{extract, LEXICON};
use what_changed_lib::domain::{Category, SymptomTag};

#[test]
fn every_declared_lexicon_row_matches_one_representative_word() {
    for row in LEXICON {
        let model = extract(row.words[0], &[], 42);
        assert!(model.tags.contains(&row.tag), "missing {:?}", row.tag);
        for category in row.categories {
            assert!(model.categories.contains(category));
        }
    }
}

#[test]
fn unmatched_words_do_not_guess_a_category() {
    let model = extract("the frobnicator is purple", &[], 42);
    assert!(model.categories.is_empty());
    assert!(model.entity_ids.is_empty());
}

#[test]
fn port_and_audio_use_typed_tags() {
    assert!(extract("localhost refused port 3000", &[], 0)
        .tags
        .contains(&SymptomTag::Port));
    let audio = extract("microphone sound", &[], 0);
    assert!(audio.tags.contains(&SymptomTag::Audio));
    assert!(audio.categories.contains(&Category::Driver));
}

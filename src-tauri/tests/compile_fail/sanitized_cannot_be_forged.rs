//! Not compiled, on purpose. Cargo builds only the `.rs` files at the top level
//! of `tests/`, so this file is a record rather than a test: it lists the ways of
//! obtaining a `Sanitized<T>` without going through the sanitizer, and every one
//! of them is rejected by the compiler today.
//!
//! It is written down rather than proven with `trybuild` because that would add a
//! proc-macro dependency and a build-time cost to assert something the type
//! system already refuses. The guarantee lives in `privacy::sanitize`: a private
//! field, no `new`, no `From`, no `Deref`. This file is where a future maintainer
//! can check that assumption by hand — paste a block into a real integration
//! test and confirm it still fails with the error named above it.
//!
//! If any of these ever compiles, the privacy chokepoint is gone. The repository
//! accepts `Sanitized<T>` precisely because the wrapper cannot be forged.

use serde_json::json;
use what_changed_lib::privacy::{Sanitized, Sanitizer};

fn the_tuple_constructor_is_private() {
    // error[E0603]: tuple struct constructor `Sanitized` is private
    let _forged = Sanitized(String::from("DB_PASSWORD=hunter2"));
}

fn there_is_no_associated_constructor() {
    // error[E0599]: no function or associated item named `new` found for struct
    // `Sanitized` in the current scope
    let _forged = Sanitized::new(String::from("DB_PASSWORD=hunter2"));
}

fn there_is_no_conversion_into_it() {
    // error[E0277]: the trait bound `Sanitized<String>: From<String>` is not
    // satisfied
    let _forged: Sanitized<String> = String::from("DB_PASSWORD=hunter2").into();
}

fn there_is_no_default_to_fill_in() {
    // error[E0277]: the trait bound `Sanitized<String>: Default` is not satisfied
    let _forged = Sanitized::<String>::default();
}

fn it_cannot_be_deserialized_past_the_rules() {
    // error[E0277]: the trait bound `Sanitized<String>: Deserialize<'_>` is not
    // satisfied — a wrapper that could be parsed from JSON would let any file on
    // disk claim to have been scrubbed.
    let _forged: Sanitized<String> = serde_json::from_str(r#""DB_PASSWORD=hunter2""#).unwrap();
}

fn the_field_cannot_be_reached_from_outside() {
    let (sanitized, _) = Sanitizer::new().sanitize(json!({ "DB_PASSWORD": "hunter2" }));
    // error[E0616]: field `0` of struct `Sanitized` is private
    let _inner = sanitized.0;
}

fn the_value_cannot_be_taken_out_of_the_crate() {
    let (sanitized, _) = Sanitizer::new().sanitize(json!({ "DB_PASSWORD": "hunter2" }));
    // error[E0624]: method `into_inner` is private — it is `pub(crate)`, so the
    // only code that can unwrap a sanitized value is the crate that scrubbed it.
    let _inner = sanitized.into_inner();
}

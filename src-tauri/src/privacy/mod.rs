//! Redaction. Everything that can persist or print a value goes through here.
//!
//! Two entry points, one implementation of the rules:
//!
//! - `Sanitizer` + `Sanitized<T>` for structured values on their way to
//!   storage, where the type system carries the proof that scrubbing happened;
//! - `scrub_line` for text on its way to a log file or the console, where there
//!   is no type to hang a proof on and the writer itself does the work.
//!
//! The rules live in `keys` (the name), `patterns` (the shape) and `entropy`
//! (the gate on the generic rule). Nothing derived from a secret is kept: no
//! digest, no prefix, no length. Config diffs compare only sanitized values:
//! retained `[REDACTED]` values are deliberately indistinguishable, while a
//! sensitive key appearing or disappearing remains a structural fact.

pub mod entropy;
pub mod keys;
pub mod patterns;
pub mod sanitize;
pub mod scrub;

pub use patterns::{Kind, REDACTED};
pub use sanitize::{Redaction, Sanitized, Sanitizer, Scrub};
pub use scrub::{scrub_line, scrub_value};

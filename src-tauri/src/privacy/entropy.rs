//! Entropy gate for the generic opaque-string rule.
//!
//! The vendor-prefixed patterns catch credentials that announce themselves. A
//! long base64-looking blob announces nothing, so it is judged on information
//! density instead. That judgement is deliberately narrow: this product exists
//! to show hashes, versions, ports and container ids, so a rule that eats them
//! is worse than one that misses an unusual secret. Hex-only and digit-only
//! strings are therefore excluded before entropy is even measured.

/// Minimum bits per character before an opaque string is treated as a secret.
/// Random base64 sits near 5.5; a sha256 in hex near 4.0; ordinary identifiers
/// well below.
const MIN_BITS_PER_CHAR: f64 = 3.6;

/// Shortest string the generic rule will consider. Also the run length the
/// shape pre-filter looks for, so the two cannot drift apart.
pub(super) const MIN_LEN: usize = 40;

/// Shannon entropy in bits per character, over byte frequencies.
///
/// Bytes rather than scalar values: candidates come from an ASCII-only pattern,
/// and for anything else the byte view still measures density without needing a
/// grapheme opinion.
pub fn bits_per_char(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut counts = [0u32; 256];
    for &b in s.as_bytes() {
        counts[b as usize] += 1;
    }
    let len = s.len() as f64;
    -counts
        .iter()
        .filter(|&&n| n > 0)
        .map(|&n| {
            let p = f64::from(n) / len;
            p * p.log2()
        })
        .sum::<f64>()
}

fn is_all_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_all_digits(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit())
}

fn has_mixed_case(s: &str) -> bool {
    s.chars().any(char::is_uppercase) && s.chars().any(char::is_lowercase)
}

fn has_base64_symbol(s: &str) -> bool {
    s.contains('+') || s.contains('/') || s.contains('=')
}

/// Is this opaque string dense enough, and unlike enough to an identifier, to
/// be treated as a credential?
///
/// All four gates must hold: long enough, not a hash or a number, visibly
/// encoded rather than a word, and dense.
pub fn looks_high_entropy(s: &str) -> bool {
    s.len() >= MIN_LEN
        && !is_all_hex(s)
        && !is_all_digits(s)
        && (has_mixed_case(s) || has_base64_symbol(s))
        && bits_per_char(s) >= MIN_BITS_PER_CHAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_the_reference_cases() {
        assert_eq!(bits_per_char(""), 0.0);
        assert_eq!(bits_per_char("aaaa"), 0.0);
        // Two equally frequent symbols carry exactly one bit each.
        assert!((bits_per_char("abab") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn fires_on_an_opaque_encoded_blob() {
        assert!(looks_high_entropy(
            "Zm9vYmFyYmF6cXV1eEhlbGxvV29ybGREZWFkQmVlZjEyMzQ1Ng=="
        ));
        assert!(looks_high_entropy(
            "aB3xY7pQ9rS2tU5vW8yZ1cD4eF6gH0jK2lM4nP6q"
        ));
    }

    #[test]
    fn spares_the_values_the_product_exists_to_show() {
        // A sha256 and a git sha are pure hex, which the gate excludes even
        // though their density is high.
        assert!(!looks_high_entropy(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ));
        assert!(!looks_high_entropy(
            "9d3c7f1b2a4e6d8c0b5a7e9f1d3c5b7a9e1f3d5c"
        ));
        // A long integer, a version, an identifier without case variety.
        assert!(!looks_high_entropy(
            "18446744073709551615000000000000000000000000"
        ));
        assert!(!looks_high_entropy(
            "abcdefghijklmnopqrstuvwxyz0123456789abcdefgh"
        ));
        // Too short to judge.
        assert!(!looks_high_entropy("aB3xY7pQ9rS2tU5v"));
    }

    #[test]
    fn spares_a_repetitive_string_that_only_looks_encoded() {
        assert!(!looks_high_entropy(
            "AaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAa"
        ));
    }
}

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::domain::driver_naming::{brand, class_noun, friendly_name};

#[test]
fn section_73_nvidia_example_is_exact() {
    assert_eq!(
        format!(
            "{} updated 566.36 → 566.45",
            friendly_name("NVIDIA Corporation", "Display")
        ),
        "NVIDIA graphics driver updated 566.36 → 566.45"
    );
}

#[test]
fn class_table_and_fallback_are_stable() {
    assert_eq!(class_noun("Net"), "network driver");
    assert_eq!(class_noun("AudioEndpoint"), "audio driver");
    assert_eq!(class_noun("media"), "audio driver");
    assert_eq!(class_noun("USB"), "USB driver");
    assert_eq!(class_noun("Biometric"), "Biometric driver");
    assert_eq!(class_noun(""), "device driver");
    assert_eq!(brand("Intel Corporation"), Some("Intel".to_owned()));
    assert_eq!(brand("   "), None);
}

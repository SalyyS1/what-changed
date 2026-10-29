#![cfg(test)]

use super::*;

#[test]
fn move_guard_blocks_readers_and_follows_the_same_file_through_rename() {
    let root = crate::testing::temp_dir("generation-native-move");
    let path = root.join("source.db");
    let target = root.join("moved.db");
    std::fs::write(&path, b"fixture").unwrap();
    let guard = open(&path, false, Sharing::Move).unwrap();
    assert!(File::open(&path).is_err());
    rename_held(&guard, &path, &target, false).unwrap();
    assert!(File::open(&target).is_err());
    drop(guard);
    assert_eq!(std::fs::read(target).unwrap(), b"fixture");
}

#[test]
fn held_file_and_directory_move_between_pinned_parents() {
    let root = crate::testing::temp_dir("generation-native-cross-parent");
    let source_parent = root.join("source-parent");
    let target_parent = root.join("target-parent");
    std::fs::create_dir(&source_parent).unwrap();
    std::fs::create_dir(&target_parent).unwrap();
    let source = source_parent.join("record-\u{0111}.txt");
    let target = target_parent.join("record-\u{0111}.txt");
    std::fs::write(&source, b"exact held bytes").unwrap();
    let guard = open(&source, false, Sharing::Move).unwrap();
    rename_held(&guard, &source, &target, false).unwrap();
    validate_location(&guard, &target).unwrap();
    drop(guard);
    assert_eq!(std::fs::read(&target).unwrap(), b"exact held bytes");

    let source = source_parent.join("objects");
    let target = target_parent.join("objects");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("child"), b"same directory").unwrap();
    let guard = open(&source, true, Sharing::Move).unwrap();
    rename_held(&guard, &source, &target, false).unwrap();
    validate_location(&guard, &target).unwrap();
    drop(guard);
    assert_eq!(
        std::fs::read(target.join("child")).unwrap(),
        b"same directory"
    );
}

#[test]
fn absolute_rename_names_are_terminated_at_every_buffer_alignment() {
    let root = crate::testing::temp_dir("generation-native-terminated-name");
    for width in 1..=8 {
        let source = root.join("source");
        let target = root.join("x".repeat(width));
        std::fs::write(&source, b"exact destination").unwrap();
        let guard = open(&source, false, Sharing::Move).unwrap();
        rename_held(&guard, &source, &target, false).unwrap();
        validate_location(&guard, &target).unwrap();
        drop(guard);
        assert_eq!(std::fs::read(&target).unwrap(), b"exact destination");
        std::fs::remove_file(target).unwrap();
    }
}

#[test]
fn substituted_source_name_cannot_promote_different_bytes() {
    let root = crate::testing::temp_dir("generation-native-substitution");
    let source = root.join("source");
    let moved = root.join("displaced");
    let destination = root.join("destination");
    std::fs::write(&source, b"validated").unwrap();
    let guard = open(&source, false, Sharing::Move).unwrap();
    rename_held(&guard, &source, &moved, false).unwrap();
    std::fs::write(&source, b"unvalidated substitution").unwrap();
    assert!(rename_held(&guard, &source, &destination, false).is_err());
    assert!(!destination.exists());
    drop(guard);
    assert_eq!(std::fs::read(moved).unwrap(), b"validated");
    assert_eq!(std::fs::read(source).unwrap(), b"unvalidated substitution");
}

#[test]
fn unexpected_destination_is_preserved() {
    let root = crate::testing::temp_dir("generation-native-destination");
    let source = root.join("source");
    let destination = root.join("destination");
    std::fs::write(&source, b"validated").unwrap();
    std::fs::write(&destination, b"already present").unwrap();
    let guard = open(&source, false, Sharing::Move).unwrap();
    assert!(rename_held(&guard, &source, &destination, false).is_err());
    drop(guard);
    assert_eq!(std::fs::read(destination).unwrap(), b"already present");
}

#[test]
fn pinned_staging_parent_refuses_replacement() {
    let root = crate::testing::temp_dir("generation-native-parent");
    let stage = root.join("stage");
    let redirected = root.join("replaced");
    std::fs::create_dir(&stage).unwrap();
    let pin = open(&stage, true, Sharing::Pin).unwrap();
    assert!(std::fs::rename(&stage, &redirected).is_err());
    drop(pin);
    std::fs::rename(&stage, &redirected).unwrap();
}

#[test]
fn hardlink_is_rejected_without_changing_either_name() {
    let root = crate::testing::temp_dir("generation-native-hardlink");
    let source = root.join("original");
    let alias = root.join("alias");
    std::fs::write(&source, b"preserved").unwrap();
    std::fs::hard_link(&source, &alias).unwrap();
    assert!(open(&alias, false, Sharing::Move).is_err());
    assert_eq!(std::fs::read(source).unwrap(), b"preserved");
}

#[test]
fn a_junction_is_rejected_through_its_opened_handle() {
    use windows::Win32::System::IO::DeviceIoControl;
    let root = crate::testing::temp_dir("generation-native-junction");
    let outside = crate::testing::temp_dir("generation-native-junction-target");
    std::fs::write(outside.join("preserved"), b"untouched").unwrap();
    let junction = root.join("redirect");
    std::fs::create_dir(&junction).unwrap();
    let target = outside.to_string_lossy();
    let target = target.strip_prefix(r"\\?\").unwrap_or(&target);
    let substitute = format!(r"\??\{target}").encode_utf16().collect::<Vec<_>>();
    let print = target.encode_utf16().collect::<Vec<_>>();
    let mut data = Vec::new();
    data.extend(0xa0000003_u32.to_le_bytes());
    let payload_len = 8 + 2 * (substitute.len() + print.len() + 2);
    data.extend((payload_len as u16).to_le_bytes());
    data.extend(0_u16.to_le_bytes());
    data.extend(0_u16.to_le_bytes());
    data.extend(((substitute.len() * 2) as u16).to_le_bytes());
    data.extend((((substitute.len() + 1) * 2) as u16).to_le_bytes());
    data.extend(((print.len() * 2) as u16).to_le_bytes());
    for unit in substitute
        .into_iter()
        .chain(Some(0))
        .chain(print)
        .chain(Some(0))
    {
        data.extend(unit.to_le_bytes());
    }
    let handle = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS).0)
        .open(&junction)
        .unwrap();
    let mut returned = 0_u32;
    // SAFETY: the temporary directory handle is owned, and the mount-point
    // reparse buffer and returned-byte pointer remain valid during the call.
    unsafe {
        DeviceIoControl(
            HANDLE(handle.as_raw_handle()),
            0x000900a4,
            Some(data.as_ptr().cast()),
            data.len() as u32,
            None,
            0,
            Some(&mut returned),
            None,
        )
    }
    .unwrap();
    drop(handle);
    assert!(open(&junction, true, Sharing::Pin).is_err());
    assert_eq!(
        std::fs::read(outside.join("preserved")).unwrap(),
        b"untouched"
    );
    std::fs::remove_dir(junction).unwrap();
}

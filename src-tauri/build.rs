fn main() -> std::io::Result<()> {
    // Cargo tests run this build script without Tauri's beforeBuildCommand.
    // Keep the configured resource resolvable, but never treat this placeholder
    // as a release notice: the release generator and verifier reject it.
    let notice = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dist-installer/THIRD_PARTY_NOTICES.txt");
    let parent = notice.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "notice parent is missing")
    })?;
    std::fs::create_dir_all(parent)?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&notice)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(b"PENDING - NOT FOR RELEASE\nBuild-only resource placeholder. Release packaging requires generated, reviewed third-party notices.\n")
                ?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    // Windows' installer-detection heuristic treats test executables whose
    // names contain words such as "update" as elevation candidates. Every
    // collector test is deliberately non-elevated, so embed an explicit
    // asInvoker manifest in MSVC test binaries instead of relying on the file
    // name heuristic.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-tests=/MANIFESTUAC:level='asInvoker' uiAccess='false'");
    }
    tauri_build::build();
    Ok(())
}

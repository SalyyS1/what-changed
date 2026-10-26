//! Writes `src/lib/tauri/bindings.ts`, or checks it.
//!
//! ```text
//! cargo run --example export_bindings --features devtools            # write
//! cargo run --example export_bindings --features devtools -- --check # compare
//! ```
//!
//! `--check` is what `bun run check:bindings` runs. It exits non-zero and
//! prints the first differing line rather than a whole diff: the fix is always
//! "regenerate and commit", so the useful output is the one line that proves
//! drift, not a wall of it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::ExitCode;

/// Where the generated file lives, relative to the crate manifest.
const TARGET: &str = "../src/lib/tauri/bindings.ts";

fn main() -> ExitCode {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TARGET);
    let generated: String = what_changed_lib::dto::bindings::generate_full();
    let checking = std::env::args().any(|arg| arg == "--check");

    if !checking {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("the bindings directory can be created");
        }
        std::fs::write(&target, &generated).expect("the bindings file can be written");
        println!("wrote {}", target.display());
        return ExitCode::SUCCESS;
    }

    let Ok(committed) = std::fs::read_to_string(&target) else {
        eprintln!(
            "bindings check failed: {} does not exist. Run: bun run gen:bindings",
            target.display()
        );
        return ExitCode::FAILURE;
    };

    // Line endings are not the contract. A checkout with `core.autocrlf` on
    // would otherwise fail this gate on every line of a file nobody touched.
    let committed = normalize(&committed);
    let generated = normalize(&generated);
    if committed == generated {
        println!("bindings are up to date");
        return ExitCode::SUCCESS;
    }

    let first = committed
        .lines()
        .zip(generated.lines())
        .enumerate()
        .find(|(_, (a, b))| a != b);
    match first {
        Some((line, (committed, generated))) => {
            eprintln!("bindings drifted at line {}:", line + 1);
            eprintln!("  committed: {committed}");
            eprintln!("  generated: {generated}");
        }
        None => eprintln!("bindings drifted: the files are different lengths"),
    }
    eprintln!("Run: bun run gen:bindings, then commit the result.");
    ExitCode::FAILURE
}

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
}

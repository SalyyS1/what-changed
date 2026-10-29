//! Devtools backend-only diagnostic; excludes Tauri/WebView acceptance.

mod health;
mod paths;
mod runner;

use std::path::PathBuf;
const DEFAULT_SECONDS: u64 = 14_400;

struct Options {
    data_dir: PathBuf,
    trigger_file: PathBuf,
    ack_file: PathBuf,
    seconds: u64,
}

fn usage() -> &'static str {
    "usage: WhatChanged.exe --soak --data-dir <what-changed-perf-soak-...> \
     --trigger-file <path> --ack-file <path> [--seconds N]"
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut data_dir = None;
    let mut trigger_file = None;
    let mut ack_file = None;
    let mut seconds = DEFAULT_SECONDS;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--soak" => {}
            "--data-dir" => data_dir = Some(args.next().ok_or_else(|| usage().to_owned())?),
            "--trigger-file" => trigger_file = Some(args.next().ok_or_else(|| usage().to_owned())?),
            "--ack-file" => ack_file = Some(args.next().ok_or_else(|| usage().to_owned())?),
            "--seconds" => {
                seconds = args
                    .next()
                    .ok_or_else(|| usage().to_owned())?
                    .parse()
                    .map_err(|_| "--seconds must be an unsigned integer".to_owned())?;
                if seconds == 0 || seconds > 86_400 {
                    return Err("--seconds must be between 1 and 86400".to_owned());
                }
            }
            "--help" | "-h" => return Err(usage().to_owned()),
            unknown => return Err(format!("unknown argument {unknown}\n{}", usage())),
        }
    }
    Ok(Options {
        data_dir: PathBuf::from(data_dir.ok_or_else(|| "--data-dir is required".to_owned())?),
        trigger_file: PathBuf::from(
            trigger_file.ok_or_else(|| "--trigger-file is required".to_owned())?,
        ),
        ack_file: PathBuf::from(ack_file.ok_or_else(|| "--ack-file is required".to_owned())?),
        seconds,
    })
}

pub fn run(args: impl IntoIterator<Item = String>) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return 2;
        }
    };
    match runner::run(options) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("backend soak failed: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests;

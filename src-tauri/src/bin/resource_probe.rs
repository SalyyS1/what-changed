//! Bounded resource sampler for the Phase 20 worksheet.
//!
//! Win32 acquisition lives in `platform::windows::resource_probe`; keeping
//! this binary as a formatter makes the platform boundary auditable and keeps
//! the CSV free of hostnames, paths, command lines and usernames.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use what_changed_lib::platform::windows::resource_probe::{sample, ResourceSample};

#[cfg(not(windows))]
mod portable_probe {
    use std::time::Instant;

    #[derive(Debug, Clone, Copy, Default)]
    pub struct ResourceSample {
        pub process_count: usize,
        pub total_working_set_bytes: u64,
        pub private_working_set_bytes: Option<u64>,
        pub cpu_percent: Option<f64>,
        pub handle_count: Option<u32>,
        pub cpu_time_100ns: u64,
    }

    pub fn sample(_previous: Option<(Instant, u64)>) -> ResourceSample {
        ResourceSample::default()
    }
}

#[cfg(not(windows))]
use portable_probe::{sample, ResourceSample};

#[derive(Debug, Clone, Copy)]
struct Options {
    seconds: u64,
    interval_ms: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            seconds: 600,
            interval_ms: 60_000,
        }
    }
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (name, value) = match arg.as_str() {
            "--seconds" => ("seconds", args.next()),
            "--interval-ms" => ("interval-ms", args.next()),
            "--help" | "-h" => {
                return Err("usage: resource_probe [--seconds N] [--interval-ms N]".to_owned())
            }
            unknown => return Err(format!("unknown argument {unknown}")),
        };
        let value = value.ok_or_else(|| format!("missing value for --{name}"))?;
        let parsed = value
            .parse::<u64>()
            .map_err(|_| format!("--{name} must be an unsigned integer"))?;
        if parsed == 0 {
            return Err(format!("--{name} must be greater than zero"));
        }
        match name {
            "seconds" => options.seconds = parsed,
            "interval-ms" => options.interval_ms = parsed,
            _ => unreachable!("closed parser names"),
        }
    }
    Ok(options)
}

fn epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn main() {
    let options = match parse_options(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    println!("timestamp_ms,total_working_set_bytes,private_working_set_bytes,process_count,cpu_percent,handle_count");
    let started = Instant::now();
    // The first sample establishes a baseline.  Taking a CPU delta against
    // only the current process would otherwise include all descendant CPU time
    // accumulated before the probe started and make the first row misleading.
    let mut previous: Option<(Instant, u64)> = None;
    let deadline = started + Duration::from_secs(options.seconds);
    loop {
        let sample: ResourceSample = sample(previous);
        let cpu = sample
            .cpu_percent
            .map(|value| format!("{value:.3}"))
            .unwrap_or_default();
        let private = sample
            .private_working_set_bytes
            .map(|value| value.to_string())
            .unwrap_or_default();
        let handles = sample
            .handle_count
            .map(|value| value.to_string())
            .unwrap_or_default();
        println!(
            "{},{},{},{},{},{}",
            epoch_ms(),
            sample.total_working_set_bytes,
            private,
            sample.process_count,
            cpu,
            handles
        );
        if Instant::now() >= deadline {
            break;
        }
        let wait = Duration::from_millis(options.interval_ms).min(
            deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_default(),
        );
        std::thread::sleep(wait);
        previous = Some((Instant::now(), sample.cpu_time_100ns));
    }
}

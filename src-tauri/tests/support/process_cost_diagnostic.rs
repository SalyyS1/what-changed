//! Paired live-process timings; observations only, never an acceptance gate.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::collectors::process::{self, ProcessCollector};
use what_changed_lib::collectors::process_map::ProcessMap;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{Collector, CollectorSnapshot, SnapshotContext};
use what_changed_lib::error::AppResult;
use what_changed_lib::platform::traits::{ProcessProvider, ProcessRow};
use what_changed_lib::platform::windows::process::WindowsProcessProvider;
use what_changed_lib::storage::CollectorCommitReceipt;

use super::{lock_perf, CYCLES};

#[derive(Default)]
struct ProviderTiming {
    elapsed_ns: AtomicU64,
    row_count: AtomicUsize,
}

struct TimedProcessProvider<P> {
    inner: P,
    timing: Arc<ProviderTiming>,
}

impl<P: ProcessProvider> ProcessProvider for TimedProcessProvider<P> {
    fn enumerate(&self) -> AppResult<Vec<ProcessRow>> {
        self.inner.enumerate()
    }

    fn enumerate_into(&self, output: &mut Vec<ProcessRow>) -> AppResult<()> {
        let began = Instant::now();
        let result = self.inner.enumerate_into(output);
        let elapsed = began.elapsed();
        // The collector reuses its Vec; only numeric evidence crosses this seam.
        // Reads and writes occur on the same test thread, so Relaxed is sufficient.
        self.timing.elapsed_ns.store(
            u64::try_from(elapsed.as_nanos()).unwrap(),
            Ordering::Relaxed,
        );
        self.timing.row_count.store(output.len(), Ordering::Relaxed);
        result
    }
}

struct Sample {
    cycle: usize,
    // Total, provider, snapshot remainder, acknowledgement region, destruction.
    stages: [Duration; 5],
    rows: usize,
    kind: u8,
    observations: usize,
}

fn report(mut samples: Vec<Sample>) {
    eprintln!("process paired-stage diagnostic: observation only, not acceptance; cycles={CYCLES}");
    eprintln!("kind codes: 0=Noop, 1=Atomic, 2=Legacy; all timings are nanoseconds");
    eprintln!(
        "overhead retained: provider adds two clock reads and two atomic stores; \
         cycle adds two intermediate clock reads and result classification; \
         numeric resets, reads and sample appends occur outside the full timer"
    );
    eprintln!(
        "snapshot remainder includes instrumentation and all non-provider snapshot work; \
         private collector stages and durable SQLite commits are not individually measured"
    );
    eprintln!(
        "acknowledgement_region includes result classification and the conditional branch; \
         Noop and Legacy measure only this overhead, with no acknowledgement hook"
    );
    let mut kind_counts = [0; 3];
    for sample in &samples {
        kind_counts[usize::from(sample.kind)] += 1;
    }
    eprintln!(
        "kind_counts noop={} atomic={} legacy={}",
        kind_counts[0], kind_counts[1], kind_counts[2]
    );
    for (stage, label) in [
        "total",
        "provider",
        "snapshot_remainder",
        "acknowledgement_region",
        "destruction",
    ]
    .into_iter()
    .enumerate()
    {
        let mut values: Vec<_> = samples.iter().map(|sample| sample.stages[stage]).collect();
        values.sort_unstable();
        let p50 = values[(values.len() * 50).div_ceil(100) - 1];
        let p95 = values[(values.len() * 95).div_ceil(100) - 1];
        eprintln!(
            "stage={label} p50_ns={} p95_ns={}",
            p50.as_nanos(),
            p95.as_nanos()
        );
    }
    samples.sort_unstable_by_key(|sample| std::cmp::Reverse(sample.stages[0]));
    for sample in samples.iter().take(5) {
        let [total, provider, remainder, acknowledgement, destruction] = sample.stages;
        eprintln!(
            "slow_cycle={} total_ns={} provider_ns={} snapshot_remainder_ns={} \
             acknowledgement_region_ns={} destruction_ns={} rows={} kind={} observations={}",
            sample.cycle,
            total.as_nanos(),
            provider.as_nanos(),
            remainder.as_nanos(),
            acknowledgement.as_nanos(),
            destruction.as_nanos(),
            sample.rows,
            sample.kind,
            sample.observations,
        );
    }
}

#[test]
#[ignore = "live paired-stage diagnostic; invoke explicitly, not an acceptance result"]
fn real_process_collector_paired_stage_costs() {
    let _serial = lock_perf();
    let live = live_context(process::ID)
        .unwrap_or_else(|_| panic!("process diagnostic context setup failed"));
    let timing = Arc::new(ProviderTiming::default());
    let provider = TimedProcessProvider {
        inner: WindowsProcessProvider,
        timing: Arc::clone(&timing),
    };
    let mut collector = ProcessCollector::new(
        provider,
        ProcessMap::new(),
        process::catalog_descriptor()
            .unwrap_or_else(|_| panic!("process diagnostic descriptor unavailable")),
    );
    collector
        .start(live.context.clone())
        .unwrap_or_else(|_| panic!("process diagnostic collector start failed"));
    let receipt = CollectorCommitReceipt { event_ids: vec![] };
    let initial_context = SnapshotContext::for_devtools(1);
    let initial = collector
        .snapshot_batch(initial_context)
        .unwrap_or_else(|_| panic!("process diagnostic initial snapshot failed"));
    assert!(matches!(initial, CollectorSnapshot::Atomic { .. }));
    collector
        .commit_succeeded(initial_context.token(), &receipt)
        .unwrap_or_else(|_| panic!("process diagnostic initial acknowledgement failed"));

    let mut samples = Vec::with_capacity(CYCLES);
    for index in 0..CYCLES {
        timing.elapsed_ns.store(0, Ordering::Relaxed);
        timing.row_count.store(0, Ordering::Relaxed);
        let context = SnapshotContext::for_devtools(u64::try_from(index).unwrap() + 2);
        let began = Instant::now();
        let snapshot = collector
            .snapshot_batch(context)
            .unwrap_or_else(|_| panic!("process diagnostic snapshot failed"));
        let snapshot_done = Instant::now();
        let (kind, observations) = match &snapshot {
            CollectorSnapshot::Noop => (0, 0),
            CollectorSnapshot::Atomic { observations, .. } => (1, observations.len()),
            CollectorSnapshot::Legacy(observations) => (2, observations.len()),
        };
        if kind == 1 {
            collector
                .commit_succeeded(context.token(), &receipt)
                .unwrap_or_else(|_| panic!("process diagnostic acknowledgement failed"));
        }
        let acknowledged = Instant::now();
        std::hint::black_box(&snapshot);
        drop(snapshot);
        let finished = Instant::now();

        let provider = Duration::from_nanos(timing.elapsed_ns.load(Ordering::Relaxed));
        samples.push(Sample {
            cycle: index + 1,
            stages: [
                finished.duration_since(began),
                provider,
                snapshot_done
                    .duration_since(began)
                    .checked_sub(provider)
                    .unwrap(),
                acknowledged.duration_since(snapshot_done),
                finished.duration_since(acknowledged),
            ],
            rows: timing.row_count.load(Ordering::Relaxed),
            kind,
            observations,
        });
    }
    collector.stop();
    report(samples);
}

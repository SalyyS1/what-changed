//! Isolated synthetic builder comparison; never an application acceptance gate.

use super::*;
use std::time::{Duration, Instant};

const SAMPLES: usize = 11;

fn measure(
    builder: fn(&[SystemEvent], &[Relationship]) -> StructuralGraph,
    events: &[SystemEvent],
    relationships: &[Relationship],
) -> (StructuralGraph, Duration) {
    let started = Instant::now();
    let graph = std::hint::black_box(builder(
        std::hint::black_box(events),
        std::hint::black_box(relationships),
    ));
    let elapsed = started.elapsed();
    (graph, elapsed)
}

#[test]
#[ignore = "Isolated synthetic diagnostic; explicitly compare builders after equality tests"]
fn isolated_synthetic_old_and_new_builder_durations() {
    for duplicates in [true, false] {
        let (events, relationships) = fixture(5000, 150000, duplicates);
        let original = original_build(&events, &relationships);
        let optimized = StructuralGraph::build(&events, &relationships);
        assert_eq!(optimized, original);
        drop(original);
        drop(optimized);
        let mut original_samples = [Duration::ZERO; SAMPLES];
        let mut optimized_samples = [Duration::ZERO; SAMPLES];
        for index in 0..SAMPLES {
            let ((original, original_elapsed), (optimized, optimized_elapsed)) = if index % 2 == 0 {
                (
                    measure(original_build, &events, &relationships),
                    measure(StructuralGraph::build, &events, &relationships),
                )
            } else {
                let optimized = measure(StructuralGraph::build, &events, &relationships);
                let original = measure(original_build, &events, &relationships);
                (original, optimized)
            };
            // Full output comparison and graph destruction are outside timing.
            assert_eq!(optimized, original);
            original_samples[index] = original_elapsed;
            optimized_samples[index] = optimized_elapsed;
        }
        let shape = if duplicates {
            "duplicate_heavy"
        } else {
            "mostly_unique"
        };
        let original_total: Duration = original_samples.iter().sum();
        let optimized_total: Duration = optimized_samples.iter().sum();
        eprintln!(
            "DIAGNOSTIC isolated_synthetic_graph_builder shape={shape} events={} relationships={} samples={SAMPLES} original_total_ms={:.3} optimized_total_ms={:.3} original_mean_ms={:.3} optimized_mean_ms={:.3}",
            events.len(), relationships.len(), original_total.as_secs_f64() * 1000.0,
            optimized_total.as_secs_f64() * 1000.0,
            original_total.as_secs_f64() * 1000.0 / SAMPLES as f64,
            optimized_total.as_secs_f64() * 1000.0 / SAMPLES as f64,
        );
    }
}

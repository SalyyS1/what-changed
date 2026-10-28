# Seed batch boundary negative proof output — 2026-10-05

Source and protocol: [bounded experiment](qa-261005-seed-batch-size-experiment.md).
Exit 0 proves rejection, not successful equivalence or performance acceptance.
No semantic timestamp was normalized. Entire selected-test stdout retained:

```text
    Finished `release` profile [optimized] target(s) in 0.63s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 1 test
test seed_batch_boundary_proof::canonical_cross_boundary_endpoint_rejects_larger_batch_parity ... seeded 10000/10000 synthetic events
seeded 10000/10000 synthetic events
DIAGNOSTIC seed_batch_boundary_proof rows=10000 events_mismatched=0 entity_rows=2578 entities_mismatched=1 relationship_rows=1500 relationships_mismatched=0 ownership_equal=1 witness_edge_index=986 witness_first_subject_index=3186 witness_first_seen_delta_ms=1710891089 status=rejected_semantic_parity
DIAGNOSTIC seed_batch_boundary_entity_field field=first_seen mismatches=1
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.59s
```

The actual witness edge precedes its first subject across the standard batch
boundary. Complete persisted events/relationship evidence and ownership match;
one entity `first_seen` differs. Candidate 5,000 is rejected, default remains
1,000; no checkpoint/investigation parity or performance experiment follows.

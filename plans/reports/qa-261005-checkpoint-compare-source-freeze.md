# Complete checkpoint comparison capture — formatted source freeze

Status: source/spec preparation only; no tests, ignored workload or acceptance claim yet.

Twelve owned files, post-rustfmt SHA-256. Reconstruction of the prior seven preimages
was verified separately; live source was not restored. Ten protected service/diff/PAGE,
Cargo, migration, shared-fixture and perf-harness hashes match their pre-edit pins.
Existing window implementation/evidence/tests are outside this change.

| File (repository relative) | Lines | SHA-256 |
| --- | ---: | --- |
| `src-tauri/src/storage/checkpoint_repository.rs` | 597 | `D466DDB02E48EF92A97364440A4D8F3D41C75D66B6DEFB67E3FAE303CF7D69E6` |
| `src-tauri/src/storage/mod.rs` | 109 | `8CDEF2495E7D9C6696707BA32BFC5E8C7C63AD66323786447F69A0E3F220A9A9` |
| `src-tauri/src/storage/checkpoint_compare_plan_capture.rs` | 108 | `3E632DE866D2E38E59DA5774A04A1AEE660E2D6D305A62955557D185B53CD27E` |
| `src-tauri/src/storage/checkpoint_compare_plan_evidence.rs` | 94 | `3D7C30B50BDCA7C4F1C96FD63A0C19660B0A8CBD1D85079A80E38191F87A5B46` |
| `src-tauri/src/storage/checkpoint_compare_plan_execution.rs` | 113 | `67CAD81C217B09A5C9B399C37712427BA5002D26CF42437B599E4679E08A92B2` |
| `src-tauri/src/storage/checkpoint_compare_plan_explain.rs` | 122 | `7C3C0A648B3F2E727145066CA5A7A06F427191AFBE5D7574162C0E9C89563AB8` |
| `src-tauri/src/storage/checkpoint_compare_plan_explain_tests.rs` | 111 | `E93DA46425E4F668E3F06D09F1238781ADD4492C38FBC372B7CCD1D5C6146061` |
| `src-tauri/src/storage/checkpoint_compare_plan_capture_tests.rs` | 177 | `7BD5AA8456BD58C54C69BF2700B636048A2CF172525C69900920CF60A282371E` |
| `src-tauri/src/storage/checkpoint_compare_plan_owner_tests.rs` | 254 | `B4A16B75F40F73300DF082A3E8DEB68F5CD85A6888D770251B2F7926C1985CBF` |
| `src-tauri/src/storage/checkpoint_compare_plan_service_tests.rs` | 260 | `6C4187D4CACA40C9C051A80BA66499BFA126F778DB30918F4E91085765D87A9E` |
| `src-tauri/src/storage/checkpoint_compare_plan_test_fixture.rs` | 176 | `194FBD263FB8AD0496E110E6A2D19B90AB24A92EA2C375CBC6CD730DC9CBA2D0` |
| `src-tauri/tests/checkpoint_window_plan_profile.rs` | 135 | `21449861633FF4F0B25A6BC8067CCCBACA5B76E8A230241846212FBA19180BAB` |

Spec pass: capture original owner SQL/connection/bind array; preserve cached-statement
reset before metadata domain decode and items full JSON consumption. Five ordered reads
and 2/2/1 apply only to complete successful Some comparison. Return original operation
value/error independently of bounded typed evidence. Reuse unchanged window capture;
new projection retains indexed filtering plus temporary ordering, rejects unknown
vocabulary without storing SQL, bind values or private schema names. Thread-local
non-Send guards clear after nested rejection/error/unwind. Normal builds compile hooks
out. Active devtools capture adds diagnostic EQP reads; normal-build query behavior
is preserved. Small core lifecycle/token/projection modules stay separate; cohesive owner and
service test modules exceed 200 lines but share behavior fixtures rather than arbitrary
splits. No service, schema, index, dependency, timer, p95 or network change.

Sixteen new ordinary cases (lifecycle5/projection3/direct-owner4/service3/cleanup1);
narrow prefix includes ten unchanged window regression cases. Terminal output, not
source counts, will determine results. Live simplify gate exceeds default thresholds;
scoped simplify and independent quality review precede ordinary gates. Global fmt and
owned diff-whitespace check pass. No ignored test authorized by this source receipt.

See [protocol ledger](qa-261005-checkpoint-compare-capture-ledger.md).

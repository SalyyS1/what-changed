# Paired investigation diagnostic preflight — 2026-10-04

Outcome: 10k exact 30-call diagnostic PASS, 1 test passed / zero failed /
4 filtered, 5.54 s. All 30 measured intervals contain exactly one completed
engine record and one completed service record, in ordinal order. One warm
interval has both records and is excluded. No sample discarded.

## Scope and evidence

Only `tests/investigation_stage_profile.rs` changed, adding a separately
ignored 30-call helper. Production SQL, scoring, service timers, corpus recipe
and profiler-free acceptance are unchanged. Reviewer found no actionable
defect. Advisory supervision approved marker pairing and one predeclared 1M
diagnostic after this preflight. Cleanup target passed 3 tests / 2 ignored;
release rebuild 1m20s. Coverage not measured.

Markers are emitted outside the outer clock; existing stage stderr output
remains inside it. The synchronous engine and service emit before returning.
Observed paired durations below are diagnostic, not acceptance. Input is
nested in service total; engine is nested in service analysis. Do not add
nested totals or independent stage percentiles.

Command, workdir `src-tauri`, release target cache, one Cargo job, owned
`.perf-temp`, rows=10000, stage opt-in=1:

```powershell
cargo test --release --locked --features devtools --test investigation_stage_profile thirty_persisted_investigations_report_paired_stage_timings -- --ignored --exact --nocapture --test-threads=1
```

Canonical 10k events / 3k entities / 1,500 related / 800 bursts /
40 checkpoints / 9,800 snapshots / 20 investigations / 90 days.
Warm first sorted recurrence COUNT used its covering index, 5,000 completed
executions. All 31 calls scored and persisted. E2E total 2,855.241 ms, mean
95.175, min/max 85.919/147.632. Seed wall 2,445.027 ms; inner total
2,385.441, insertion 1,467.702 ms.

## Service records — milliseconds

Service and input columns inclusive; remaining columns exclusive.
All measured calls retained.

| Call | E2E | Service | Input | Setup | Window | Boundary | Entities | Relations | Recurrence | Finalize | Analysis | Sanitize | Persist |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 92.230 | 92.119 | 53.154 | 0.012 | 33.979 | 0.780 | 4.192 | 2.730 | 11.046 | 0.426 | 37.879 | 0.556 | 0.517 |
| 2 | 85.919 | 85.793 | 52.659 | 0.010 | 33.351 | 0.827 | 4.415 | 2.796 | 10.970 | 0.299 | 32.091 | 0.580 | 0.454 |
| 3 | 86.912 | 86.715 | 54.090 | 0.009 | 33.787 | 0.811 | 4.001 | 2.685 | 12.648 | 0.157 | 31.487 | 0.628 | 0.501 |
| 4 | 86.159 | 86.041 | 53.088 | 0.008 | 33.939 | 0.828 | 4.337 | 2.760 | 11.067 | 0.157 | 31.561 | 0.796 | 0.588 |
| 5 | 87.519 | 87.398 | 53.905 | 0.008 | 34.160 | 0.879 | 4.203 | 3.034 | 11.478 | 0.151 | 32.242 | 0.733 | 0.510 |
| 6 | 86.551 | 86.410 | 54.052 | 0.008 | 34.398 | 0.826 | 4.465 | 2.845 | 11.367 | 0.150 | 31.228 | 0.634 | 0.488 |
| 7 | 88.934 | 88.803 | 55.193 | 0.007 | 34.951 | 0.850 | 4.310 | 3.080 | 11.852 | 0.150 | 32.343 | 0.748 | 0.512 |
| 8 | 87.959 | 87.831 | 54.500 | 0.007 | 35.027 | 0.901 | 4.293 | 2.870 | 11.262 | 0.147 | 32.274 | 0.486 | 0.563 |
| 9 | 88.371 | 88.251 | 54.539 | 0.007 | 34.946 | 0.817 | 4.137 | 2.775 | 11.540 | 0.324 | 32.219 | 0.875 | 0.611 |
| 10 | 90.828 | 90.693 | 55.920 | 0.008 | 35.527 | 0.989 | 4.330 | 2.979 | 11.938 | 0.157 | 33.568 | 0.560 | 0.638 |
| 11 | 91.322 | 91.180 | 57.220 | 0.029 | 37.520 | 0.857 | 4.233 | 2.906 | 11.551 | 0.153 | 32.683 | 0.750 | 0.498 |
| 12 | 96.279 | 96.077 | 56.774 | 0.007 | 36.287 | 0.971 | 4.213 | 3.139 | 12.010 | 0.155 | 38.320 | 0.495 | 0.480 |
| 13 | 89.885 | 89.762 | 56.707 | 0.010 | 36.630 | 0.809 | 4.327 | 2.934 | 11.854 | 0.154 | 31.874 | 0.531 | 0.640 |
| 14 | 89.854 | 89.721 | 56.737 | 0.008 | 36.122 | 0.837 | 4.703 | 3.051 | 11.859 | 0.166 | 31.869 | 0.675 | 0.432 |
| 15 | 86.296 | 86.116 | 54.130 | 0.007 | 34.148 | 0.825 | 4.201 | 3.226 | 11.571 | 0.159 | 30.890 | 0.558 | 0.531 |
| 16 | 89.386 | 89.276 | 54.439 | 0.008 | 34.734 | 0.844 | 4.134 | 2.805 | 11.769 | 0.153 | 33.430 | 0.958 | 0.441 |
| 17 | 89.220 | 89.092 | 55.423 | 0.008 | 34.719 | 0.841 | 4.321 | 2.869 | 12.521 | 0.153 | 32.468 | 0.686 | 0.506 |
| 18 | 90.613 | 90.462 | 56.475 | 0.008 | 36.448 | 0.873 | 4.416 | 3.099 | 11.492 | 0.147 | 32.786 | 0.431 | 0.762 |
| 19 | 86.863 | 86.686 | 53.974 | 0.008 | 33.734 | 0.830 | 4.635 | 2.929 | 11.682 | 0.164 | 31.506 | 0.641 | 0.557 |
| 20 | 89.414 | 89.263 | 55.918 | 0.008 | 35.898 | 0.853 | 4.599 | 2.880 | 11.533 | 0.154 | 32.193 | 0.708 | 0.436 |
| 21 | 86.350 | 86.041 | 53.425 | 0.007 | 33.663 | 0.871 | 4.403 | 3.081 | 11.253 | 0.153 | 31.563 | 0.417 | 0.629 |
| 22 | 91.956 | 91.831 | 57.340 | 0.008 | 36.107 | 0.867 | 4.407 | 2.999 | 12.792 | 0.169 | 33.188 | 0.670 | 0.625 |
| 23 | 89.770 | 89.655 | 56.465 | 0.009 | 35.771 | 0.805 | 4.433 | 3.099 | 12.185 | 0.172 | 31.723 | 0.731 | 0.728 |
| 24 | 94.092 | 93.977 | 61.140 | 0.009 | 36.993 | 1.091 | 5.330 | 3.341 | 14.117 | 0.268 | 31.607 | 0.453 | 0.767 |
| 25 | 130.175 | 129.869 | 53.213 | 0.009 | 32.974 | 0.926 | 4.400 | 2.972 | 11.782 | 0.159 | 74.437 | 1.169 | 1.041 |
| 26 | 147.632 | 147.407 | 99.338 | 0.014 | 61.756 | 1.653 | 9.244 | 7.165 | 19.264 | 0.257 | 46.352 | 0.905 | 0.798 |
| 27 | 122.868 | 122.721 | 79.639 | 0.011 | 50.282 | 1.264 | 5.894 | 4.497 | 17.486 | 0.217 | 41.269 | 1.172 | 0.630 |
| 28 | 111.036 | 110.870 | 69.542 | 0.009 | 44.280 | 1.023 | 5.517 | 3.679 | 14.847 | 0.195 | 39.993 | 0.712 | 0.614 |
| 29 | 101.562 | 101.438 | 63.986 | 0.008 | 40.631 | 1.042 | 5.396 | 3.403 | 13.324 | 0.191 | 36.142 | 0.715 | 0.587 |
| 30 | 99.283 | 99.167 | 63.306 | 0.008 | 40.864 | 0.958 | 4.938 | 3.298 | 13.076 | 0.172 | 34.834 | 0.469 | 0.551 |

## Engine records — milliseconds

Engine inclusive; remaining columns exclusive, nested within service Analysis.

| Call | Engine | Preprocess | Graph | Chain | Failure | Preparation | Score | Selection |
|---|---|---|---|---|---|---|---|---|
| 1 | 37.285 | 2.520 | 13.278 | 1.421 | 1.016 | 1.892 | 6.075 | 11.083 |
| 2 | 30.968 | 2.134 | 7.619 | 1.258 | 0.960 | 1.927 | 6.287 | 10.784 |
| 3 | 30.942 | 2.069 | 7.355 | 1.349 | 1.003 | 1.892 | 5.754 | 11.520 |
| 4 | 30.880 | 2.514 | 7.434 | 1.338 | 0.965 | 1.961 | 6.135 | 10.533 |
| 5 | 31.617 | 2.113 | 7.540 | 1.374 | 1.117 | 2.023 | 6.454 | 10.997 |
| 6 | 30.668 | 2.293 | 7.180 | 1.330 | 0.945 | 2.038 | 5.980 | 10.901 |
| 7 | 31.809 | 2.167 | 8.289 | 1.458 | 1.000 | 1.991 | 6.174 | 10.730 |
| 8 | 31.248 | 2.109 | 6.746 | 1.400 | 1.003 | 1.965 | 5.981 | 12.043 |
| 9 | 31.263 | 2.136 | 7.073 | 1.350 | 1.019 | 1.979 | 6.297 | 11.410 |
| 10 | 32.820 | 2.154 | 7.956 | 1.544 | 1.069 | 2.223 | 6.820 | 11.055 |
| 11 | 32.134 | 2.224 | 8.037 | 1.321 | 1.134 | 2.012 | 6.143 | 11.262 |
| 12 | 37.855 | 2.152 | 9.987 | 1.337 | 2.094 | 3.800 | 7.013 | 11.473 |
| 13 | 31.355 | 2.158 | 7.390 | 1.320 | 1.073 | 2.072 | 6.191 | 11.151 |
| 14 | 31.194 | 2.246 | 7.196 | 1.442 | 0.964 | 1.933 | 6.194 | 11.220 |
| 15 | 30.411 | 2.311 | 6.758 | 1.222 | 0.957 | 1.973 | 6.256 | 10.933 |
| 16 | 32.797 | 2.140 | 6.952 | 1.532 | 1.004 | 2.122 | 6.593 | 12.454 |
| 17 | 31.871 | 2.914 | 7.552 | 1.280 | 0.967 | 2.008 | 6.178 | 10.972 |
| 18 | 31.701 | 2.100 | 7.916 | 1.260 | 1.031 | 1.982 | 6.213 | 11.199 |
| 19 | 30.919 | 2.087 | 7.245 | 1.396 | 1.209 | 1.959 | 6.096 | 10.927 |
| 20 | 31.609 | 2.265 | 7.558 | 1.371 | 1.014 | 2.217 | 6.316 | 10.868 |
| 21 | 30.410 | 2.099 | 7.252 | 1.252 | 0.961 | 1.894 | 6.172 | 10.780 |
| 22 | 32.567 | 2.147 | 7.593 | 1.375 | 1.081 | 2.000 | 6.486 | 11.886 |
| 23 | 30.988 | 2.254 | 6.791 | 1.267 | 1.128 | 2.018 | 6.046 | 11.485 |
| 24 | 31.138 | 2.204 | 6.953 | 1.273 | 1.012 | 2.023 | 6.013 | 11.659 |
| 25 | 73.434 | 2.167 | 9.199 | 24.336 | 1.800 | 3.616 | 11.311 | 21.006 |
| 26 | 45.691 | 3.338 | 10.269 | 2.028 | 1.505 | 2.916 | 9.577 | 16.058 |
| 27 | 40.540 | 2.864 | 9.482 | 1.760 | 1.382 | 2.563 | 8.330 | 14.158 |
| 28 | 39.374 | 2.625 | 8.777 | 1.807 | 1.760 | 2.951 | 8.012 | 13.442 |
| 29 | 35.520 | 2.541 | 8.317 | 1.482 | 1.188 | 2.350 | 6.834 | 12.808 |
| 30 | 33.692 | 2.428 | 7.606 | 1.433 | 1.111 | 2.152 | 6.745 | 12.217 |

## Provenance and remaining gates

- Profile source SHA-256: 503D618B363AD5E1EBE7C1BAE177D3C13E9D63C2BAB674AEC483F895F220C49F.
- Profile executable SHA-256: E2FB6C7CBBE39AE9D3BCD67271FDF7AA3A7090FFF7AF82791272ED3345DBF317.
- Seeder SHA-256: CFC0A3F215980BBAA335FF1AF3E8F293B09ED6C402248E0363CCD45D1995BB20.
- Graph SHA-256 remains DAF84DD922DA45E4E4DCF1A2323C2E5AACBF3435F11F50EA590E867BF57D7C98.

[Profiler-free query matrix](qa-261004-graph-candidate-query-matrix.md) still
records 1M investigation 1629.336/1500 ms FAIL and seed 532.237/240 s MISS.
The full plan remains 312/369; no phase or release readiness is promoted.

The [predeclared 1M paired diagnostic](qa-261004-paired-investigation-tail.md)
has completed with all 30 observations retained. It does not close acceptance.
Unresolved questions: exact signature/window reuse leverage before selecting
a recurrence optimization; 1M query and seed budgets remain unmet.

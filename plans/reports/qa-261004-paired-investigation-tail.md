# Paired investigation tail attribution — 2026-10-04

Outcome: exactly one predeclared canonical 1M diagnostic PASS, 1 test passed /
zero failed / 4 filtered, 468.03 s. All 30 observations retained; every measured
interval has exactly one completed engine and one completed service record.
Warm is separately paired and excluded. This is attribution, not acceptance.

## Workload and provenance

Same command/source/artifacts as the [10k preflight](qa-261004-paired-investigation-preflight.md),
with rows=1000000 and stage timing opt-in=1. Source unchanged after preflight.
Canonical counts: 1M events / 3k entities / 150k related / 80k bursts /
40 checkpoints / 9,800 snapshots / 20 investigations / 90 days.
All 31 actual-service calls scored and persisted. Warm recurrence plan used
the covering index and recorded 5,000 completed bound COUNT executions.

Seed wall **412,905.481/240,000 ms — MISS**; inner 412,819.772 ms:
generation 7,967.328, insertion 384,947.988, graph 16,960.655, inventory
70.963, checkpoints 128.425 and investigations 2,625.687 ms.
E2E total **38,954.782**, mean **1,298.493**, min/max **919.125/1,933.324 ms**.
Diagnostic p95 (29th ordered observation) **1,892.938 ms**, including stage
stderr overhead. It neither replaces nor changes the
[profiler-free 1,629.336/1,500 ms failure](qa-261004-graph-candidate-query-matrix.md).

## Same-call tail interpretation

The slowest calls are 15, 4 and 22, respectively 1,933.324, 1,892.938 and
1,750.042 ms. Their exclusive recurrence times are 883.653, 859.664 and
770.817 ms: about 46%, 45% and 44% of their own e2e durations. Recurrence
remains a substantial owner in the same slow calls, not just another process.

Those calls also have exclusive relationships 344.629, 373.204, 162.564 ms
and graph 295.075, 225.737, 377.405 ms. Another slow call (2, 1,503.298 ms)
has engine graph 431.089 ms and scoring 218.078 ms, alongside recurrence
456.924 ms. Do not claim a single universal tail cause or infer host
interference; no concurrent host measurement explains these intervals.

Service/input and engine totals are nested, not additive. Do not sum independent
stage p95s. Markers lie outside e2e clocks; source timing output lies inside.
No acceptance threshold, cap, SQL or corpus was changed.

## Service records — milliseconds

Service/input inclusive; remaining columns exclusive. All measured calls:

| Call | E2E | Service | Input | Setup | Window | Boundary | Entities | Relations | Recurrence | Finalize | Analysis | Sanitize | Persist |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 928.681 | 928.572 | 565.205 | 0.008 | 42.418 | 0.828 | 4.280 | 111.635 | 405.890 | 0.153 | 362.224 | 0.499 | 0.637 |
| 2 | 1503.298 | 1503.052 | 634.985 | 0.008 | 46.061 | 0.955 | 4.996 | 125.768 | 456.924 | 0.282 | 865.936 | 0.776 | 1.346 |
| 3 | 1576.490 | 1576.269 | 975.313 | 0.012 | 84.124 | 1.873 | 9.730 | 192.242 | 687.128 | 0.217 | 598.332 | 1.203 | 1.408 |
| 4 | 1892.938 | 1892.786 | 1434.268 | 0.012 | 184.990 | 2.387 | 13.735 | 373.204 | 859.664 | 0.289 | 457.247 | 0.546 | 0.712 |
| 5 | 1106.462 | 1106.320 | 739.728 | 0.009 | 48.026 | 1.724 | 7.532 | 156.481 | 525.805 | 0.160 | 365.471 | 0.461 | 0.650 |
| 6 | 1013.775 | 1013.587 | 594.896 | 0.008 | 44.401 | 0.928 | 4.433 | 111.758 | 433.220 | 0.157 | 417.486 | 0.516 | 0.681 |
| 7 | 971.870 | 971.742 | 619.446 | 0.008 | 47.248 | 0.947 | 4.641 | 121.138 | 445.314 | 0.158 | 351.208 | 0.426 | 0.653 |
| 8 | 981.154 | 981.041 | 573.880 | 0.009 | 42.530 | 0.838 | 4.194 | 105.597 | 420.543 | 0.178 | 405.932 | 0.486 | 0.734 |
| 9 | 919.125 | 918.979 | 568.938 | 0.008 | 44.694 | 0.881 | 4.322 | 110.223 | 408.665 | 0.152 | 348.888 | 0.486 | 0.660 |
| 10 | 990.958 | 990.797 | 602.762 | 0.009 | 42.701 | 0.858 | 4.409 | 120.699 | 433.935 | 0.160 | 386.555 | 0.660 | 0.810 |
| 11 | 956.225 | 956.106 | 576.057 | 0.008 | 44.311 | 0.912 | 4.195 | 110.970 | 415.514 | 0.155 | 378.995 | 0.447 | 0.598 |
| 12 | 1034.361 | 1034.216 | 618.038 | 0.008 | 47.045 | 1.263 | 4.692 | 117.864 | 447.015 | 0.158 | 415.069 | 0.467 | 0.636 |
| 13 | 1021.336 | 1021.215 | 624.170 | 0.009 | 48.794 | 1.123 | 4.461 | 120.154 | 449.412 | 0.226 | 395.934 | 0.483 | 0.618 |
| 14 | 1131.437 | 1131.018 | 613.125 | 0.008 | 47.131 | 0.923 | 4.723 | 116.919 | 443.260 | 0.169 | 515.297 | 1.124 | 1.463 |
| 15 | 1933.324 | 1933.080 | 1330.725 | 0.016 | 87.884 | 2.478 | 11.849 | 344.629 | 883.653 | 0.233 | 600.974 | 0.659 | 0.706 |
| 16 | 1312.872 | 1312.667 | 778.391 | 0.010 | 63.408 | 1.186 | 6.224 | 149.730 | 557.663 | 0.181 | 532.846 | 0.628 | 0.791 |
| 17 | 1046.959 | 1046.827 | 653.310 | 0.009 | 53.671 | 1.092 | 5.275 | 127.025 | 466.067 | 0.180 | 392.119 | 0.502 | 0.887 |
| 18 | 1208.246 | 1207.384 | 675.358 | 0.008 | 44.795 | 0.884 | 4.425 | 117.103 | 507.978 | 0.174 | 530.775 | 0.505 | 0.737 |
| 19 | 1382.014 | 1381.854 | 895.225 | 0.011 | 89.644 | 0.941 | 4.702 | 149.636 | 650.134 | 0.167 | 483.760 | 2.063 | 0.795 |
| 20 | 1493.747 | 1493.536 | 827.053 | 0.009 | 100.419 | 1.002 | 5.271 | 138.318 | 581.869 | 0.174 | 662.647 | 0.483 | 3.344 |
| 21 | 1451.366 | 1451.190 | 956.873 | 0.023 | 122.618 | 0.947 | 12.087 | 172.410 | 648.644 | 0.168 | 492.841 | 0.560 | 0.893 |
| 22 | 1750.042 | 1748.457 | 1001.981 | 0.008 | 61.381 | 1.198 | 5.688 | 162.564 | 770.817 | 0.332 | 743.531 | 0.639 | 2.298 |
| 23 | 1448.482 | 1446.319 | 890.582 | 0.010 | 96.927 | 1.071 | 6.083 | 167.736 | 618.508 | 0.258 | 550.967 | 3.832 | 0.928 |
| 24 | 1629.531 | 1629.376 | 952.944 | 0.010 | 147.991 | 1.151 | 5.213 | 185.439 | 612.958 | 0.192 | 671.596 | 2.003 | 2.824 |
| 25 | 1729.590 | 1729.452 | 1124.987 | 0.009 | 135.174 | 1.471 | 6.979 | 277.867 | 703.322 | 0.175 | 602.556 | 1.147 | 0.753 |
| 26 | 1268.038 | 1267.860 | 759.522 | 0.008 | 45.595 | 0.932 | 4.725 | 137.698 | 570.313 | 0.258 | 506.343 | 0.679 | 1.308 |
| 27 | 1359.172 | 1359.022 | 834.603 | 0.009 | 67.659 | 1.692 | 7.219 | 168.792 | 589.036 | 0.205 | 523.039 | 0.636 | 0.735 |
| 28 | 1225.918 | 1225.762 | 764.963 | 0.009 | 60.815 | 1.153 | 5.632 | 164.015 | 533.145 | 0.203 | 459.612 | 0.595 | 0.583 |
| 29 | 1246.995 | 1246.837 | 799.701 | 0.009 | 55.001 | 1.049 | 5.287 | 139.045 | 599.125 | 0.194 | 445.842 | 0.529 | 0.755 |
| 30 | 1440.375 | 1440.153 | 886.026 | 0.009 | 54.270 | 1.212 | 7.136 | 147.174 | 675.914 | 0.321 | 552.406 | 0.690 | 1.021 |

## Engine records — milliseconds

Engine inclusive; remaining columns exclusive, nested in service Analysis:

| Call | Engine | Preprocess | Graph | Chain | Failure | Preparation | Score | Selection |
|---|---|---|---|---|---|---|---|---|
| 1 | 353.049 | 2.061 | 161.530 | 1.357 | 20.886 | 41.657 | 101.274 | 24.285 |
| 2 | 850.722 | 4.072 | 431.089 | 3.344 | 56.434 | 95.834 | 218.078 | 41.871 |
| 3 | 580.667 | 3.103 | 251.276 | 1.732 | 30.753 | 56.688 | 189.975 | 47.140 |
| 4 | 444.767 | 3.546 | 225.737 | 1.604 | 25.968 | 52.540 | 112.794 | 22.579 |
| 5 | 356.189 | 2.245 | 168.005 | 1.309 | 21.689 | 41.285 | 100.358 | 21.298 |
| 6 | 407.269 | 2.238 | 190.614 | 1.554 | 27.220 | 46.474 | 117.135 | 22.034 |
| 7 | 342.727 | 2.165 | 160.831 | 1.203 | 21.382 | 39.833 | 96.849 | 20.464 |
| 8 | 396.071 | 2.427 | 194.995 | 1.367 | 23.751 | 44.530 | 107.314 | 21.687 |
| 9 | 338.707 | 2.126 | 158.883 | 1.210 | 20.288 | 40.598 | 95.528 | 20.075 |
| 10 | 376.538 | 2.444 | 179.219 | 1.772 | 23.139 | 45.076 | 102.536 | 22.352 |
| 11 | 369.617 | 2.229 | 168.827 | 1.400 | 22.669 | 46.915 | 105.739 | 21.838 |
| 12 | 404.617 | 2.299 | 190.776 | 1.409 | 25.604 | 46.070 | 115.107 | 23.353 |
| 13 | 384.275 | 2.364 | 180.976 | 1.374 | 23.262 | 43.912 | 110.337 | 22.048 |
| 14 | 494.758 | 2.378 | 187.555 | 1.412 | 23.701 | 44.550 | 177.789 | 57.372 |
| 15 | 587.697 | 3.475 | 295.075 | 2.871 | 37.563 | 67.351 | 152.877 | 28.485 |
| 16 | 519.960 | 2.607 | 266.971 | 1.684 | 27.964 | 55.981 | 138.048 | 26.704 |
| 17 | 381.883 | 2.369 | 180.610 | 1.403 | 23.535 | 43.120 | 108.407 | 22.440 |
| 18 | 520.727 | 2.473 | 229.509 | 1.414 | 30.831 | 72.195 | 161.983 | 22.321 |
| 19 | 468.644 | 2.453 | 216.018 | 1.720 | 28.936 | 49.957 | 146.524 | 23.036 |
| 20 | 646.739 | 3.522 | 328.482 | 1.413 | 46.549 | 71.458 | 168.097 | 27.218 |
| 21 | 480.380 | 2.367 | 257.602 | 2.326 | 24.923 | 47.145 | 116.283 | 29.732 |
| 22 | 730.865 | 8.896 | 377.405 | 1.916 | 42.022 | 88.345 | 172.621 | 39.661 |
| 23 | 539.054 | 2.772 | 238.563 | 1.549 | 38.398 | 60.262 | 158.008 | 39.502 |
| 24 | 650.179 | 7.077 | 332.944 | 9.001 | 37.687 | 94.390 | 138.760 | 30.319 |
| 25 | 591.409 | 7.780 | 266.166 | 1.441 | 52.228 | 60.321 | 166.646 | 36.827 |
| 26 | 492.293 | 4.189 | 229.191 | 1.676 | 30.939 | 57.338 | 138.696 | 30.264 |
| 27 | 510.715 | 2.976 | 250.142 | 2.218 | 31.878 | 53.618 | 141.648 | 28.236 |
| 28 | 447.550 | 2.880 | 217.980 | 1.575 | 27.378 | 49.625 | 123.596 | 24.517 |
| 29 | 433.612 | 2.799 | 210.311 | 1.557 | 25.913 | 48.377 | 114.505 | 30.151 |
| 30 | 536.434 | 4.127 | 263.403 | 1.798 | 33.887 | 58.489 | 145.700 | 29.030 |

## Verification limits and next action

Review and cleanup checks passed as recorded in the preflight. Production
source remains at the correctness-tested graph revision; this test-only change
does not constitute full default verification or current installer evidence.
Coverage and installed working set were not measured. All acceptance counts
remain 312/369, 17/25 completed phases.

Next: counsel on a bounded recurrence experiment, after quantifying actual
signature/window repetition and overlap. Preserve inclusive since/exclusive
before, multiplicity, original order, duplicate IDs within/across 150-query
blocks, checked overflow and snapshot/error behavior. Never allocate storage
proportional to all historical timestamps.

Unresolved questions: actual signature/window reuse leverage; query and seed
budgets remain unmet. Owned fixture is absent after completion; D: free
2,992,541,696 bytes; all owned Cargo/rustc/seeder/profile processes exited.
Focused release/devtools Clippy for this diagnostic target passed with
`-D warnings` in 1.79 s; rustfmt and scoped diff checks passed.

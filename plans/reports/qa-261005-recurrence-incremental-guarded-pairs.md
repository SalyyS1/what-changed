# Guarded rolling COUNT matched pairs — 2026-10-05

Frozen span-guard revision, test-only. All 13 parity tests and the one fixed
88-pair benchmark passed. No production activation or acceptance claim.
Protocol and source hashes: [prototype report](qa-261005-recurrence-incremental-prototype.md).
Unit artifact SHA-256: `6AC06A96414CFEDC69FD7FAB6E2C82006323AB28A25915A204FF64D651871EC6`.

## Round summaries

| Family | Round | Old mean ms | New mean ms | Aggregate change % | Faster pairs/11 | New full/delta SQL |
|---|---:|---:|---:|---:|---:|---:|
| canonical_like_input | 1 | 30.917582 | 25.810782 | -16.5175 | 11 | 3429/3142 |
| unique_sparse | 1 | 6.096845 | 6.067818 | -0.4761 | 4 | 5000/0 |
| dense_overlap | 1 | 288.930500 | 15.897918 | -94.4977 | 11 | 64/9872 |
| wide_shift_tiny_overlap | 1 | 92.502518 | 96.631455 | 4.4636 | 4 | 5000/0 |
| canonical_like_input | 2 | 31.987927 | 26.146827 | -18.2603 | 11 | 3429/3142 |
| unique_sparse | 2 | 5.664545 | 5.837082 | 3.0459 | 2 | 5000/0 |
| dense_overlap | 2 | 262.126555 | 15.527136 | -94.0765 | 11 | 64/9872 |
| wide_shift_tiny_overlap | 2 | 75.717364 | 75.290109 | -0.5643 | 5 | 5000/0 |

All baseline calls execute 5,000 COUNTs; identical reuses are zero.
Negative change means faster. All 88 observations, including the larger
first-round dense baseline and tiny-overlap timings, are retained without
outlier removal or attribution to host conditions. Sparse and tiny-overlap
aggregate changes vary in sign; no zero-overhead claim is established.
Canonical-like density/bounds remain synthetic, not actual canonical history.
Counsel authorizes a separately reviewed production candidate, not acceptance
or release. [Original unguarded regression](qa-261005-recurrence-incremental-unguarded-pairs.md) remains preserved.

## Every paired observation (nanoseconds)

| Family | Round | Pair | Old first | Old ns | New ns |
|---|---:|---:|---:|---:|---:|
| canonical_like_input | 1 | 1 | 1 | 30345400 | 23537300 |
| canonical_like_input | 1 | 2 | 0 | 30432400 | 27134300 |
| canonical_like_input | 1 | 3 | 1 | 30885500 | 29735600 |
| canonical_like_input | 1 | 4 | 0 | 29033400 | 25778200 |
| canonical_like_input | 1 | 5 | 1 | 31174700 | 24376300 |
| canonical_like_input | 1 | 6 | 0 | 29965900 | 25335100 |
| canonical_like_input | 1 | 7 | 1 | 31589100 | 25608000 |
| canonical_like_input | 1 | 8 | 0 | 32176600 | 24444900 |
| canonical_like_input | 1 | 9 | 1 | 32861400 | 25679300 |
| canonical_like_input | 1 | 10 | 0 | 30929300 | 26204800 |
| canonical_like_input | 1 | 11 | 1 | 30699700 | 26084800 |
| unique_sparse | 1 | 1 | 1 | 6778000 | 6023200 |
| unique_sparse | 1 | 2 | 0 | 5975100 | 6322600 |
| unique_sparse | 1 | 3 | 1 | 6428100 | 5938100 |
| unique_sparse | 1 | 4 | 0 | 5277700 | 5500800 |
| unique_sparse | 1 | 5 | 1 | 5123000 | 5400100 |
| unique_sparse | 1 | 6 | 0 | 6017400 | 6464500 |
| unique_sparse | 1 | 7 | 1 | 5861200 | 5968000 |
| unique_sparse | 1 | 8 | 0 | 6821000 | 6566900 |
| unique_sparse | 1 | 9 | 1 | 6929100 | 6041600 |
| unique_sparse | 1 | 10 | 0 | 5861500 | 6401700 |
| unique_sparse | 1 | 11 | 1 | 5993200 | 6118500 |
| dense_overlap | 1 | 1 | 1 | 264025300 | 15378800 |
| dense_overlap | 1 | 2 | 0 | 261552900 | 14451400 |
| dense_overlap | 1 | 3 | 1 | 261181100 | 15504000 |
| dense_overlap | 1 | 4 | 0 | 262939000 | 14320600 |
| dense_overlap | 1 | 5 | 1 | 267683400 | 16803300 |
| dense_overlap | 1 | 6 | 0 | 260127300 | 16716400 |
| dense_overlap | 1 | 7 | 1 | 262621800 | 15271800 |
| dense_overlap | 1 | 8 | 0 | 270186800 | 15340500 |
| dense_overlap | 1 | 9 | 1 | 259720300 | 16106200 |
| dense_overlap | 1 | 10 | 0 | 305464600 | 15327100 |
| dense_overlap | 1 | 11 | 1 | 502733000 | 19657000 |
| wide_shift_tiny_overlap | 1 | 1 | 1 | 91533500 | 97432300 |
| wide_shift_tiny_overlap | 1 | 2 | 0 | 80047000 | 99624300 |
| wide_shift_tiny_overlap | 1 | 3 | 1 | 82452500 | 80747700 |
| wide_shift_tiny_overlap | 1 | 4 | 0 | 86029700 | 83085000 |
| wide_shift_tiny_overlap | 1 | 5 | 1 | 82975500 | 89008800 |
| wide_shift_tiny_overlap | 1 | 6 | 0 | 125046400 | 128573100 |
| wide_shift_tiny_overlap | 1 | 7 | 1 | 136318000 | 121251600 |
| wide_shift_tiny_overlap | 1 | 8 | 0 | 92881900 | 111409600 |
| wide_shift_tiny_overlap | 1 | 9 | 1 | 87837800 | 98312500 |
| wide_shift_tiny_overlap | 1 | 10 | 0 | 78032600 | 77362300 |
| wide_shift_tiny_overlap | 1 | 11 | 1 | 74372800 | 76138800 |
| canonical_like_input | 2 | 1 | 0 | 29752000 | 26181200 |
| canonical_like_input | 2 | 2 | 1 | 34474500 | 26326500 |
| canonical_like_input | 2 | 3 | 0 | 31533300 | 27739500 |
| canonical_like_input | 2 | 4 | 1 | 31575900 | 27862000 |
| canonical_like_input | 2 | 5 | 0 | 31373100 | 26488800 |
| canonical_like_input | 2 | 6 | 1 | 31679600 | 24812700 |
| canonical_like_input | 2 | 7 | 0 | 33333500 | 26485700 |
| canonical_like_input | 2 | 8 | 1 | 31306300 | 24781900 |
| canonical_like_input | 2 | 9 | 0 | 30959000 | 26385100 |
| canonical_like_input | 2 | 10 | 1 | 33047300 | 24933900 |
| canonical_like_input | 2 | 11 | 0 | 32832700 | 25617800 |
| unique_sparse | 2 | 1 | 0 | 5740000 | 6081800 |
| unique_sparse | 2 | 2 | 1 | 6695400 | 5472100 |
| unique_sparse | 2 | 3 | 0 | 5312200 | 5855400 |
| unique_sparse | 2 | 4 | 1 | 5742800 | 6159500 |
| unique_sparse | 2 | 5 | 0 | 5251600 | 5630600 |
| unique_sparse | 2 | 6 | 1 | 5422200 | 5544300 |
| unique_sparse | 2 | 7 | 0 | 5714900 | 6377900 |
| unique_sparse | 2 | 8 | 1 | 6104300 | 5901600 |
| unique_sparse | 2 | 9 | 0 | 5349300 | 5585300 |
| unique_sparse | 2 | 10 | 1 | 5622000 | 5706200 |
| unique_sparse | 2 | 11 | 0 | 5355300 | 5893200 |
| dense_overlap | 2 | 1 | 0 | 259184600 | 15740400 |
| dense_overlap | 2 | 2 | 1 | 274741700 | 16531900 |
| dense_overlap | 2 | 3 | 0 | 272472700 | 16279900 |
| dense_overlap | 2 | 4 | 1 | 260898100 | 15950900 |
| dense_overlap | 2 | 5 | 0 | 256561700 | 14796400 |
| dense_overlap | 2 | 6 | 1 | 255613300 | 14334700 |
| dense_overlap | 2 | 7 | 0 | 268964800 | 14518700 |
| dense_overlap | 2 | 8 | 1 | 261167200 | 16371000 |
| dense_overlap | 2 | 9 | 0 | 264956800 | 14886300 |
| dense_overlap | 2 | 10 | 1 | 252864100 | 15997700 |
| dense_overlap | 2 | 11 | 0 | 255967100 | 15390600 |
| wide_shift_tiny_overlap | 2 | 1 | 0 | 75657600 | 74192500 |
| wide_shift_tiny_overlap | 2 | 2 | 1 | 73909400 | 77455400 |
| wide_shift_tiny_overlap | 2 | 3 | 0 | 74631100 | 74251600 |
| wide_shift_tiny_overlap | 2 | 4 | 1 | 73650100 | 78528000 |
| wide_shift_tiny_overlap | 2 | 5 | 0 | 76948900 | 71609000 |
| wide_shift_tiny_overlap | 2 | 6 | 1 | 74368000 | 74654600 |
| wide_shift_tiny_overlap | 2 | 7 | 0 | 72842100 | 74508200 |
| wide_shift_tiny_overlap | 2 | 8 | 1 | 76370300 | 69960500 |
| wide_shift_tiny_overlap | 2 | 9 | 0 | 76219500 | 77292500 |
| wide_shift_tiny_overlap | 2 | 10 | 1 | 79268700 | 76128900 |
| wide_shift_tiny_overlap | 2 | 11 | 0 | 79025300 | 79610000 |


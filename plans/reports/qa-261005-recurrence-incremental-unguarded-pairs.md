# Unguarded rolling COUNT matched pairs — 2026-10-05

Frozen first variant, test-only. Complete-map parity/probe assertions passed;
all 88 observations retained. No production activation or acceptance claim.
Protocol and source hashes: [prototype report](qa-261005-recurrence-incremental-prototype.md).
Unit artifact SHA-256: `93AC86D012923CAF53DAF56068CB2AC52E1387474135C5FE44F3260185A8B1E0`.

## Round summaries

| Family | Round | Old mean ms | New mean ms | Aggregate change % | Faster pairs/11 | New full/delta SQL |
|---|---:|---:|---:|---:|---:|---:|
| canonical_like_input | 1 | 35.362645 | 28.773282 | -18.6337 | 11 | 3429/3142 |
| canonical_like_input | 2 | 32.265236 | 26.051491 | -19.2583 | 11 | 3429/3142 |
| unique_sparse | 1 | 5.634164 | 5.754745 | 2.1402 | 5 | 5000/0 |
| unique_sparse | 2 | 5.659336 | 5.626918 | -0.5728 | 6 | 5000/0 |
| dense_overlap | 1 | 253.872400 | 15.262364 | -93.9882 | 11 | 64/9872 |
| dense_overlap | 2 | 252.744018 | 14.977655 | -94.0740 | 11 | 64/9872 |
| wide_shift_tiny_overlap | 1 | 72.971755 | 127.665255 | 74.9516 | 0 | 1000/8000 |
| wide_shift_tiny_overlap | 2 | 74.243455 | 130.954400 | 76.3851 | 0 | 1000/8000 |

All baseline calls execute 5,000 COUNTs. Negative change means faster;
canonical-like density/bounds are synthetic, not measured canonical history.
Unique-sparse results vary in sign; all 22 tiny-overlap pairs are slower.
The unguarded variant is not approved for production. Counsel authorizes one
span-cost eligibility refinement, with a separate unchanged-protocol rerun;
this file preserves the original observations rather than replacing them.

## Every paired observation (nanoseconds)

| Family | Round | Pair | Old first | Old ns | New ns |
|---|---:|---:|---:|---:|---:|
| canonical_like_input | 1 | 1 | 1 | 32099700 | 26026500 |
| canonical_like_input | 1 | 2 | 0 | 31796900 | 26988700 |
| canonical_like_input | 1 | 3 | 1 | 32961800 | 26656400 |
| canonical_like_input | 1 | 4 | 0 | 34813200 | 28367000 |
| canonical_like_input | 1 | 5 | 1 | 36733000 | 28385300 |
| canonical_like_input | 1 | 6 | 0 | 30308100 | 27136000 |
| canonical_like_input | 1 | 7 | 1 | 32468300 | 24695000 |
| canonical_like_input | 1 | 8 | 0 | 32392600 | 25783200 |
| canonical_like_input | 1 | 9 | 1 | 29854000 | 26482800 |
| canonical_like_input | 1 | 10 | 0 | 56933500 | 49389200 |
| canonical_like_input | 1 | 11 | 1 | 38628000 | 26596000 |
| unique_sparse | 1 | 1 | 1 | 5648700 | 5904900 |
| unique_sparse | 1 | 2 | 0 | 5568900 | 6071100 |
| unique_sparse | 1 | 3 | 1 | 5262200 | 5098400 |
| unique_sparse | 1 | 4 | 0 | 5002700 | 5805000 |
| unique_sparse | 1 | 5 | 1 | 5686800 | 5333400 |
| unique_sparse | 1 | 6 | 0 | 5398600 | 5635800 |
| unique_sparse | 1 | 7 | 1 | 5758800 | 5550600 |
| unique_sparse | 1 | 8 | 0 | 5825600 | 5958300 |
| unique_sparse | 1 | 9 | 1 | 6089100 | 5985900 |
| unique_sparse | 1 | 10 | 0 | 5651900 | 6185000 |
| unique_sparse | 1 | 11 | 1 | 6082500 | 5773800 |
| dense_overlap | 1 | 1 | 1 | 249620900 | 15393300 |
| dense_overlap | 1 | 2 | 0 | 244099000 | 14396000 |
| dense_overlap | 1 | 3 | 1 | 252741700 | 15236700 |
| dense_overlap | 1 | 4 | 0 | 250007700 | 15756400 |
| dense_overlap | 1 | 5 | 1 | 255221200 | 14691500 |
| dense_overlap | 1 | 6 | 0 | 256529000 | 14626100 |
| dense_overlap | 1 | 7 | 1 | 259138600 | 15456800 |
| dense_overlap | 1 | 8 | 0 | 256989100 | 16492100 |
| dense_overlap | 1 | 9 | 1 | 256092900 | 15407200 |
| dense_overlap | 1 | 10 | 0 | 258111700 | 15622700 |
| dense_overlap | 1 | 11 | 1 | 254044600 | 14807200 |
| wide_shift_tiny_overlap | 1 | 1 | 1 | 72865100 | 134975600 |
| wide_shift_tiny_overlap | 1 | 2 | 0 | 73007100 | 130805400 |
| wide_shift_tiny_overlap | 1 | 3 | 1 | 75548300 | 129995100 |
| wide_shift_tiny_overlap | 1 | 4 | 0 | 70879400 | 127983400 |
| wide_shift_tiny_overlap | 1 | 5 | 1 | 72729500 | 125086600 |
| wide_shift_tiny_overlap | 1 | 6 | 0 | 72221900 | 121432200 |
| wide_shift_tiny_overlap | 1 | 7 | 1 | 72814400 | 127046100 |
| wide_shift_tiny_overlap | 1 | 8 | 0 | 70671700 | 125758900 |
| wide_shift_tiny_overlap | 1 | 9 | 1 | 73668300 | 126793600 |
| wide_shift_tiny_overlap | 1 | 10 | 0 | 74322000 | 123803500 |
| wide_shift_tiny_overlap | 1 | 11 | 1 | 73961600 | 130637400 |
| canonical_like_input | 2 | 1 | 0 | 33040900 | 27644900 |
| canonical_like_input | 2 | 2 | 1 | 30513900 | 25836500 |
| canonical_like_input | 2 | 3 | 0 | 32848800 | 26431800 |
| canonical_like_input | 2 | 4 | 1 | 31620100 | 26240200 |
| canonical_like_input | 2 | 5 | 0 | 29362700 | 26334400 |
| canonical_like_input | 2 | 6 | 1 | 34175400 | 26776900 |
| canonical_like_input | 2 | 7 | 0 | 32722200 | 26287400 |
| canonical_like_input | 2 | 8 | 1 | 33138400 | 26471200 |
| canonical_like_input | 2 | 9 | 0 | 31998300 | 24697600 |
| canonical_like_input | 2 | 10 | 1 | 33247000 | 24657100 |
| canonical_like_input | 2 | 11 | 0 | 32249900 | 25188400 |
| unique_sparse | 2 | 1 | 0 | 5688100 | 5687600 |
| unique_sparse | 2 | 2 | 1 | 6080100 | 5531500 |
| unique_sparse | 2 | 3 | 0 | 5467900 | 5643100 |
| unique_sparse | 2 | 4 | 1 | 6048200 | 5799000 |
| unique_sparse | 2 | 5 | 0 | 5689100 | 5794300 |
| unique_sparse | 2 | 6 | 1 | 5731100 | 5470300 |
| unique_sparse | 2 | 7 | 0 | 5568800 | 5648500 |
| unique_sparse | 2 | 8 | 1 | 5635400 | 5447600 |
| unique_sparse | 2 | 9 | 0 | 5343000 | 5764400 |
| unique_sparse | 2 | 10 | 1 | 5547800 | 5495100 |
| unique_sparse | 2 | 11 | 0 | 5453200 | 5614700 |
| dense_overlap | 2 | 1 | 0 | 253257000 | 14823600 |
| dense_overlap | 2 | 2 | 1 | 261935700 | 14513700 |
| dense_overlap | 2 | 3 | 0 | 255154500 | 14131500 |
| dense_overlap | 2 | 4 | 1 | 245820100 | 14876300 |
| dense_overlap | 2 | 5 | 0 | 250721700 | 16113900 |
| dense_overlap | 2 | 6 | 1 | 246996400 | 15686100 |
| dense_overlap | 2 | 7 | 0 | 251715400 | 15145000 |
| dense_overlap | 2 | 8 | 1 | 256598500 | 14736000 |
| dense_overlap | 2 | 9 | 0 | 262814900 | 15191600 |
| dense_overlap | 2 | 10 | 1 | 247567600 | 15268200 |
| dense_overlap | 2 | 11 | 0 | 247602400 | 14268300 |
| wide_shift_tiny_overlap | 2 | 1 | 0 | 74716200 | 132043300 |
| wide_shift_tiny_overlap | 2 | 2 | 1 | 72907900 | 127256400 |
| wide_shift_tiny_overlap | 2 | 3 | 0 | 76555400 | 130629100 |
| wide_shift_tiny_overlap | 2 | 4 | 1 | 76108200 | 129866100 |
| wide_shift_tiny_overlap | 2 | 5 | 0 | 71790000 | 126561300 |
| wide_shift_tiny_overlap | 2 | 6 | 1 | 71819700 | 134463800 |
| wide_shift_tiny_overlap | 2 | 7 | 0 | 77001800 | 139420300 |
| wide_shift_tiny_overlap | 2 | 8 | 1 | 75219500 | 136858800 |
| wide_shift_tiny_overlap | 2 | 9 | 0 | 73061600 | 127969500 |
| wide_shift_tiny_overlap | 2 | 10 | 1 | 71391200 | 129003900 |
| wide_shift_tiny_overlap | 2 | 11 | 0 | 76106500 | 126425900 |


# Runtime source delta review proposal — 2026-10-30

**State: PROPOSED; exact new inventory is not approved.** Existing notice gate remains blocked. No publication, G2, G3, installed acceptance or new human decision is asserted.

## Decision scope

Approve or decline one owned runner source change for a new unsigned QA build. Previous user approval covered exact old inventory `cc3bb9c020ec053b11df341d925f3f25e26a6dc9f0cfbdb5ab1d1cf3d3ec5554`; it does not automatically approve the new aggregate inventory. The original frozen inventory, review and user approval remain unchanged.

Proposed inventory: [runtime-delta-inventory.json](runtime-delta-inventory.json), an exact byte copy of `runner/bundle/bundle-inventory.json`. This is a proposal, not the active release freeze.

| Inventory | File bytes | Runtime files | Sum of runtime file bytes | Raw inventory SHA256 |
|---|---:|---:|---:|---|
| Approved original | 36898 | 197 | 543579634 | `cc3bb9c020ec053b11df341d925f3f25e26a6dc9f0cfbdb5ab1d1cf3d3ec5554` |
| Proposed current | 53491 | 197 | 543580088 | `31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299` |

Inventory SHA256 hashes the actual JSON file bytes. Runtime size is the sum of listed files, excluding the inventory JSON. The proposal adds 454 runtime bytes; this is not an installer or complete-download hash.

## Exact changed file

| File | Approved original | Proposed current |
|---|---|---|
| `src/policy.mjs` bytes | 4058 | 4512 |
| SHA256 | `ee1c8922d8c990ecef9b829ad87f2857c7eadc38dfa7d0c2a194f6fcc4ac38e6` | `0a4cd02e3153754ceb133cc76f5eab9025ce470508fb03687ce66df5c0a0c912` |

Only `originOf` changes: reject explicit port 0 and validate an explicit TCP port is an integer from 1 to 65535. Existing URL parsing also rejects malformed/out-of-range URLs. This preserves the permission boundary to an actual connectable endpoint; it does not enable remote targets.

Exact insertion after the existing `Unsupported origin` check:

```js
  // URL accepts port 0, but it is not a connectable application endpoint and
  // would make a permission grant ambiguous (the OS chooses an ephemeral port
  // only when binding, never when connecting). Keep every route bound to a
  // real TCP port and reject malformed/out-of-range values before normalizing.
  if (url.port && (!/^\d+$/.test(url.port) || Number(url.port) < 1 || Number(url.port) > 65535)) throw new Error('Unsupported origin port');
```

Original policy bytes were independently read through GitHub contents API at hosted build source `e5906ea6ae2fb06859d7b361df00b36425dd496c`; their size and hash match the original frozen record. No large artifact was downloaded.

## Third-party equality proof

- All **186** third-party records are identical by exact path, bytes and SHA256; no additions/removals. They total **543539407 bytes**.
- Node v24.12.0, Playwright 1.62.1, ungoogled Chromium 156.0.8078.47-1.1 and the browser path/checksum source are unchanged.
- Original and proposed third-party canonical record digest: `333c162ddabca2420481ed78ff62dbf1cb5aab766defc016e875d7de10f8122a`.
- Canonicalization: filter out paths beginning `src/`; map each record to properties `path`, `bytes`, `sha256` in that order; sort paths in ascending JavaScript code-unit order (`a.path < b.path`); hash UTF-8 `JSON.stringify(records) + "\n"`.
- Complete bundle guard rehashed the actual current 197-file bundle and checked source parity, file-list completeness and redirects. It passed; installed runtime and confinement are still unproven.
- Every reviewed license/source file still matches its review-record hash. Packaged notice payload is unchanged.

Existing review/approval/payload file hashes:

| File | SHA256 |
|---|---|
| `release/runner-notice-review.json` | `105103b3164a44f4a99a78395c3ad976a3591d7b9e997eb6b3cf05c6058eb0f7` |
| `release/runner-notice-approval.json` | `c91ae47d2377f9540b499ed1a88c97b9ec7c769419b5c1b5c26ddff8ee6235f2` |
| `release/RUNNER_THIRD_PARTY_NOTICES.txt` | `8ad9c7c28d00641f26af587959a1020e60749fd017a3d60945f4efcb53010d9a` |

## Current owned source consistency

All 11 inventory source records match exact current files in `runner/src`, and the complete source-file set is present. Ten are unchanged. Owned source total: 40681 bytes. Current source record digest under the same canonicalization: `357a9b82684f9ad6c54667f1e6e70850df4652c8ebf3dcd3873b2fb0710146f1`.

Local source HEAD: `65ecf83a5d90a2c2118d8798d70aaad3f59f100b`. Concurrent website work is outside this packet. A future build must record its actual clean source commit; this proposal freezes only the listed runtime inputs.

| Inventory source path | Bytes | Current source SHA256 | Original comparison |
|---|---:|---|---|
| `src/browser-session.mjs` | 3015 | `fe03c59d93af3de42a908b22611e310151f7f31e8f69ee91415789e529ca0f28` | Unchanged |
| `src/egress-proxy.mjs` | 5157 | `51107f3099082472a79bc74941e8923f9d3b5343272126dcf2df817de924bb44` | Unchanged |
| `src/execute-recipe.mjs` | 6372 | `ec5bf4b61469509bdd636d7301723e7e94270d29a69b668770ee8c5fe61a906a` | Unchanged |
| `src/export-harness.mjs` | 3776 | `b15b8e4dab5999a9e2df8654c66feaaf21b01c8f3bf22398e4cc8114c18b6d35` | Unchanged |
| `src/main.mjs` | 5450 | `c57b2d39c9aae6e9670b6cbf63d406bc8d520a31dc0ed44d2f06fcbe05f28f5a` | Unchanged |
| `src/policy.mjs` | 4512 | `0a4cd02e3153754ceb133cc76f5eab9025ce470508fb03687ce66df5c0a0c912` | Changed |
| `src/protocol.mjs` | 1463 | `7c51f310b41de0c9421444cfbfce8ac5bb5cb7e860bb1d63841604844a3fd139` | Unchanged |
| `src/recipe-validation.mjs` | 5031 | `3983b19981027add69d9f8a4c9d89688d53e65bfb3fc03641c26d5a2a452533a` | Unchanged |
| `src/record-locator.mjs` | 1482 | `784c3a3d39f054c7b62093e03976f3abb0016239647c0bcc47cf065a81589806` | Unchanged |
| `src/revision-marker.mjs` | 1332 | `d0a02aacc98da7c2548a871fb285fdaefdc7fcc29c66b8ccc1342a37aeaf7edd` | Unchanged |
| `src/session-authority.mjs` | 3091 | `afcb9b879a4ccdb889275e6d368704148caae2e04f4be10bf4957ef933c42cd7` | Unchanged |

## Existing gate and checks

- `node scripts/check-runner-bundle.mjs runner/bundle`: passes on proposed 197 files / 543580088 bytes / `31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299`.
- `node scripts/check-runner-notices.mjs`: currently fails closed because review/approval bind the old aggregate inventory. This packet does not change that check.
- Earlier same-source runner policy/recipe/protocol suite: 15/15 pass; packaging/release guard suite: 55/55 pass. See [release audit](../reports/release-261030-1908-readiness.md). No native acceptance is inferred.

## After an explicit approval of this exact proposal

1. Record the actual user answer as a new decision evidence file bound to this report hash and proposed inventory SHA256. Keep the original approval as history; do not invent a response or approver.
2. Copy the approved proposal bytes to `release/runner-runtime-inventory.json`. Update the active notice review inventory/reference to that genuine new evidence; retain unchanged vendor versions, terms, source archive hashes and notice payload. Any other byte change requires another comparison and scope decision.
3. Run inventory/source preflight and existing bundle/notice/guard tests. The current hydration script copies current source against frozen inventory, so its inputs must match before downloading. No gate bypass is authorized.
4. Freeze the intended clean source revision and build through the hosted Windows preview route. Stage all seven verified corresponding-source archives, notices and build/relink instructions together with the installer.
5. Record resulting installer/full artifact hashes and source provenance. Native install/upgrade, clean Windows, privacy/network/lifecycle/accessibility, performance/soak and real-owner acceptance remain required. Public release/site download still requires exact G2/G3 approval.

## Approval question for controller

Approve this exact +454-byte owned `policy.mjs` delta and proposed runtime inventory `31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299` for a new unsigned QA build, retaining unchanged third-party bytes/notices and their source-delivery obligations? This authorizes neither public-v1 publication nor G2/G3 closure.

Unresolved: user decision on this exact proposal; clean Windows host and consenting pilot owners for remaining acceptance.

# Public download format proposal

Status: proposed; no approval, schema change, Release or download activation.

## Measured problem

Successful QA run `41091508412`, source `75dcb67c`, produced complete Actions
artifact `12995563059` at 2,227,565,645 bytes. Its seven pinned browser-source
archives total 2,046,353,087 bytes before ZIP metadata, receipts and instructions.
The complete QA ZIP exceeds 2 GiB by 80,081,997 bytes.

[GitHub's release documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
requires each Release asset to be under 2 GiB and allows multiple assets. The
current public manifest accepts only one installer EXE. Neither route describes
the complete approved QA packet. Its existing same-Actions-artifact approval
remains valid; this proposal does not change it.

## Recommended public route

Publish one frozen GitHub Release with an installer and a corresponding-source
ZIP as a single release set. Both must be available from that same release with
the same access and retention. The source ZIP contains all seven exact archives,
the descriptor, verification receipt, checksums, complete notices and build/relink
instructions. Check its actual final size; the archive totals above alone do not
prove the ZIP fits. Keep the current complete Actions artifact for QA.

The landing's primary action would download the installer and show the matching
source download beside its version, size and checksum. Users would install only
the app; the source ZIP would remain equally accessible without an account or a
source request. This changes the previously approved single-packet presentation
and therefore requires a new explicit decision before implementation/publication.
No source is dropped, no upstream-homepage link substitutes for delivery, and no
three-year written offer is created.

## Coordinated implementation after format approval

1. Extend the manifest schema, validator and release assessment together to bind
   both asset URLs, exact bytes/hashes, source descriptor and runtime inventory
   to the same frozen source/version. Keep the installer identity separate from
   the source ZIP identity. Existing unavailable manifests must remain valid.
2. Prepare the source ZIP on hosted CI from already verified source materials;
   verify its contents and size before upload. Do not repack reviewed inputs or
   change their hashes. Missing sources fail the entire publication preparation.
3. Prepare a draft Release with both assets, notices and checksums; verify actual
   remote bytes and source co-availability before exact G3 review. Keep the site
   unavailable while that review or native acceptance remains open.
4. Make the download surface display both assets and their relationship. Test
   absent/mismatched sources, changed versions/hashes, expired/unavailable assets
   and unsupported manifests. Any failed required binding keeps downloads off.
5. Publish only after native/clean-Windows/soak/pilot evidence and G2/G3 approve
   that exact release set. Freeze a new candidate if implementation changes
   packaged code; do not label a later build with the current candidate's hashes.

## Alternative

Retain one complete ZIP at a separately approved HTTPS host that accepts its
actual size, then change the manifest to bind that ZIP and its inner installer/
source identities. This preserves a single downloadable packet but adds hosting,
retention and large-download operations. No such host has been supplied.

## Unresolved decision

Approve the recommended same-Release two-asset format, or supply an approved host
for one complete packet. This format decision does not itself approve public v1,
new license choices, production brand assets or unperformed acceptance checks.

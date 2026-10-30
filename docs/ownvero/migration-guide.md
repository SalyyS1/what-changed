# From What Changed to the owner workspace

**Evidence state: development-source guidance; no approved owner-workspace installer or completed native acceptance.**

Ownvero is the draft name for a new owner workflow. The public identity switch is
not approved yet. This local source preview does not rename the public repository,
change the Pages base path, or publish a new installer.

## Your existing history

The first transition retains dev.whatchanged.app as the application identity and
uses the existing resolved per-user data root. Existing event history, collector
preferences, watched folders, checkpoints, and local diagnostics keep their roles.
Creating an owner app does not automatically enable collectors.

The owner workspace has its own store under the same data root. A linked local
project is optional and does not convert machine events into confirmed business
capabilities. Keep backups and follow the release-specific upgrade guidance when
an approved installer becomes available.

## Two decisions with different meanings

**Mark working** records a diagnostic checkpoint: the machine or project had a
known-good starting point. **Accept change** records the owner’s decision about a
candidate and its reviewed behavior evidence. The new action does not reinterpret
old checkpoints or turn them into business acceptance.

## Installation and update limits

The existing unsigned QA installer predates this owner workflow. It is not linked
as an Ownvero download. A future unsigned artifact may show Unknown publisher;
a matching checksum detects corruption and does not replace code signing.

The updater has no approved production feed or release public key. No automatic
background update is implied. WebView2 setup may use the network if the runtime is
missing. The owner runner introduces a separate permission and packaging boundary;
clean-Windows acceptance is still required.

## Moving records

Review portable exports before sharing them. Imported rules require confirmation;
evidence stays historical, and local permission is not imported. Preserve your
original record and read [Privacy](privacy.md) before exporting business content.

## Existing documentation links

The site preserves the /what-changed/ route and the old overview, investigation,
timeline, workspace, privacy, build, and status hash links. Legacy guides remain
available as pinned source links, while the new technical docs identify themselves
as development documentation rather than matching an approved installer.

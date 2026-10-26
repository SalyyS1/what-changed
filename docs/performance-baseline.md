# Performance baseline

**Evidence state: available engineering baseline; release gate remains open.**

The project keeps performance measurements and bounded targets in the source tree. The important product boundary is that idle monitoring should remain quiet, history queries stay paginated, and large histories must remain usable without storing secret values.

Run the focused commands in [Contributing](../CONTRIBUTING.md) before treating a local build as a candidate. Machine-specific benchmark output is intentionally kept out of this public preview.

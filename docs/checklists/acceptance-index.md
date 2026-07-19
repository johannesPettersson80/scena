# Acceptance index

Status: active release index

The release checkpoint combines focused milestone tests with these workspace
and packaging proofs:

- `m5-benchmarks.json`
- `m5-public-api-freeze.json`
- `cargo check --examples`
- `cargo publish --dry-run`

See the active M1-M6 checklists for feature-specific proof names. A checked
feature surface does not override a failed typed artifact, missing hardware
lane, provenance mismatch, or independent-review requirement.

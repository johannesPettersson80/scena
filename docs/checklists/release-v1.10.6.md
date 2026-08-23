# scena 1.10.6 release checklist

- [ ] Confirm the focused glTF-line and mixed CAD viewport contracts on the
      isolated remote builder.
- [ ] Build and install the extracted `.crate` with `--features agent`, then
      confirm the installed binary reports the agent feature.
- [ ] Complete one full native, WASM, browser, doctor, documentation, release-
      artifact, readiness, and publish rehearsal on the exact candidate.
- [ ] Collect every failed job before making one consolidated repair; do not
      make serial trial pushes.
- [ ] Create and push tag `v1.10.6` only after the frozen rehearsal is green.
- [ ] Verify tag CI, the Release workflow, GitHub Latest, crates.io `1.10.6`,
      the published archive's photo policy, and docs.rs `scena/1.10.6` on the
      same source commit.

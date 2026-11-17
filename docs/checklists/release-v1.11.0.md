# scena 1.11.0 release checklist

- [ ] Confirm formatting, clippy, xtask tests and `doctor --full` on the
      release candidate.
- [ ] Confirm the full CI matrix on the release candidate before tagging.
- [ ] Build and install the extracted `.crate` with `--features agent`, then
      confirm the installed binary reports the agent feature.
- [ ] Collect every failed job before making one consolidated repair; do not
      make serial trial pushes.
- [ ] Create and push tag `v1.11.0` only after the candidate is green.
- [ ] Verify tag CI, the Release workflow, GitHub Latest, crates.io `1.11.0`,
      the published archive's photo policy, and docs.rs `scena/1.11.0` on the
      same source commit.

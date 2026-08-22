# scena 1.10.3 release checklist

- [ ] Complete the isolated remote release validation matrix on the exact candidate commit.
- [ ] Do not push, tag, or dispatch a release until that full matrix is green. A focused
      proof or interrupted broad gate is not a substitute.
- [ ] For a failed matrix, collect every failure before one consolidated repair batch;
      do not make serial trial pushes.
- [ ] Push the validated release branch and create tag `v1.10.3` only after the full matrix passes.
- [ ] Verify the public workflow and published release version.

---
name: scena-git-github
description: Use when working with scena Git state, branches, commits, tags, GitHub issues, pull requests, workflow runs, release publication, or when the user asks to verify local state against GitHub.
---

# Scena Git And GitHub

## Core Rules

- Treat local git state, remote git state, GitHub workflow state, and published release state
  as separate facts.
- Do not commit, tag, push, merge, close issues, or delete branches unless the user asks for
  that action.
- Commit messages must describe the user-visible or code-level change in product terms.
  Do not use internal planning labels such as "Round A", "Round B", checklist numbers, or
  transient branch names unless the user explicitly asks for that wording. Before running
  `git commit`, sanity-check that the subject would make sense to a maintainer reading
  project history without the current chat context.
- Before changing files, check the current branch and dirty tree when the task involves
  commits, branches, release work, or crash recovery.
- Never discard dirty files you did not create unless the user explicitly asks and the exact
  paths have been compared against the target remote state.
- If no GitHub remote exists yet, say that GitHub proof is unavailable and continue with
  local git evidence.
- Treat the Hetzner build host as separate execution state. When reporting proof from
  `scena-builder`, include both the local/remote GitHub evidence that matters and the remote
  builder checkout state.

## Standard Evidence

Use the narrowest evidence that answers the task:

```bash
git status --short --branch
git log --oneline --decorate -5
git remote -v
git rev-parse HEAD
git ls-remote --heads origin
```

For GitHub state, prefer `gh` after confirming the repository owner/name:

```bash
gh repo view --json nameWithOwner,defaultBranchRef
gh issue view <id> --json number,title,state,url,body
gh pr view <id> --json number,title,state,mergeStateStatus,url,headRefName,baseRefName
gh run list --limit 10
```

For a failed run, collect the complete exact-SHA evidence before editing:

```bash
scripts/collect_ci_failure_evidence.sh <run-id>
```

Classify every failed job as product, harness, environment, policy, or provenance, then
batch all known fixes into one release-candidate push. After two remedies reproduce the same
signature, the investigation circuit breaker forbids a third push until a smaller probe
distinguishes the cause. Do not rerun already-passing jobs merely to refresh timestamps.

## Issue And PR Workflow

1. Fetch the live issue or PR before accepting its summary.
2. Verify the claim in the current checkout before patching.
3. Keep unrelated dirty files untouched.
4. Use `scena-remote-builder` to run the validation ladder on `scena-builder`: focused
   proof first, scoped cargo/doctor gates for the touched surface, and full release gates
   only when the task is release-ready or explicitly asks for them.
5. If asked to push or merge, verify the remote branch and monitor GitHub checks until the
   deciding run has completed.
6. If asked to close an issue, leave a concise comment with the fix commit, version or
   release if applicable, and verification evidence.

## Remote Builder Evidence

Use `scena-remote-builder` before remote gates. For git-sensitive work, capture:

```bash
ssh scena-builder 'git -C "$HOME/.cache/codex-worktrees/scena-<task-slug>" status --short --branch'
ssh scena-builder 'git -C "$HOME/.cache/codex-worktrees/scena-<task-slug>" log --oneline --decorate -1'
```

Do not confuse the remote builder checkout with GitHub branch state or the local checkout.
For commit-only requests after a multi-step cleanup, do not invent new full-suite runs just
to make the commit feel safer. Report the focused/scoped evidence already collected for the
diff, and run only missing gates that match files changed since that evidence.

## Release Follow-Through

Before the release-candidate push or tag, inspect repository topology and workflow triggers
instead of assuming a branch push starts validation:

```bash
git fetch origin --prune
git log --oneline --decorate origin/main..HEAD
git log --oneline --decorate HEAD..origin/main
git merge-base origin/main HEAD
sed -n '1,45p' .github/workflows/ci.yml
sed -n '1,45p' .github/workflows/release.yml
```

Record the intended release branch, frozen SHA, tag target, and which event starts CI and
Release. If the default branch is behind the release train, preserve the release branch
unless the user explicitly authorizes changing topology; report the condition instead of
deleting the only public release history.

After tagging, monitor every triggered run by run ID. Prefer concise status/job polling over
a high-volume `gh run watch` stream. Wait for all jobs to become terminal before editing; on
failure, collect the complete run and batch remedies. Do not start a duplicate matrix while
the deciding run is healthy.

When a release is requested, release work is incomplete until all requested layers are true:

- local version/changelog/docs state is correct,
- the release commit is on the intended remote branch,
- the tag exists on GitHub,
- the release workflow completed successfully,
- the GitHub release object is published and marked latest when that is the expected state,
- the tag dereferences to the frozen tested SHA, and
- crates.io reports the requested version when crate publication is part of the workflow.

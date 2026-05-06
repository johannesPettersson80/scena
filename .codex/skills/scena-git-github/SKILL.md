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
- Before changing files, check the current branch and dirty tree when the task involves
  commits, branches, release work, or crash recovery.
- Never discard dirty files you did not create unless the user explicitly asks and the exact
  paths have been compared against the target remote state.
- If no GitHub remote exists yet, say that GitHub proof is unavailable and continue with
  local git evidence.

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

## Issue And PR Workflow

1. Fetch the live issue or PR before accepting its summary.
2. Verify the claim in the current checkout before patching.
3. Keep unrelated dirty files untouched.
4. Run the required local gates and any feature-specific proof.
5. If asked to push or merge, verify the remote branch and monitor GitHub checks until the
   deciding run has completed.
6. If asked to close an issue, leave a concise comment with the fix commit, version or
   release if applicable, and verification evidence.

## Release Follow-Through

When a release is requested, release work is incomplete until all requested layers are true:

- local version/changelog/docs state is correct,
- the release commit is on the intended remote branch,
- the tag exists on GitHub,
- the release workflow completed successfully,
- the GitHub release object is published and marked latest when that is the expected state.

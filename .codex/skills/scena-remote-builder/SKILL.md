---
name: scena-remote-builder
description: Use when compiling scena, running cargo fmt/clippy/test/doc/doctor/publish dry-run, synchronizing local work to the Hetzner CPU builder, or reporting remote build/test proof.
---

# Scena Remote Builder

## Builder Contract

Use the Hetzner CPU builder for heavy Rust compilation and test gates.

- SSH alias: `scena-builder`
- Remote repo path: `/home/johannes/projects/scena`
- Remote user: `johannes`
- Purpose: cargo compile, fmt, clippy, tests, docs, doctor, publish dry-run, and CPU/headless
  proof.
- Not purpose: real GPU/WebGPU/WebGL2 proof. Use a real GPU machine for GPU-specific visual
  validation.

Do not store private SSH key material, cloud credentials, or provider tokens in the repo.

## Sync Rule

Before a remote gate, make the remote checkout match the exact work being validated.

For a clean branch already pushed to GitHub:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && git fetch origin && git checkout <branch> && git pull --ff-only'
```

For local uncommitted work:

1. Check local and remote status.
2. If the remote has unrelated dirty changes, stop and report them.
3. Mirror the local working tree to the remote repo, keeping the remote `.git` and build
   cache intact:

```bash
git status --short --branch
ssh scena-builder 'git -C "$HOME/projects/scena" status --short --branch'
rsync -az --delete --exclude .git --exclude target ./ scena-builder:~/projects/scena/
```

After syncing, remote `git status --short --branch` should show the same relevant working
tree changes as the local checkout.

## Gate Commands

Run gates through SSH from the local machine:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo check --all-targets'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo build --all-targets'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo fmt --check'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo clippy --all-targets -- -D warnings'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --full'
```

Release-ready handoff also requires:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo publish --dry-run'
```

## Reporting Proof

Report:

- command run
- remote host alias and repo path
- pass/fail status and timing when available
- remote git status and HEAD when relevant
- any gate not run and the concrete reason

If a command fails due to environment drift, fix the builder when safe and rerun. If the
failure is in project code, patch the code and rerun the focused failing gate first.

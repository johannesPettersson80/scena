#!/usr/bin/env node
"use strict";

const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const childProcess = require("child_process");

function fail(message) {
  throw new Error(message);
}

function sha256(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function main(argv) {
  if (argv.length !== 2) {
    fail("usage: node scripts/promote_q01_waterbottle_reference.cjs <candidate-dir> <approval.json>");
  }
  const repoRoot = childProcess.execFileSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
  }).trim();
  const dirty = childProcess.execFileSync("git", ["status", "--porcelain", "--untracked-files=normal"], {
    cwd: repoRoot,
    encoding: "utf8",
  }).trim();
  if (dirty) fail("reference promotion requires a clean checkout");

  const candidateDir = path.resolve(repoRoot, argv[0]);
  const approvedRoot = path.resolve(repoRoot, "target/reference-candidates");
  if (!candidateDir.startsWith(`${approvedRoot}${path.sep}`)) {
    fail("candidate directory must be under target/reference-candidates");
  }
  const approvalPath = path.resolve(repoRoot, argv[1]);
  const manifest = readJson(path.join(candidateDir, "candidate.json"));
  const approval = readJson(approvalPath);
  const referencePath = path.join(
    repoRoot,
    "tests/assets/gltf/khronos/WaterBottle/reference_cpu_256.png",
  );
  const anchorPath = path.join(
    repoRoot,
    "tests/assets/gltf/khronos/WaterBottle/reference_blender_cycles_512.png",
  );
  const candidatePath = path.join(candidateDir, "candidate.png");
  const diffPath = path.join(candidateDir, "diff-heatmap.png");

  if (
    manifest.schema !== "scena.q11.reference_candidate.v1"
      || manifest.status !== "review-required"
      || manifest.release_evidence !== false
      || manifest.candidate_only !== true
      || manifest.approval !== null
  ) fail("candidate manifest is not a non-certifying Q11 candidate");
  if (
    approval.schema !== "scena.q11.reference_approval.v1"
      || approval.status !== "approved"
      || typeof approval.reviewer !== "string"
      || approval.reviewer.trim().length < 3
      || typeof approval.reviewed_at !== "string"
      || approval.reviewed_generator_commit !== manifest.generator_commit
      || approval.candidate_sha256 !== manifest.candidate.sha256
      || approval.before_reference_sha256 !== manifest.current_reference.sha256
      || approval.external_anchor_sha256 !== manifest.external_anchor.sha256
      || approval.diff_sha256 !== manifest.diff.sha256
      || approval.external_anchor_reviewed !== true
      || approval.before_after_diff_reviewed !== true
      || approval.tolerance_change_approved !== false
  ) fail("approval does not independently bind the candidate, prior reference, diff, anchor, and named human reviewer");
  if (
    sha256(candidatePath) !== manifest.candidate.sha256
      || sha256(diffPath) !== manifest.diff.sha256
      || sha256(referencePath) !== manifest.current_reference.sha256
      || sha256(anchorPath) !== manifest.external_anchor.sha256
  ) fail("candidate, diff, current reference, or external anchor checksum changed after review");

  fs.copyFileSync(candidatePath, referencePath);
  process.stdout.write(
    "reference promoted; update the pinned SHA in metadata/tests/doctor, retain the approval, and collect fresh Linux/macOS/Windows Q11 evidence without loosening thresholds\n",
  );
}

main(process.argv.slice(2));

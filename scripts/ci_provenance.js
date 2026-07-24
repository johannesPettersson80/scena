"use strict";

const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const EXPECTED_REPOSITORY = "johannesPettersson80/scena";
const PROVENANCE_FILENAME = "ci-provenance.json";
const HEX40 = /^[0-9a-f]{40}$/;

function compareUtf8(left, right) {
  return Buffer.compare(Buffer.from(left, "utf8"), Buffer.from(right, "utf8"));
}

function required(env, name) {
  const value = env[name];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`CI provenance requires non-blank ${name}`);
  }
  return value.trim();
}

function canonicalArtifactFiles(root) {
  const canonicalRoot = path.resolve(root);
  const files = [];
  function visit(directory) {
    for (const entry of fs
      .readdirSync(directory, { withFileTypes: true })
      .sort((left, right) => compareUtf8(left.name, right.name))) {
      const absolute = path.join(directory, entry.name);
      const relative = path.relative(canonicalRoot, absolute).split(path.sep).join("/");
      if (entry.isSymbolicLink()) {
        throw new Error(`CI provenance refuses symbolic-link artifact ${relative}`);
      }
      if (entry.isDirectory()) {
        visit(absolute);
      } else if (entry.isFile() && relative !== PROVENANCE_FILENAME) {
        files.push({
          path: relative,
          sha256: crypto.createHash("sha256").update(fs.readFileSync(absolute)).digest("hex"),
        });
      }
    }
  }
  visit(canonicalRoot);
  return files.sort((left, right) => compareUtf8(left.path, right.path));
}

function digestEntries(entries) {
  const digest = crypto.createHash("sha256");
  for (const entry of entries) {
    digest.update(entry.path, "utf8");
    digest.update(Buffer.from([0]));
    digest.update(entry.sha256, "ascii");
    digest.update("\n", "ascii");
  }
  return digest.digest("hex");
}

function canonicalArtifactTreeDigest(root) {
  return digestEntries(canonicalArtifactFiles(root));
}

function buildCiProvenance(root, env = process.env, timestamp = Math.floor(Date.now() / 1000)) {
  if (env.GITHUB_ACTIONS !== "true") {
    throw new Error("CI provenance can only be generated in a trusted GitHub Actions context");
  }
  const repository = required(env, "GITHUB_REPOSITORY");
  if (repository !== EXPECTED_REPOSITORY) {
    throw new Error(
      `CI provenance repository must be ${EXPECTED_REPOSITORY}, got ${repository}`,
    );
  }
  const workflowRef = required(env, "GITHUB_WORKFLOW_REF");
  const workflowPrefix = `${repository}/.github/workflows/`;
  if (
    !workflowRef.startsWith(workflowPrefix) ||
    !["ci.yml", "release.yml"].some((workflow) =>
      workflowRef.startsWith(`${workflowPrefix}${workflow}@`),
    )
  ) {
    throw new Error(`CI provenance workflow_ref is not an approved scena workflow: ${workflowRef}`);
  }
  const workflowSha = required(env, "GITHUB_WORKFLOW_SHA").toLowerCase();
  const sourceCommit = required(env, "GITHUB_SHA").toLowerCase();
  if (!HEX40.test(workflowSha) || !HEX40.test(sourceCommit)) {
    throw new Error("CI provenance workflow and source commits must be exact lowercase 40-hex SHAs");
  }
  const runId = required(env, "GITHUB_RUN_ID");
  const runAttemptText = required(env, "GITHUB_RUN_ATTEMPT");
  if (!/^[1-9][0-9]*$/.test(runId) || !/^[1-9][0-9]*$/.test(runAttemptText)) {
    throw new Error("CI provenance run id and attempt must be positive decimal integers");
  }
  const files = canonicalArtifactFiles(root);
  if (files.length === 0) {
    throw new Error("CI provenance requires at least one downloaded artifact file");
  }
  return {
    schema: "scena.ci_provenance.v1",
    repository,
    workflow_ref: workflowRef,
    workflow_sha: workflowSha,
    ref: required(env, "GITHUB_REF"),
    run_id: runId,
    run_attempt: Number(runAttemptText),
    job: required(env, "GITHUB_JOB"),
    source_commit: sourceCommit,
    artifact_digest: digestEntries(files),
    artifact_file_count: files.length,
    issuer: "https://token.actions.githubusercontent.com",
    generated_at_unix_seconds: timestamp,
    release_evidence: false,
    release_rejection_codes: ["CI_ATTESTATION_NOT_YET_VERIFIED"],
    attestation: {
      predicate_type: "https://slsa.dev/provenance/v1",
      verification_status: "pending",
    },
  };
}

function main(args) {
  if (args.length !== 2) {
    throw new Error("usage: node scripts/ci_provenance.js <artifact-root> <output-json>");
  }
  const root = path.resolve(args[0]);
  const output = path.resolve(args[1]);
  if (path.dirname(output) !== root || path.basename(output) !== PROVENANCE_FILENAME) {
    throw new Error(`CI provenance output must be ${path.join(root, PROVENANCE_FILENAME)}`);
  }
  const provenance = buildCiProvenance(root);
  fs.writeFileSync(output, `${JSON.stringify(provenance, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`${output}\n`);
}

if (require.main === module) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  buildCiProvenance,
  canonicalArtifactFiles,
  canonicalArtifactTreeDigest,
};

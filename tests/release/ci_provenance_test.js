const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const {
  buildCiProvenance,
  canonicalArtifactFiles,
  canonicalArtifactTreeDigest,
} = require("../../scripts/ci_provenance.js");

const root = fs.mkdtempSync(path.join(os.tmpdir(), "scena-ci-provenance-"));
fs.mkdirSync(path.join(root, "lane"), { recursive: true });
fs.writeFileSync(path.join(root, "lane", "result.json"), '{"status":"passed"}\n');
fs.mkdirSync(path.join(root, "lane", "examples"), { recursive: true });
fs.writeFileSync(path.join(root, "lane", "examples", "camera_framing.ppm"), "ppm\n");
fs.writeFileSync(
  path.join(root, "lane", "examples", "camera_framing_frame_bounds.json"),
  "{}\n",
);
const canonicalPaths = canonicalArtifactFiles(root).map((entry) => entry.path);
assert.ok(
  canonicalPaths.indexOf("lane/examples/camera_framing.ppm") <
    canonicalPaths.indexOf("lane/examples/camera_framing_frame_bounds.json"),
  "CI provenance paths must use the Rust verifier's raw UTF-8 lexical order",
);
const commit = "0123456789abcdef0123456789abcdef01234567";
const workflowSha = "89abcdef0123456789abcdef0123456789abcdef";
const env = {
  GITHUB_ACTIONS: "true",
  GITHUB_REPOSITORY: "johannesPettersson80/scena",
  GITHUB_WORKFLOW_REF:
    "johannesPettersson80/scena/.github/workflows/release.yml@refs/tags/v1.9.0",
  GITHUB_WORKFLOW_SHA: workflowSha,
  GITHUB_REF: "refs/tags/v1.9.0",
  GITHUB_RUN_ID: "123456789",
  GITHUB_RUN_ATTEMPT: "2",
  GITHUB_JOB: "publish",
  GITHUB_SHA: commit,
};

const provenance = buildCiProvenance(root, env, 1_800_000_000);
assert.strictEqual(provenance.schema, "scena.ci_provenance.v1");
assert.strictEqual(provenance.repository, env.GITHUB_REPOSITORY);
assert.strictEqual(provenance.workflow_ref, env.GITHUB_WORKFLOW_REF);
assert.strictEqual(provenance.workflow_sha, workflowSha);
assert.strictEqual(provenance.ref, env.GITHUB_REF);
assert.strictEqual(provenance.run_id, env.GITHUB_RUN_ID);
assert.strictEqual(provenance.run_attempt, 2);
assert.strictEqual(provenance.job, env.GITHUB_JOB);
assert.strictEqual(provenance.source_commit, commit);
assert.strictEqual(provenance.issuer, "https://token.actions.githubusercontent.com");
assert.strictEqual(provenance.artifact_digest, canonicalArtifactTreeDigest(root));
assert.strictEqual(provenance.release_evidence, false);
assert.deepStrictEqual(provenance.attestation, {
  predicate_type: "https://slsa.dev/provenance/v1",
  verification_status: "pending",
});
assert.deepStrictEqual(provenance.release_rejection_codes, [
  "CI_ATTESTATION_NOT_YET_VERIFIED",
]);

const beforeTamper = provenance.artifact_digest;
fs.writeFileSync(path.join(root, "lane", "result.json"), '{"status":"tampered"}\n');
assert.notStrictEqual(canonicalArtifactTreeDigest(root), beforeTamper);

for (const [label, field, value, pattern] of [
  ["untrusted context", "GITHUB_ACTIONS", "false", /trusted GitHub Actions context/],
  ["wrong repository", "GITHUB_REPOSITORY", "attacker/fork", /repository/],
  ["missing job", "GITHUB_JOB", "", /GITHUB_JOB/],
  ["wrong ref", "GITHUB_REF", "", /GITHUB_REF/],
]) {
  assert.throws(
    () => buildCiProvenance(root, { ...env, [field]: value }),
    pattern,
    `${label} mutation must be rejected`,
  );
}

const replayedRun = buildCiProvenance(root, { ...env, GITHUB_RUN_ID: "123456788" });
assert.notStrictEqual(
  replayedRun.run_id,
  provenance.run_id,
  "replayed run identity must remain distinct for the trusted-context consumer",
);

fs.rmSync(root, { recursive: true, force: true });
console.log("CI-issued release provenance manifest: pass");

const assert = require("assert");
const crypto = require("crypto");
const fs = require("fs");
const os = require("os");
const path = require("path");

const {
  attachReleaseArtifactProvenance,
} = require("./release_artifact_provenance.js");

const root = fs.mkdtempSync(path.join(os.tmpdir(), "scena-release-provenance-"));
const source = "producer.js";
fs.writeFileSync(path.join(root, source), "release producer source\n");
const commit = "0123456789abcdef0123456789abcdef01234567";
const before = Math.floor(Date.now() / 1000);
const artifact = attachReleaseArtifactProvenance(
  { status: "passed" },
  {
    root,
    schema: "scena.release.provenance.test.v1",
    producer: "node tests/release/release_artifact_provenance_test.js",
    sourcePaths: [source],
    env: { SCENA_RELEASE_COMMIT: commit },
  },
);
const after = Math.floor(Date.now() / 1000);

assert.strictEqual(artifact.schema, "scena.release.provenance.test.v1");
assert.strictEqual(
  artifact.producer,
  "node tests/release/release_artifact_provenance_test.js",
);
assert.strictEqual(artifact.commit_sha, commit);
assert.ok(artifact.timestamp_unix_seconds >= before);
assert.ok(artifact.timestamp_unix_seconds <= after);
assert.deepStrictEqual(artifact.source_checksums, [
  {
    path: source,
    sha256: crypto
      .createHash("sha256")
      .update(fs.readFileSync(path.join(root, source)))
      .digest("hex"),
  },
]);
assert.throws(
  () =>
    attachReleaseArtifactProvenance(
      {},
      {
        root,
        schema: "scena.release.provenance.test.v1",
        producer: "test",
        sourcePaths: [source],
        env: { SCENA_RELEASE_COMMIT: "local-checkout" },
      },
    ),
  /exactly 40 hexadecimal/,
);

fs.rmSync(root, { recursive: true, force: true });
console.log("release artifact provenance contract: pass");

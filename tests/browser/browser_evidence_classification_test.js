"use strict";

const assert = require("assert");
const {
  classifyBrowserEvidence,
} = require("./required_gpu_parity.js");

const smoke = classifyBrowserEvidence({
  required: false,
  selectedBackends: ["webgl2"],
  evaluations: [{ backend: "webgl2", status: "diagnostic", pixel_parity: null }],
});
assert.deepStrictEqual(smoke, {
  proof_class: "renderer-smoke",
  release_evidence: false,
  parity_claim: "not-claimed",
  parity_scope: [],
});

const diagnosticPixels = classifyBrowserEvidence({
  required: false,
  selectedBackends: ["webgpu"],
  evaluations: [{ backend: "webgpu", status: "diagnostic", pixel_parity: { status: "passed" } }],
});
assert.strictEqual(
  diagnosticPixels.proof_class,
  "renderer-conformance-with-diagnostic-webgpu-pixel-diff",
);
assert.strictEqual(diagnosticPixels.release_evidence, false);
assert.deepStrictEqual(diagnosticPixels.parity_scope, ["webgpu:m6-identical-unlit-triangle-v1"]);

const requiredPixels = classifyBrowserEvidence({
  required: true,
  selectedBackends: ["webgpu", "webgl2"],
  evaluations: [
    { backend: "webgpu", status: "passed", pixel_parity: { status: "passed" } },
    { backend: "webgl2", status: "passed", pixel_parity: null },
  ],
});
assert.strictEqual(
  requiredPixels.proof_class,
  "renderer-smoke-with-required-webgpu-full-frame-parity",
);
assert.strictEqual(requiredPixels.release_evidence, true);
assert.strictEqual(requiredPixels.parity_claim, "full-frame-reference-diff");

const missingPixels = classifyBrowserEvidence({
  required: true,
  selectedBackends: ["webgpu"],
  evaluations: [{ backend: "webgpu", status: "failed", pixel_parity: null }],
});
assert.strictEqual(missingPixels.release_evidence, false);
assert.strictEqual(missingPixels.proof_class, "required-webgpu-parity-failed");

console.log("browser evidence classification: pass");

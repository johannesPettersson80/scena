#!/usr/bin/env node

"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const {
  attachReleaseArtifactProvenance,
} = require("../tests/release/release_artifact_provenance.js");
const {
  cropRoundEMaterialTiles,
  evaluateRoundEMaterialTiles,
  parseThresholds,
} = require("./round_e_material_evaluator.cjs");

const root = process.cwd();
const manifestPath = path.join(
  root,
  "target/gate-artifacts/round-e-cpu-material-proof/live-cpu-frame.json",
);
const resultPath = path.join(root, "target/gate-artifacts/round-e-cpu-material-proof.json");
const thresholdPath = path.join(
  root,
  "tests/visual/references/round_e_material_thresholds.toml",
);
const expectedProducer =
  "cargo test --test examples_visual_proof q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact";

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function fail(message) {
  throw new Error(`Q02 CPU material proof: ${message}`);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
if (manifest.schema !== "scena.q02.live_cpu_material_frame.v1") fail("unexpected manifest schema");
if (manifest.proof_class !== "live-cpu-rendered-round-e-material-showcase") {
  fail("manifest does not prove live CPU renderer invocation");
}
if (manifest.producer !== expectedProducer) fail("producer command drifted");
if (!/^[0-9a-f]{40}$/.test(manifest.commit_sha || "")) fail("commit SHA is missing or invalid");

const frame = manifest.frame || {};
if (frame.pixel_format !== "rgba8-srgb-top-to-bottom") fail("frame pixel format drifted");
const rgba = Buffer.from(frame.rgba8_base64 || "", "base64");
if (rgba.length !== frame.width * frame.height * 4) fail("frame RGBA byte count is invalid");
if (sha256(rgba) !== frame.rgba8_sha256) fail("frame RGBA checksum mismatch");
const pngPath = path.join(root, "target/gate-artifacts", frame.png_path || "");
if (!fs.existsSync(pngPath)) fail("live frame PNG artifact is missing");
if (sha256(fs.readFileSync(pngPath)) !== frame.png_sha256) fail("live frame PNG checksum mismatch");
const tiles = cropRoundEMaterialTiles({
  width: frame.width,
  height: frame.height,
  data: rgba,
});

const thresholds = parseThresholds(fs.readFileSync(thresholdPath, "utf8"));
const evaluation = evaluateRoundEMaterialTiles({
  surface: "live-cpu-headless",
  tiles,
  thresholds,
  requireReferenceDelta: false,
});
if (manifest.commit_sha !== process.env.SCENA_RELEASE_COMMIT && process.env.SCENA_RELEASE_COMMIT) {
  fail("live frame commit does not match SCENA_RELEASE_COMMIT");
}
const artifact = attachReleaseArtifactProvenance({
  proof_class: "live-cpu-round-e-shared-threshold-evaluation",
  status: evaluation.status === "pass" ? "passed" : "failed",
  renderer_producer: manifest.producer,
  threshold_evaluator: {
    proof_class: evaluation.proof_class,
    evaluator_version: evaluation.evaluator_version,
    surface: evaluation.surface,
  },
  live_renderer: manifest.renderer,
  live_frame: {
    path: path.relative(root, pngPath),
    sha256: frame.png_sha256,
    width: frame.width,
    height: frame.height,
  },
  thresholds,
  per_material: evaluation.per_material,
  neighbor_pairs: evaluation.neighbor_pairs,
  errors: evaluation.errors,
}, {
  root,
  schema: "scena.q02.round_e_cpu_material_proof.v1",
  producer: "node scripts/evaluate_round_e_cpu_materials.cjs",
  sourcePaths: [
    "scripts/evaluate_round_e_cpu_materials.cjs",
    "scripts/round_e_material_evaluator.cjs",
    "tests/examples_visual_proof.rs",
    "tests/visual/references/round_e_material_thresholds.toml",
    "tests/release/release_artifact_provenance.js",
  ],
});
if (artifact.commit_sha !== manifest.commit_sha) fail("evaluator and live frame commits differ");
fs.mkdirSync(path.dirname(resultPath), { recursive: true });
fs.writeFileSync(resultPath, `${JSON.stringify(artifact, null, 2)}\n`);
console.log(`wrote ${path.relative(root, resultPath)} (${artifact.status})`);
if (artifact.status !== "passed") {
  for (const error of artifact.errors) console.error(`round-e-cpu: ${error.code}: ${error.message}`);
  process.exit(1);
}

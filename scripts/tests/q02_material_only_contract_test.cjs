#!/usr/bin/env node

"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");

function read(relative) {
  return fs.readFileSync(relative, "utf8");
}

const packageJson = read("package.json");
const browserProbe = read("tests/browser/m6_rust_wasm_renderer_probe.js");
const laneArtifacts = read("crates/xtask/src/app/release/lane_artifacts.rs");
const doctor = read("crates/xtask/src/app/doctor_visual_release/round_e_materials.rs");

assert.match(
  packageJson,
  /"browser:q02-materials":\s*"node tests\/browser\/m6_rust_wasm_renderer_probe\.js --q02-material-only"/,
);
assert.match(browserProbe, /process\.argv\.includes\("--q02-material-only"\)/);
assert.match(browserProbe, /Q02 material-only proof requires exactly the webgpu backend/);
assert.match(browserProbe, /if \(materialOnly\) \{[\s\S]*return;[\s\S]*\}/);
assert.match(laneArtifacts, /"npm run browser:q02-materials"/);
assert.match(doctor, /npm run browser:q02-materials/);
for (const workflow of [".github/workflows/ci.yml", ".github/workflows/release.yml"]) {
  assert.match(read(workflow), /linux-webgpu-chromium npm run browser:q02-materials/);
}

console.log("Q02 dedicated WebGPU material-only lane contract passed");

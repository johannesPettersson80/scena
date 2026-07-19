const assert = require("assert");
const { validateOutputToggleResult } = require("./pf01_output_toggle_validation.js");

function phase(id, hash, resources) {
  return {
    id,
    fnv1a64: hash,
    nonblack: 10,
    resources_before_render: resources,
    resources_after_render: resources,
  };
}

function validResult() {
  const baseline = [8, 19, 3, 7, 6, 6];
  const enabled = [9, 22, 6, 20, 9, 18];
  return {
    backend: "webgpu",
    phases: {
      off: phase("off", "off", baseline),
      bloom_only: phase("bloom_only", "bloom", enabled),
      fxaa_only: phase("fxaa_only", "fxaa", enabled),
      on: phase("on", "combined", enabled),
      off_again: phase("off_again", "off", baseline),
    },
  };
}

assert.doesNotThrow(() => validateOutputToggleResult(validResult()));

const lostBloom = validResult();
lostBloom.phases.on.fnv1a64 = lostBloom.phases.fxaa_only.fnv1a64;
assert.throws(
  () => validateOutputToggleResult(lostBloom),
  /combined output is identical to FXAA-only/,
);

const lostFxaa = validResult();
lostFxaa.phases.on.fnv1a64 = lostFxaa.phases.bloom_only.fnv1a64;
assert.throws(
  () => validateOutputToggleResult(lostFxaa),
  /combined output is identical to bloom-only/,
);

const inertBloom = validResult();
inertBloom.phases.bloom_only.fnv1a64 = inertBloom.phases.off.fnv1a64;
assert.throws(
  () => validateOutputToggleResult(inertBloom),
  /bloom-only output is identical to baseline/,
);

const inertFxaa = validResult();
inertFxaa.phases.fxaa_only.fnv1a64 = inertFxaa.phases.off.fnv1a64;
assert.throws(
  () => validateOutputToggleResult(inertFxaa),
  /FXAA-only output is identical to baseline/,
);

const unstableResources = validResult();
unstableResources.phases.on.resources_after_render = [99];
assert.throws(
  () => validateOutputToggleResult(unstableResources),
  /render changed its prepared resource shape/,
);

console.log("PF01 output-toggle validator: pass");

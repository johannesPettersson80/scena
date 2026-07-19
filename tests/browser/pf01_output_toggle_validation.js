const PHASE_IDS = ["off", "bloom_only", "fxaa_only", "on", "off_again"];

function sameResourceShape(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function validateOutputToggleResult(result) {
  const backend = result?.backend || "unknown backend";
  const phases = result?.phases;
  if (!phases || typeof phases !== "object") {
    throw new Error(`${backend} output-toggle result has no phases`);
  }
  for (const id of PHASE_IDS) {
    const phase = phases[id];
    if (!phase || phase.id !== id) {
      throw new Error(`${backend} output-toggle result is missing phase ${id}`);
    }
    if (!Number.isInteger(phase.nonblack) || phase.nonblack <= 0) {
      throw new Error(`${backend} ${id} output is blank`);
    }
    if (typeof phase.fnv1a64 !== "string" || phase.fnv1a64.length === 0) {
      throw new Error(`${backend} ${id} output has no pixel hash`);
    }
    if (!sameResourceShape(phase.resources_before_render, phase.resources_after_render)) {
      throw new Error(`${backend} ${id} render changed its prepared resource shape`);
    }
  }

  const { off, bloom_only: bloomOnly, fxaa_only: fxaaOnly, on, off_again: offAgain } = phases;
  if (bloomOnly.fnv1a64 === off.fnv1a64) {
    throw new Error(`${backend} bloom-only output is identical to baseline`);
  }
  if (fxaaOnly.fnv1a64 === off.fnv1a64) {
    throw new Error(`${backend} FXAA-only output is identical to baseline`);
  }
  if (on.fnv1a64 === off.fnv1a64) {
    throw new Error(`${backend} combined output is identical to baseline`);
  }
  if (on.fnv1a64 === bloomOnly.fnv1a64) {
    throw new Error(`${backend} combined output is identical to bloom-only`);
  }
  if (on.fnv1a64 === fxaaOnly.fnv1a64) {
    throw new Error(`${backend} combined output is identical to FXAA-only`);
  }
  if (off.fnv1a64 !== offAgain.fnv1a64) {
    throw new Error(`${backend} off-again output is not deterministic`);
  }

  for (const phase of [bloomOnly, fxaaOnly, on]) {
    if (sameResourceShape(off.resources_before_render, phase.resources_before_render)) {
      throw new Error(`${backend} ${phase.id} did not prepare a distinct resource shape`);
    }
  }
  if (!sameResourceShape(off.resources_before_render, offAgain.resources_before_render)) {
    throw new Error(`${backend} disabling FXAA/bloom did not restore baseline resources`);
  }

  return result;
}

module.exports = { validateOutputToggleResult };

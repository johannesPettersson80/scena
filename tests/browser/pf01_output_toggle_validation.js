const PHASE_IDS = ["off", "bloom_only", "fxaa_only", "on", "off_again"];

function sameResourceShape(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function validateFxaaEffect(baseline, candidate, backend) {
  const left = baseline?.aa_edge_metrics;
  const right = candidate?.aa_edge_metrics;
  if (!left || !right) {
    throw new Error(`${backend} FXAA effect proof is missing edge metrics`);
  }
  if (right.intermediate_luma_pixels <= left.intermediate_luma_pixels) {
    throw new Error(`${backend} FXAA did not add intermediate-luma edge coverage`);
  }
  if (right.relative_hard_transitions >= left.relative_hard_transitions) {
    throw new Error(`${backend} FXAA did not reduce relative hard transitions`);
  }
  if (
    right.normalized_squared_edge_energy
      >= left.normalized_squared_edge_energy * 0.9
  ) {
    throw new Error(`${backend} FXAA did not materially reduce normalized edge energy`);
  }
  if (right.luma_range < left.luma_range * 0.9) {
    throw new Error(`${backend} FXAA collapsed global contrast`);
  }
  if (candidate.nonblack > baseline.nonblack * 1.25) {
    throw new Error(`${backend} FXAA spread coverage beyond the edge-local bound`);
  }
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
  validateFxaaEffect(off, fxaaOnly, backend);

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

module.exports = { validateFxaaEffect, validateOutputToggleResult };

#!/usr/bin/env node

const assert = require("node:assert/strict");
const {
  evaluateRoundEMaterialTiles,
} = require("../round_e_material_evaluator.cjs");

const WIDTH = 128;
const HEIGHT = 128;

function image(background = [147, 150, 156, 255]) {
  const data = new Uint8Array(WIDTH * HEIGHT * 4);
  for (let offset = 0; offset < data.length; offset += 4) {
    data.set(background, offset);
  }
  return { width: WIDTH, height: HEIGHT, data };
}

function clone(source) {
  return { width: source.width, height: source.height, data: source.data.slice() };
}

function fillRect(target, x0, y0, x1, y1, color) {
  for (let y = y0; y < y1; y += 1) {
    for (let x = x0; x < x1; x += 1) {
      target.data.set(color, (y * target.width + x) * 4);
    }
  }
}

function subject(color) {
  const target = image();
  fillRect(target, 20, 20, 108, 108, [...color, 255]);
  return target;
}

function chrome() {
  const target = image();
  for (let y = 20; y < 108; y += 1) {
    for (let x = 20; x < 108; x += 1) {
      const value = x < 82 ? 25 : 255;
      target.data.set([value, value, value, 255], (y * WIDTH + x) * 4);
    }
  }
  return target;
}

function brushedSteel() {
  const target = subject([78, 86, 96]);
  fillRect(target, 24, 59, 104, 67, [250, 250, 250, 255]);
  return target;
}

function textured(base, amplitude) {
  const target = image();
  for (let y = 20; y < 108; y += 1) {
    for (let x = 20; x < 108; x += 1) {
      const offset = ((x + y) % 8) < 4 ? amplitude : -amplitude;
      target.data.set(
        [base[0] + offset, base[1] + offset, base[2] + offset, 255],
        (y * WIDTH + x) * 4,
      );
    }
  }
  return target;
}

function clearGlass() {
  const target = image([190, 190, 190, 255]);
  fillRect(target, 18, 18, 110, 110, [165, 180, 190, 255]);
  fillRect(target, 78, 40, 88, 88, [35, 35, 35, 255]);
  return target;
}

function frostedGlass() {
  const target = image([190, 190, 190, 255]);
  fillRect(target, 18, 18, 110, 110, [110, 170, 210, 255]);
  return target;
}

function goodTiles() {
  const plastic = subject([65, 95, 190]);
  const clearcoat = clone(plastic);
  fillRect(clearcoat, 24, 24, 104, 52, [245, 245, 255, 255]);
  const satin = subject([85, 20, 80]);
  fillRect(satin, 24, 24, 104, 44, [245, 120, 235, 255]);
  return {
    matte: subject([40, 70, 150]),
    plastic,
    metal: subject([185, 185, 190]),
    rough_metal: textured([105, 110, 120], 18),
    chrome: chrome(),
    brushed_steel: brushedSteel(),
    clearcoat_plastic: clearcoat,
    satin,
    leather: textured([150, 72, 28], 18),
    clear_glass: clearGlass(),
    frosted_glass: frostedGlass(),
    rubber: textured([38, 42, 48], 12),
  };
}

const thresholds = {
  matte: { delta_e2000_max: 1 },
  plastic: { delta_e2000_max: 1 },
  metal: { delta_e2000_max: 1 },
  rough_metal: { delta_e2000_max: 1 },
  chrome: {
    delta_e2000_max: 1,
    specular_dynamic_range: 2,
    dark_reflection_luminance_p05_max: 85,
    bright_reflection_luminance_p99_min: 230,
  },
  brushed_steel: { delta_e2000_max: 1, anisotropy_aspect_ratio_ibl: 2 },
  clearcoat_plastic: { delta_e2000_max: 1, clearcoat_lobe_delta: 0.05 },
  clear_glass: { delta_e2000_max: 1, refraction_offset_min: 4 },
  frosted_glass: { delta_e2000_max: 1, high_frequency_contrast_reduction_min: 0.35 },
  leather: { delta_e2000_max: 1, texture_variance_min: 0.02, local_texture_variance_min: 0.015 },
  rubber: { delta_e2000_max: 1, roughness_variance_min: 0.02, local_texture_variance_min: 0.015 },
  satin: { delta_e2000_max: 1, sheen_width_min: 0.20 },
  global: { neighbor_delta_e2000_min: 6, reference_delta_e2000_max: 1 },
  "live_cpu_headless.brushed_steel": { anisotropy_aspect_ratio_ibl: 1.30 },
  "live_cpu_headless.clearcoat_plastic": { clearcoat_lobe_delta: 0.007 },
  "live_cpu_headless.leather": { local_texture_variance_min: 0.006 },
  "live_cpu_headless.rubber": { local_texture_variance_min: 0.006 },
  "live_cpu_headless.satin": { sheen_width_min: 0.015 },
  "live_webgl2_chromium.leather": { local_texture_variance_min: 0.010 },
  "live_webgpu_chromium.brushed_steel": { anisotropy_aspect_ratio_ibl: 1.15 },
  "live_webgpu_chromium.leather": { local_texture_variance_min: 0.003 },
  "live_webgpu_chromium.rubber": { local_texture_variance_min: 0.004 },
  "live_webgpu_chromium.satin": { sheen_width_min: 0.08 },
};

function evaluate(tiles, surface = "synthetic-contract") {
  return evaluateRoundEMaterialTiles({
    surface,
    tiles,
    references: goodTiles(),
    thresholds,
    requireReferenceDelta: true,
  });
}

function assertFailsOnlyIntendedFamily(name, mutate, expectedCode) {
  for (const surface of [
    "synthetic-contract",
    "live-cpu-headless",
    "live-webgl2-chromium",
    "live-webgpu-chromium",
  ]) {
    const tiles = goodTiles();
    mutate(tiles);
    const result = evaluate(tiles, surface);
    assert.equal(result.status, "fail", `${surface} ${name} must be rejected`);
    assert.ok(
      result.errors.some((error) => error.code === expectedCode),
      `${surface} ${name} must fail ${expectedCode}; got ${JSON.stringify(result.errors)}`,
    );
  }
}

const good = evaluate(goodTiles());
if (good.status !== "pass") console.error(JSON.stringify(good, null, 2));
assert.equal(good.status, "pass", JSON.stringify(good.errors));
const cpuGood = evaluate(goodTiles(), "live-cpu-headless");
assert.equal(cpuGood.status, "pass", JSON.stringify(cpuGood.errors));
const webglGood = evaluate(goodTiles(), "live-webgl2-chromium");
assert.equal(webglGood.status, "pass", JSON.stringify(webglGood.errors));
const webgpuGood = evaluate(goodTiles(), "live-webgpu-chromium");
assert.equal(webgpuGood.status, "pass", JSON.stringify(webgpuGood.errors));

assertFailsOnlyIntendedFamily(
  "flat chrome",
  (tiles) => { tiles.chrome = subject([128, 128, 128]); },
  "chrome_specular_dynamic_range",
);
assertFailsOnlyIntendedFamily(
  "isotropic brushed metal",
  (tiles) => {
    const target = subject([78, 86, 96]);
    fillRect(target, 54, 54, 74, 74, [250, 250, 250, 255]);
    tiles.brushed_steel = target;
  },
  "brushed_steel_anisotropy",
);
assertFailsOnlyIntendedFamily(
  "identical neighbors",
  (tiles) => { tiles.rough_metal = clone(tiles.metal); },
  "neighbor_delta",
);
assertFailsOnlyIntendedFamily(
  "lost clearcoat",
  (tiles) => { tiles.clearcoat_plastic = clone(tiles.plastic); },
  "clearcoat_lobe",
);
assertFailsOnlyIntendedFamily(
  "missing transmission/refraction",
  (tiles) => { tiles.clear_glass = subject([165, 180, 190]); },
  "clear_glass_refraction",
);
assertFailsOnlyIntendedFamily(
  "removed texture variance",
  (tiles) => { tiles.leather = subject([150, 72, 28]); },
  "leather_texture_variance",
);

console.log("round-e shared material evaluator: good fixture and six mutations passed");

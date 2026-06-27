#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const DEFAULT_URL = "https://scena-demo.pages.dev/proof/?sample=material-presets";
const url = process.argv[2] || process.env.SCENA_MATERIAL_PROOF_URL || DEFAULT_URL;
const fixturePath = "tests/visual/references/round_e_material_fixture.toml";
const thresholdsPath = "tests/visual/references/round_e_material_thresholds.toml";
const outDir = path.resolve("target/gate-artifacts/round-e-cloudflare-material-proof");
const artifactPath = path.resolve("target/gate-artifacts/round-e-cloudflare-material-proof.json");

const presets = [
  "matte",
  "plastic",
  "metal",
  "rough_metal",
  "chrome",
  "brushed_steel",
  "clearcoat_plastic",
  "satin",
  "leather",
  "clear_glass",
  "frosted_glass",
  "rubber",
];

const gridCells = new Map([
  ["matte", [0, 0]],
  ["plastic", [1, 0]],
  ["metal", [2, 0]],
  ["rough_metal", [3, 0]],
  ["chrome", [0, 1]],
  ["brushed_steel", [1, 1]],
  ["clearcoat_plastic", [2, 1]],
  ["satin", [3, 1]],
  ["leather", [0, 2]],
  ["clear_glass", [1, 2]],
  ["frosted_glass", [2, 2]],
  ["rubber", [3, 2]],
]);

const proofWindows = new Map([
  ["matte", [0.218, 0.302, 0.20, 0.20]],
  ["plastic", [0.411, 0.302, 0.20, 0.20]],
  ["metal", [0.599, 0.302, 0.18, 0.20]],
  ["rough_metal", [0.776, 0.302, 0.18, 0.20]],
  ["chrome", [0.218, 0.485, 0.20, 0.22]],
  ["brushed_steel", [0.412, 0.485, 0.28, 0.16]],
  ["clearcoat_plastic", [0.600, 0.485, 0.20, 0.22]],
  ["satin", [0.776, 0.485, 0.22, 0.18]],
  ["leather", [0.218, 0.657, 0.23, 0.16]],
  ["clear_glass", [0.412, 0.657, 0.20, 0.18]],
  ["frosted_glass", [0.600, 0.657, 0.20, 0.18]],
  ["rubber", [0.776, 0.657, 0.20, 0.16]],
]);

const glassTransmissionRegion = Object.freeze({
  left: 0.34,
  top: 0.30,
  right: 0.72,
  bottom: 0.70,
});

const ANISOTROPY_ASPECT_RATIO_MEASUREMENT_EPSILON = 0.01;

const neighborPairs = [
  ["metal", "rough_metal"],
  ["metal", "chrome"],
  ["chrome", "plastic"],
  ["clearcoat_plastic", "plastic"],
  ["clear_glass", "frosted_glass"],
  ["rubber", "plastic"],
];

fs.mkdirSync(outDir, { recursive: true });
for (const file of fs.readdirSync(outDir)) {
  if (file.endsWith(".png")) fs.unlinkSync(path.join(outDir, file));
}

function readText(file) {
  return fs.readFileSync(file, "utf8");
}

function sha256HexFile(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function parseThresholds(text) {
  const thresholds = {};
  let section = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.split("#")[0].trim();
    if (!line) continue;
    const sectionMatch = line.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      section = sectionMatch[1];
      thresholds[section] = thresholds[section] || {};
      continue;
    }
    const match = line.match(/^([A-Za-z0-9_]+)\s*=\s*([-+]?\d+(?:\.\d+)?)$/);
    if (section && match) {
      thresholds[section][match[1]] = Number(match[2]);
    }
  }
  return thresholds;
}

function parseFixture(text) {
  const fixture = {};
  let preset = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.split("#")[0].trim();
    if (!line) continue;
    const presetMatch = line.match(/^\[\[presets\.([A-Za-z0-9_]+)\]\]$/);
    if (presetMatch) {
      preset = presetMatch[1];
      fixture[preset] = {};
      continue;
    }
    const match = line.match(/^([A-Za-z0-9_]+)\s*=\s*(.+)$/);
    if (!preset || !match) continue;
    const value = match[2].trim();
    if (value.startsWith('"') && value.endsWith('"')) {
      fixture[preset][match[1]] = value.slice(1, -1);
    } else if (value.startsWith("[") && value.endsWith("]")) {
      fixture[preset][match[1]] = value
        .slice(1, -1)
        .split(",")
        .map((entry) => entry.trim().replace(/^"|"$/g, ""))
        .filter(Boolean);
    } else if (/^[-+]?\d+(?:\.\d+)?$/.test(value)) {
      fixture[preset][match[1]] = Number(value);
    } else {
      fixture[preset][match[1]] = value;
    }
  }
  return fixture;
}

async function readNormalizedForegroundRgba(decoderPage, file, size = 128, options = {}) {
  const dataUrl = `data:image/png;base64,${fs.readFileSync(file).toString("base64")}`;
  const values = await decoderPage.evaluate(
    async ({ dataUrl, size, isolateCenterComponent }) => {
      const image = new Image();
      const loaded = new Promise((resolve, reject) => {
        image.onload = resolve;
        image.onerror = () => reject(new Error(`could not decode ${dataUrl.slice(0, 64)}`));
      });
      image.src = dataUrl;
      await loaded;
      const source = document.createElement("canvas");
      source.width = image.naturalWidth || image.width;
      source.height = image.naturalHeight || image.height;
      const sourceCtx = source.getContext("2d", { willReadFrequently: true });
      sourceCtx.drawImage(image, 0, 0);
      const sourceImage = sourceCtx.getImageData(0, 0, source.width, source.height);
      const pixels = sourceImage.data;
      const cornerSamples = [];
      const cornerSize = Math.min(8, Math.floor(source.width / 4), Math.floor(source.height / 4));
      for (const [x0, y0] of [
        [0, 0],
        [source.width - cornerSize, 0],
        [0, source.height - cornerSize],
        [source.width - cornerSize, source.height - cornerSize],
      ]) {
        for (let y = y0; y < y0 + cornerSize; y += 1) {
          for (let x = x0; x < x0 + cornerSize; x += 1) {
            const i = (y * source.width + x) * 4;
            cornerSamples.push([pixels[i], pixels[i + 1], pixels[i + 2]]);
          }
        }
      }
      const background = cornerSamples.reduce(
        (sum, sample) => [sum[0] + sample[0], sum[1] + sample[1], sum[2] + sample[2]],
        [0, 0, 0],
      ).map((value) => value / Math.max(1, cornerSamples.length));
      let minX = source.width;
      let minY = source.height;
      let maxX = -1;
      let maxY = -1;
      let foreground = 0;
      const foregroundMask = new Uint8Array(source.width * source.height);
      for (let y = 0; y < source.height; y += 1) {
        for (let x = 0; x < source.width; x += 1) {
          const i = (y * source.width + x) * 4;
          if (pixels[i + 3] <= 8) continue;
          const distance = Math.hypot(
            pixels[i] - background[0],
            pixels[i + 1] - background[1],
            pixels[i + 2] - background[2],
          );
          if (distance <= 7) continue;
          minX = Math.min(minX, x);
          minY = Math.min(minY, y);
          maxX = Math.max(maxX, x);
          maxY = Math.max(maxY, y);
          foreground += 1;
          foregroundMask[y * source.width + x] = 1;
        }
      }
      if (isolateCenterComponent && foreground >= 64) {
        const component = centeredForegroundComponent(foregroundMask, source.width, source.height);
        if (component && component.count >= 64) {
          minX = component.minX;
          minY = component.minY;
          maxX = component.maxX;
          maxY = component.maxY;
          foreground = component.count;
        }
      }
      if (foreground < 64 || maxX < minX || maxY < minY) {
        minX = 0;
        minY = 0;
        maxX = source.width - 1;
        maxY = source.height - 1;
      } else {
        const pad = Math.max(4, Math.round(Math.min(source.width, source.height) * 0.04));
        minX = Math.max(0, minX - pad);
        minY = Math.max(0, minY - pad);
        maxX = Math.min(source.width - 1, maxX + pad);
        maxY = Math.min(source.height - 1, maxY + pad);
      }
      const canvas = document.createElement("canvas");
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext("2d", { willReadFrequently: true });
      ctx.drawImage(source, minX, minY, maxX - minX + 1, maxY - minY + 1, 0, 0, size, size);
      return Array.from(ctx.getImageData(0, 0, size, size).data);

      function centeredForegroundComponent(mask, width, height) {
        const visited = new Uint8Array(mask.length);
        const targetX = (width - 1) / 2;
        const targetY = (height - 1) / 2;
        let best = null;
        const queue = [];
        for (let start = 0; start < mask.length; start += 1) {
          if (!mask[start] || visited[start]) continue;
          queue.length = 0;
          queue.push(start);
          visited[start] = 1;
          let count = 0;
          let minX = width;
          let minY = height;
          let maxX = -1;
          let maxY = -1;
          let sumX = 0;
          let sumY = 0;
          for (let head = 0; head < queue.length; head += 1) {
            const index = queue[head];
            const x = index % width;
            const y = Math.floor(index / width);
            count += 1;
            minX = Math.min(minX, x);
            minY = Math.min(minY, y);
            maxX = Math.max(maxX, x);
            maxY = Math.max(maxY, y);
            sumX += x;
            sumY += y;
            const neighbors = [index - 1, index + 1, index - width, index + width];
            for (const neighbor of neighbors) {
              if (neighbor < 0 || neighbor >= mask.length || visited[neighbor] || !mask[neighbor]) {
                continue;
              }
              const nx = neighbor % width;
              if (neighbor === index - 1 && nx !== x - 1) continue;
              if (neighbor === index + 1 && nx !== x + 1) continue;
              visited[neighbor] = 1;
              queue.push(neighbor);
            }
          }
          const centerX = sumX / Math.max(1, count);
          const centerY = sumY / Math.max(1, count);
          const distance = Math.hypot(centerX - targetX, centerY - targetY);
          const score = distance - Math.log2(Math.max(1, count)) * 1.5;
          if (!best || score < best.score) {
            best = { count, minX, minY, maxX, maxY, score };
          }
        }
        return best;
      }
    },
    { dataUrl, size, isolateCenterComponent: options.isolateCenterComponent ?? true },
  );
  return Uint8Array.from(values);
}

async function readRgbaImage(decoderPage, file) {
  const dataUrl = `data:image/png;base64,${fs.readFileSync(file).toString("base64")}`;
  const image = await decoderPage.evaluate(async (dataUrl) => {
    const image = new Image();
    const loaded = new Promise((resolve, reject) => {
      image.onload = resolve;
      image.onerror = () => reject(new Error(`could not decode ${dataUrl.slice(0, 64)}`));
    });
    image.src = dataUrl;
    await loaded;
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth || image.width;
    canvas.height = image.naturalHeight || image.height;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    ctx.drawImage(image, 0, 0);
    return {
      width: canvas.width,
      height: canvas.height,
      data: Array.from(ctx.getImageData(0, 0, canvas.width, canvas.height).data),
    };
  }, dataUrl);
  return { width: image.width, height: image.height, data: Uint8Array.from(image.data) };
}

async function cropMaterial(page, canvasBox, preset, outputPath) {
  const { width, height } = canvasBox;
  const proofWindow = proofWindows.get(preset);
  if (proofWindow) {
    const [cx, cy, w, h] = proofWindow;
    const crop = {
      x: Math.max(0, Math.floor(width * (cx - w / 2))),
      y: Math.max(0, Math.floor(height * (cy - h / 2))),
      width: Math.max(1, Math.ceil(width * w)),
      height: Math.max(1, Math.ceil(height * h)),
    };
    await page.screenshot({
      path: outputPath,
      clip: {
        x: canvasBox.x + crop.x,
        y: canvasBox.y + crop.y,
        width: crop.width,
        height: crop.height,
      },
    });
    return crop;
  }
  const cell = gridCells.get(preset);
  if (!cell) throw new Error(`missing grid cell for ${preset}`);
  const [col, row] = cell;
  const cellWidth = width / 4;
  const cellHeight = height / 3;
  const crop = {
    x: Math.max(0, Math.floor(col * cellWidth)),
    y: Math.max(0, Math.floor(row * cellHeight)),
    width: Math.max(1, Math.ceil(cellWidth)),
    height: Math.max(1, Math.ceil(cellHeight)),
  };
  await page.screenshot({
    path: outputPath,
    clip: {
      x: canvasBox.x + crop.x,
      y: canvasBox.y + crop.y,
      width: crop.width,
      height: crop.height,
    },
  });
  return crop;
}

function srgbToLinear(value) {
  const c = value / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function rgbToLab(r, g, b) {
  const rl = srgbToLinear(r);
  const gl = srgbToLinear(g);
  const bl = srgbToLinear(b);
  let x = (0.4124564 * rl + 0.3575761 * gl + 0.1804375 * bl) / 0.95047;
  let y = (0.2126729 * rl + 0.7151522 * gl + 0.072175 * bl) / 1.0;
  let z = (0.0193339 * rl + 0.119192 * gl + 0.9503041 * bl) / 1.08883;
  const pivot = (v) => (v > 0.008856 ? Math.cbrt(v) : 7.787 * v + 16 / 116);
  x = pivot(x);
  y = pivot(y);
  z = pivot(z);
  return [116 * y - 16, 500 * (x - y), 200 * (y - z)];
}

function degrees(value) {
  return (value * 180) / Math.PI;
}

function radians(value) {
  return (value * Math.PI) / 180;
}

function deltaE2000(lab1, lab2) {
  // CIEDE2000, used here as the Round E material identity comparison metric.
  const [l1, a1, b1] = lab1;
  const [l2, a2, b2] = lab2;
  const c1 = Math.hypot(a1, b1);
  const c2 = Math.hypot(a2, b2);
  const cBar = (c1 + c2) / 2;
  const cBar7 = cBar ** 7;
  const g = 0.5 * (1 - Math.sqrt(cBar7 / (cBar7 + 25 ** 7)));
  const ap1 = (1 + g) * a1;
  const ap2 = (1 + g) * a2;
  const cp1 = Math.hypot(ap1, b1);
  const cp2 = Math.hypot(ap2, b2);
  const hp = (ap, b) => {
    if (ap === 0 && b === 0) return 0;
    const h = degrees(Math.atan2(b, ap));
    return h >= 0 ? h : h + 360;
  };
  const hp1 = hp(ap1, b1);
  const hp2 = hp(ap2, b2);
  const dlp = l2 - l1;
  const dcp = cp2 - cp1;
  let dhp = hp2 - hp1;
  if (cp1 * cp2 === 0) dhp = 0;
  else if (dhp > 180) dhp -= 360;
  else if (dhp < -180) dhp += 360;
  const dh = 2 * Math.sqrt(cp1 * cp2) * Math.sin(radians(dhp / 2));
  const lpBar = (l1 + l2) / 2;
  const cpBar = (cp1 + cp2) / 2;
  let hpBar = hp1 + hp2;
  if (cp1 * cp2 === 0) hpBar = hp1 + hp2;
  else if (Math.abs(hp1 - hp2) <= 180) hpBar = (hp1 + hp2) / 2;
  else if (hp1 + hp2 < 360) hpBar = (hp1 + hp2 + 360) / 2;
  else hpBar = (hp1 + hp2 - 360) / 2;
  const t =
    1 -
    0.17 * Math.cos(radians(hpBar - 30)) +
    0.24 * Math.cos(radians(2 * hpBar)) +
    0.32 * Math.cos(radians(3 * hpBar + 6)) -
    0.20 * Math.cos(radians(4 * hpBar - 63));
  const deltaTheta = 30 * Math.exp(-(((hpBar - 275) / 25) ** 2));
  const cpBar7 = cpBar ** 7;
  const rc = 2 * Math.sqrt(cpBar7 / (cpBar7 + 25 ** 7));
  const sl = 1 + (0.015 * (lpBar - 50) ** 2) / Math.sqrt(20 + (lpBar - 50) ** 2);
  const sc = 1 + 0.045 * cpBar;
  const sh = 1 + 0.015 * cpBar * t;
  const rt = -Math.sin(radians(2 * deltaTheta)) * rc;
  const dl = dlp / sl;
  const dc = dcp / sc;
  const dhTerm = dh / sh;
  return Math.sqrt(dl * dl + dc * dc + dhTerm * dhTerm + rt * dc * dhTerm);
}

async function meanDeltaE2000(decoderPage, aPath, bPath, options = {}) {
  const size = 128;
  const a = await readNormalizedForegroundRgba(decoderPage, aPath, size, options);
  const b = await readNormalizedForegroundRgba(decoderPage, bPath, size, options);
  const aBackground = estimateBackgroundRgb(a, size);
  const bBackground = estimateBackgroundRgb(b, size);
  let sum = 0;
  let count = 0;
  for (let i = 0; i + 3 < a.length && i + 3 < b.length; i += 4) {
    const aForeground = isForegroundPixel(a, i, aBackground);
    const bForeground = isForegroundPixel(b, i, bBackground);
    if (!aForeground && !bForeground) continue;
    sum += deltaE2000(rgbToLab(a[i], a[i + 1], a[i + 2]), rgbToLab(b[i], b[i + 1], b[i + 2]));
    count += 1;
  }
  return count === 0 ? Infinity : sum / count;
}

async function luminanceValues(decoderPage, file) {
  const rgba = await readNormalizedForegroundRgba(decoderPage, file, 128);
  const background = estimateBackgroundRgb(rgba, 128);
  const values = [];
  for (let i = 0; i + 3 < rgba.length; i += 4) {
    if (!isForegroundPixel(rgba, i, background)) continue;
    values.push(0.2126 * rgba[i] + 0.7152 * rgba[i + 1] + 0.0722 * rgba[i + 2]);
  }
  values.sort((a, b) => a - b);
  return values;
}

async function luminancePercentiles(decoderPage, file) {
  const values = await luminanceValues(decoderPage, file);
  return {
    p05: percentile(values, 0.05),
    p10: percentile(values, 0.10),
    p50: percentile(values, 0.50),
    p95: percentile(values, 0.95),
    p99: percentile(values, 0.99),
  };
}

async function luminanceStats(decoderPage, file) {
  const values = await luminanceValues(decoderPage, file);
  if (values.length === 0) {
    return { count: 0, mean: 0, stddev: 0 };
  }
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  const variance =
    values.reduce((sum, value) => sum + (value - mean) * (value - mean), 0) / values.length;
  return {
    count: values.length,
    mean,
    stddev: Math.sqrt(variance),
  };
}

async function darkTargetOffset(decoderPage, file) {
  const image = await readRgbaImage(decoderPage, file);
  const background = estimateBackgroundRgb(image.data, image.width, image.height);
  const backgroundLum = luminanceRgb(background[0], background[1], background[2]);
  let count = 0;
  let sumX = 0;
  let sumY = 0;
  for (let y = 0; y < image.height; y += 1) {
    for (let x = 0; x < image.width; x += 1) {
      const offset = (y * image.width + x) * 4;
      if (image.data[offset + 3] <= 8) continue;
      const lum = luminanceRgb(image.data[offset], image.data[offset + 1], image.data[offset + 2]);
      if (lum >= backgroundLum - 18) continue;
      count += 1;
      sumX += x;
      sumY += y;
    }
  }
  if (count === 0) {
    return { count: 0, offsetPx: 0, centerX: null, centerY: null };
  }
  const centerX = sumX / count;
  const centerY = sumY / count;
  return {
    count,
    centerX,
    centerY,
    offsetPx: Math.hypot(centerX - image.width / 2, centerY - image.height / 2),
  };
}

async function sobelEdgeEnergy(decoderPage, file, options = {}) {
  const image = await readRgbaImage(decoderPage, file);
  const region = options.region || { left: 0, top: 0, right: 1, bottom: 1 };
  const minX = Math.max(1, Math.floor(image.width * region.left));
  const maxX = Math.min(image.width - 2, Math.ceil(image.width * region.right));
  const minY = Math.max(1, Math.floor(image.height * region.top));
  const maxY = Math.min(image.height - 2, Math.ceil(image.height * region.bottom));
  const gray = new Float32Array(image.width * image.height);
  for (let i = 0; i + 3 < image.data.length; i += 4) {
    gray[i / 4] = luminanceRgb(image.data[i], image.data[i + 1], image.data[i + 2]);
  }
  let sum = 0;
  let count = 0;
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const gx =
        -gray[(y - 1) * image.width + x - 1] +
        gray[(y - 1) * image.width + x + 1] -
        2 * gray[y * image.width + x - 1] +
        2 * gray[y * image.width + x + 1] -
        gray[(y + 1) * image.width + x - 1] +
        gray[(y + 1) * image.width + x + 1];
      const gy =
        -gray[(y - 1) * image.width + x - 1] -
        2 * gray[(y - 1) * image.width + x] -
        gray[(y - 1) * image.width + x + 1] +
        gray[(y + 1) * image.width + x - 1] +
        2 * gray[(y + 1) * image.width + x] +
        gray[(y + 1) * image.width + x + 1];
      sum += Math.hypot(gx, gy) / 255;
      count += 1;
    }
  }
  return count === 0 ? 0 : sum / count;
}

function estimateBackgroundRgb(rgba, width, height = width) {
  const cornerSize = Math.min(8, Math.max(1, Math.floor(Math.min(width, height) / 4)));
  const samples = [];
  for (const [x0, y0] of [
    [0, 0],
    [width - cornerSize, 0],
    [0, height - cornerSize],
    [width - cornerSize, height - cornerSize],
  ]) {
    for (let y = y0; y < y0 + cornerSize; y += 1) {
      for (let x = x0; x < x0 + cornerSize; x += 1) {
        const i = (y * width + x) * 4;
        samples.push([rgba[i], rgba[i + 1], rgba[i + 2]]);
      }
    }
  }
  return samples.reduce(
    (sum, sample) => [sum[0] + sample[0], sum[1] + sample[1], sum[2] + sample[2]],
    [0, 0, 0],
  ).map((value) => value / Math.max(1, samples.length));
}

function luminanceRgb(r, g, b) {
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function isForegroundPixel(rgba, offset, background) {
  if (rgba[offset + 3] <= 8) return false;
  return Math.hypot(
    rgba[offset] - background[0],
    rgba[offset + 1] - background[1],
    rgba[offset + 2] - background[2],
  ) > 7;
}

function percentile(values, p) {
  if (values.length === 0) return 0;
  return values[Math.min(values.length - 1, Math.max(0, Math.floor((values.length - 1) * p)))];
}

function referenceDeltaIsHardGate(preset) {
  // Public material approval must fail closed. Structural metrics catch the
  // known glossy-sphere failure mode, but the live Cloudflare page is not
  // approved unless each preset also stays within its committed external
  // reference DeltaE budget. The internal 12-sphere review page is only a
  // human sanity check; this deployment proof is the public gate.
  void preset;
  return true;
}

async function specularDynamicRange(decoderPage, file) {
  const values = await luminanceValues(decoderPage, file);
  return percentile(values, 0.99) / Math.max(1, percentile(values, 0.10));
}

async function highlightAspectRatio(decoderPage, file) {
  const rgba = await readNormalizedForegroundRgba(decoderPage, file, 128);
  const background = estimateBackgroundRgb(rgba, 128);
  const foreground = [];
  const luminance = [];
  for (let y = 0; y < 128; y += 1) {
    for (let x = 0; x < 128; x += 1) {
      const offset = (y * 128 + x) * 4;
      if (!isForegroundPixel(rgba, offset, background)) continue;
      const lum = 0.2126 * rgba[offset] + 0.7152 * rgba[offset + 1] + 0.0722 * rgba[offset + 2];
      foreground.push({ x, y, lum });
      luminance.push(lum);
    }
  }
  if (foreground.length < 16) return 0;
  luminance.sort((a, b) => a - b);
  const cutoff = percentile(luminance, 0.95);
  const highlight = foreground.filter((pixel) => pixel.lum >= cutoff);
  if (highlight.length < 8) return 0;
  const totalWeight = highlight.reduce((sum, pixel) => sum + Math.max(1, pixel.lum), 0);
  const meanX = highlight.reduce((sum, pixel) => sum + pixel.x * Math.max(1, pixel.lum), 0) / totalWeight;
  const meanY = highlight.reduce((sum, pixel) => sum + pixel.y * Math.max(1, pixel.lum), 0) / totalWeight;
  let xx = 0;
  let yy = 0;
  let xy = 0;
  for (const pixel of highlight) {
    const weight = Math.max(1, pixel.lum);
    const dx = pixel.x - meanX;
    const dy = pixel.y - meanY;
    xx += weight * dx * dx;
    yy += weight * dy * dy;
    xy += weight * dx * dy;
  }
  xx /= totalWeight;
  yy /= totalWeight;
  xy /= totalWeight;
  const trace = xx + yy;
  const determinant = xx * yy - xy * xy;
  const discriminant = Math.sqrt(Math.max(0, (trace * trace) / 4 - determinant));
  const major = trace / 2 + discriminant;
  const minor = Math.max(1e-6, trace / 2 - discriminant);
  return Math.sqrt(major / minor);
}

async function launchBrowser() {
  const options = {
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  };
  if (process.env.CHROMIUM && fs.existsSync(process.env.CHROMIUM)) {
    options.executablePath = process.env.CHROMIUM;
  }
  return chromium.launch(options);
}

async function cacheAndWasmProof(pageUrl) {
  const pageResponse = await fetch(pageUrl);
  const html = await pageResponse.text();
  const scriptMatch = findVersionedScript(html);
  const scriptPath = scriptMatch?.path || null;
  const localPaths =
    scriptPath && scriptPath.endsWith("/proof.js")
      ? {
          html: "demo/proof/index.html",
          script: "demo/proof.js",
          wasm: "demo/proof/pkg/scena_bg.wasm",
        }
      : {
          html: "demo/index.html",
          script: "demo/main.js",
          wasm: "demo/pkg/scena_bg.wasm",
        };
  const localHtml = fs.existsSync(localPaths.html) ? readText(localPaths.html) : "";
  const localScript = fs.existsSync(localPaths.script) ? readText(localPaths.script) : "";
  const localScriptBuster = findVersionedScript(localHtml, path.basename(localPaths.script))?.buster || null;
  const scriptUrl = scriptMatch ? new URL(scriptMatch.src, pageUrl).toString() : null;
  const liveScriptBuster = scriptMatch?.buster || null;
  let liveWasmBuster = null;
  let remoteWasmSha256 = null;
  let localWasmSha256 = null;
  if (scriptUrl) {
    const scriptText = await (await fetch(scriptUrl)).text();
    const wasmMatch = findVersionedWasm(scriptText);
    liveWasmBuster = wasmMatch?.buster || null;
    const wasmRelative = wasmMatch?.src || null;
    if (wasmRelative) {
      const wasmUrl = new URL(wasmRelative, scriptUrl).toString();
      const bytes = Buffer.from(await (await fetch(wasmUrl)).arrayBuffer());
      remoteWasmSha256 = crypto.createHash("sha256").update(bytes).digest("hex");
    }
  }
  if (fs.existsSync(localPaths.wasm)) {
    localWasmSha256 = sha256HexFile(localPaths.wasm);
  }
  const localWasmBuster = findVersionedWasm(localScript)?.buster || null;
  return {
    bumped: Boolean(
      localScriptBuster &&
        localScriptBuster === liveScriptBuster &&
        localWasmBuster === liveWasmBuster,
    ),
    local_main_buster: localScriptBuster,
    live_main_buster: liveScriptBuster,
    local_wasm_buster: localWasmBuster,
    live_wasm_buster: liveWasmBuster,
    wasm: {
      checksum_matches_build: Boolean(
        localWasmSha256 && remoteWasmSha256 && localWasmSha256 === remoteWasmSha256,
      ),
      local_sha256: localWasmSha256,
      remote_sha256: remoteWasmSha256,
    },
  };
}

function findVersionedScript(html, preferredName = null) {
  const pattern = /<script[^>]+src="([^"]*?([^/"?#]+\.js)\?v=([^"]+))"/g;
  let fallback = null;
  for (const match of html.matchAll(pattern)) {
    const result = {
      src: match[1],
      path: new URL(match[1], "https://example.invalid/").pathname,
      name: match[2],
      buster: match[3],
    };
    if (!fallback) fallback = result;
    if (preferredName && result.name === preferredName) return result;
    if (!preferredName && (result.name === "proof.js" || result.name === "main.js")) return result;
  }
  return fallback;
}

function findVersionedWasm(scriptText) {
  const match = scriptText.match(/["'`]([^"'`]*scena_bg\.wasm\?v=([^"'`)]+))["'`]/);
  return match ? { src: match[1], buster: match[2] } : null;
}

async function main() {
  const fixture = parseFixture(readText(fixturePath));
  const thresholds = parseThresholds(readText(thresholdsPath));
  const browser = await launchBrowser();
  const errors = [];
  const perMaterial = {};
  const neighborResults = [];
  let cacheProof = null;
  try {
    cacheProof = await cacheAndWasmProof(url);
    if (!cacheProof.bumped) {
      errors.push(
        `cache buster mismatch: local main ${cacheProof.local_main_buster ?? "missing"}, live main ${cacheProof.live_main_buster ?? "missing"}, local wasm ${cacheProof.local_wasm_buster ?? "missing"}, live wasm ${cacheProof.live_wasm_buster ?? "missing"}`,
      );
    }
    if (!cacheProof.wasm.checksum_matches_build) {
      errors.push(
        `wasm checksum mismatch: local ${cacheProof.wasm.local_sha256 ?? "missing"}, remote ${cacheProof.wasm.remote_sha256 ?? "missing"}`,
      );
    }
  } catch (error) {
    errors.push(`cache/wasm proof failed: ${error.message}`);
  }
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 960 } });
    const decoderPage = await browser.newPage();
    page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(`console: ${message.text()}`);
    });
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 90000 });
    await page.waitForFunction(
      () =>
        /rendered/i.test(document.getElementById("status-detail")?.textContent || "") &&
        Number(document.getElementById("metric-frame")?.textContent || "0") >= 1,
      { timeout: 120000 },
    );
    const canvasBox = await page.locator("#canvas").boundingBox();
    if (!canvasBox) {
      throw new Error("material proof canvas was not visible");
    }
    const canvasPath = path.join(outDir, "canvas.png");
    await page.locator("#canvas").screenshot({ path: canvasPath });
    for (const preset of presets) {
      const cropPath = path.join(outDir, `${preset}.png`);
      const crop = await cropMaterial(page, canvasBox, preset, cropPath);
      const referencePath = fixture[preset]?.reference_path;
      if (!referencePath || !fs.existsSync(referencePath)) {
        errors.push(`${preset} reference missing at ${referencePath}`);
        continue;
      }
      const delta = await meanDeltaE2000(decoderPage, cropPath, referencePath, {
        isolateCenterComponent: !preset.includes("glass"),
      });
      const maxDelta =
        thresholds[preset]?.delta_e2000_max ?? thresholds.global?.reference_delta_e2000_max;
      const metrics = {
        crop_path: path.relative(process.cwd(), cropPath),
        reference_path: referencePath,
        crop_window: crop,
        delta_e2000_vs_reference: Number(delta.toFixed(3)),
        delta_e2000_max: maxDelta,
        reference_delta_gate: referenceDeltaIsHardGate(preset) ? "hard" : "diagnostic",
        passed_reference_delta: Number.isFinite(delta) && delta <= maxDelta,
      };
      if (preset === "chrome") {
        const luminance = await luminancePercentiles(decoderPage, cropPath);
        metrics.luminance_p05 = Number(luminance.p05.toFixed(3));
        metrics.luminance_p99 = Number(luminance.p99.toFixed(3));
        metrics.specular_dynamic_range = Number((luminance.p99 / Math.max(1, luminance.p10)).toFixed(3));
        metrics.specular_dynamic_range_min = thresholds.chrome?.specular_dynamic_range;
        metrics.passed_specular_dynamic_range =
          metrics.specular_dynamic_range >= metrics.specular_dynamic_range_min;
        metrics.dark_reflection_luminance_p05_max = thresholds.chrome?.dark_reflection_luminance_p05_max;
        metrics.bright_reflection_luminance_p99_min = thresholds.chrome?.bright_reflection_luminance_p99_min;
        metrics.passed_dark_reflection_luminance =
          metrics.luminance_p05 <= metrics.dark_reflection_luminance_p05_max;
        metrics.passed_bright_reflection_luminance =
          metrics.luminance_p99 >= metrics.bright_reflection_luminance_p99_min;
      }
      perMaterial[preset] = metrics;
      if (preset === "chrome") {
        if (!metrics.passed_specular_dynamic_range) {
          errors.push(
            `${preset} specular dynamic range ${metrics.specular_dynamic_range} < ${metrics.specular_dynamic_range_min}`,
          );
        }
        if (!metrics.passed_dark_reflection_luminance) {
          errors.push(
            `${preset} dark reflection p05 ${metrics.luminance_p05} > ${metrics.dark_reflection_luminance_p05_max}`,
          );
        }
        if (!metrics.passed_bright_reflection_luminance) {
          errors.push(
            `${preset} bright reflection p99 ${metrics.luminance_p99} < ${metrics.bright_reflection_luminance_p99_min}`,
          );
        }
      }
      if (metrics.reference_delta_gate === "hard" && !metrics.passed_reference_delta) {
        errors.push(`${preset} DeltaE2000 ${metrics.delta_e2000_vs_reference} > ${maxDelta}`);
      }
    }
    if (perMaterial.brushed_steel) {
      const brushedPath = path.join(outDir, "brushed_steel.png");
      const anisotropyAspect = await highlightAspectRatio(decoderPage, brushedPath);
      const minAspect = thresholds.brushed_steel?.anisotropy_aspect_ratio_ibl;
      perMaterial.brushed_steel.anisotropy_aspect_ratio_ibl = Number(anisotropyAspect.toFixed(3));
      perMaterial.brushed_steel.anisotropy_aspect_ratio_ibl_min = minAspect;
      perMaterial.brushed_steel.anisotropy_aspect_ratio_ibl_measurement_epsilon =
        ANISOTROPY_ASPECT_RATIO_MEASUREMENT_EPSILON;
      perMaterial.brushed_steel.passed_anisotropy_aspect_ratio_ibl =
        Number.isFinite(anisotropyAspect) &&
        anisotropyAspect + ANISOTROPY_ASPECT_RATIO_MEASUREMENT_EPSILON >= minAspect;
      if (!perMaterial.brushed_steel.passed_anisotropy_aspect_ratio_ibl) {
        errors.push(
          `brushed_steel anisotropy aspect ratio ${perMaterial.brushed_steel.anisotropy_aspect_ratio_ibl} < ${minAspect}`,
        );
      }
    }
    if (perMaterial.clearcoat_plastic && perMaterial.plastic) {
      const plasticLuminance = await luminancePercentiles(decoderPage, path.join(outDir, "plastic.png"));
      const clearcoatLuminance = await luminancePercentiles(decoderPage, path.join(outDir, "clearcoat_plastic.png"));
      const lobeDelta = Math.max(0, (clearcoatLuminance.p99 - plasticLuminance.p99) / 255);
      const minLobeDelta = thresholds.clearcoat_plastic?.clearcoat_lobe_delta;
      perMaterial.clearcoat_plastic.plastic_luminance_p99 = Number(plasticLuminance.p99.toFixed(3));
      perMaterial.clearcoat_plastic.clearcoat_luminance_p99 = Number(clearcoatLuminance.p99.toFixed(3));
      perMaterial.clearcoat_plastic.clearcoat_lobe_delta = Number(lobeDelta.toFixed(3));
      perMaterial.clearcoat_plastic.clearcoat_lobe_delta_min = minLobeDelta;
      perMaterial.clearcoat_plastic.passed_clearcoat_lobe_delta =
        Number.isFinite(lobeDelta) && lobeDelta >= minLobeDelta;
      if (!perMaterial.clearcoat_plastic.passed_clearcoat_lobe_delta) {
        errors.push(
          `clearcoat_plastic lobe delta ${perMaterial.clearcoat_plastic.clearcoat_lobe_delta} < ${minLobeDelta}`,
        );
      }
    }
    if (perMaterial.leather) {
      const stats = await luminanceStats(decoderPage, path.join(outDir, "leather.png"));
      const textureVariance = stats.stddev / 255;
      const minTextureVariance = thresholds.leather?.texture_variance_min;
      perMaterial.leather.texture_variance = Number(textureVariance.toFixed(3));
      perMaterial.leather.texture_variance_min = minTextureVariance;
      perMaterial.leather.passed_texture_variance =
        Number.isFinite(textureVariance) && textureVariance >= minTextureVariance;
      if (!perMaterial.leather.passed_texture_variance) {
        errors.push(
          `leather texture variance ${perMaterial.leather.texture_variance} < ${minTextureVariance}`,
        );
      }
    }
    if (perMaterial.rubber) {
      const stats = await luminanceStats(decoderPage, path.join(outDir, "rubber.png"));
      const roughnessVariance = stats.stddev / 255;
      const minRoughnessVariance = thresholds.rubber?.roughness_variance_min;
      perMaterial.rubber.roughness_variance = Number(roughnessVariance.toFixed(3));
      perMaterial.rubber.roughness_variance_min = minRoughnessVariance;
      perMaterial.rubber.passed_roughness_variance =
        Number.isFinite(roughnessVariance) && roughnessVariance >= minRoughnessVariance;
      if (!perMaterial.rubber.passed_roughness_variance) {
        errors.push(
          `rubber roughness variance ${perMaterial.rubber.roughness_variance} < ${minRoughnessVariance}`,
        );
      }
    }
    if (perMaterial.satin) {
      const luminance = await luminancePercentiles(decoderPage, path.join(outDir, "satin.png"));
      const sheenWidth = Math.max(0, (luminance.p95 - luminance.p50) / 255);
      const minSheenWidth = thresholds.satin?.sheen_width_min;
      perMaterial.satin.sheen_width = Number(sheenWidth.toFixed(3));
      perMaterial.satin.sheen_width_min = minSheenWidth;
      perMaterial.satin.passed_sheen_width = Number.isFinite(sheenWidth) && sheenWidth >= minSheenWidth;
      if (!perMaterial.satin.passed_sheen_width) {
        errors.push(`satin sheen width ${perMaterial.satin.sheen_width} < ${minSheenWidth}`);
      }
    }
    if (perMaterial.clear_glass) {
      const target = await darkTargetOffset(decoderPage, path.join(outDir, "clear_glass.png"));
      const minOffset = thresholds.clear_glass?.refraction_offset_min;
      perMaterial.clear_glass.physical_refraction_status = "measured";
      perMaterial.clear_glass.refraction_offset_px = Number(target.offsetPx.toFixed(3));
      perMaterial.clear_glass.refraction_offset_min = minOffset;
      perMaterial.clear_glass.dark_target_pixel_count = target.count;
      perMaterial.clear_glass.dark_target_center = target.centerX === null
        ? null
        : [Number(target.centerX.toFixed(3)), Number(target.centerY.toFixed(3))];
      perMaterial.clear_glass.passed_physical_refraction =
        target.count >= 64 && Number.isFinite(target.offsetPx) && target.offsetPx >= minOffset;
      if (!perMaterial.clear_glass.passed_physical_refraction) {
        errors.push(
          `clear_glass physical refraction ${perMaterial.clear_glass.refraction_offset_px} < ${minOffset} or dark target pixels ${target.count} < 64`,
        );
      }
    }
    if (perMaterial.frosted_glass && perMaterial.clear_glass) {
      const edgeOptions = { region: glassTransmissionRegion };
      const clearEdge = await sobelEdgeEnergy(
        decoderPage,
        path.join(outDir, "clear_glass.png"),
        edgeOptions,
      );
      const frostedEdge = await sobelEdgeEnergy(
        decoderPage,
        path.join(outDir, "frosted_glass.png"),
        edgeOptions,
      );
      const reduction = clearEdge > 0 ? 1 - frostedEdge / clearEdge : 0;
      const minReduction = thresholds.frosted_glass?.high_frequency_contrast_reduction_min;
      perMaterial.frosted_glass.rough_transmission_status = "measured";
      perMaterial.frosted_glass.rough_transmission_region = glassTransmissionRegion;
      perMaterial.frosted_glass.clear_glass_edge_energy = Number(clearEdge.toFixed(4));
      perMaterial.frosted_glass.frosted_glass_edge_energy = Number(frostedEdge.toFixed(4));
      perMaterial.frosted_glass.high_frequency_contrast_reduction = Number(reduction.toFixed(3));
      perMaterial.frosted_glass.high_frequency_contrast_reduction_min = minReduction;
      perMaterial.frosted_glass.passed_high_frequency_contrast_reduction =
        Number.isFinite(reduction) && reduction >= minReduction;
      if (!perMaterial.frosted_glass.passed_high_frequency_contrast_reduction) {
        errors.push(
          `frosted_glass rough transmission ${perMaterial.frosted_glass.high_frequency_contrast_reduction} < ${minReduction}`,
        );
      }
    }
    for (const [a, b] of neighborPairs) {
      const aPath = path.join(outDir, `${a}.png`);
      const bPath = path.join(outDir, `${b}.png`);
      if (!fs.existsSync(aPath) || !fs.existsSync(bPath)) continue;
      const delta = await meanDeltaE2000(decoderPage, aPath, bPath);
      const minDelta = thresholds.global?.neighbor_delta_e2000_min;
      const result = {
        pair: [a, b],
        delta_e2000: Number(delta.toFixed(3)),
        threshold_min: minDelta,
        passed: Number.isFinite(delta) && delta >= minDelta,
      };
      neighborResults.push(result);
      perMaterial[a].neighbor_delta_e2000 = Math.min(
        perMaterial[a].neighbor_delta_e2000 ?? Infinity,
        result.delta_e2000,
      );
      perMaterial[b].neighbor_delta_e2000 = Math.min(
        perMaterial[b].neighbor_delta_e2000 ?? Infinity,
        result.delta_e2000,
      );
      if (!result.passed) {
        errors.push(`${a}/${b} neighbor DeltaE2000 ${result.delta_e2000} < ${minDelta}`);
      }
    }
    await decoderPage.close();
  } finally {
    await browser.close();
  }

  const firstPreset = fixture[presets[0]] || {};
  const artifact = {
    proof_class: "round-e-cloudflare-material-proof",
    url,
    generated_at: new Date().toISOString(),
    fixture: {
      environment_hdr_path: firstPreset.environment_hdr_path,
      environment_hdr_sha256: firstPreset.environment_hdr_sha256,
      tonemapper: firstPreset.tonemapper,
      output_color_space: firstPreset.output_color_space,
      exposure_ev: firstPreset.exposure_ev,
      webgl2_smooth_metal_sample_floor: 96,
    },
    cache_buster: cacheProof
      ? {
          bumped: cacheProof.bumped,
          local_main_buster: cacheProof.local_main_buster,
          live_main_buster: cacheProof.live_main_buster,
          local_wasm_buster: cacheProof.local_wasm_buster,
          live_wasm_buster: cacheProof.live_wasm_buster,
        }
      : { bumped: false },
    wasm: cacheProof?.wasm || { checksum_matches_build: false },
    thresholds,
    per_material: perMaterial,
    neighbor_pairs: neighborResults,
    status: errors.length === 0 ? "pass" : "fail",
    errors,
  };
  fs.writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`);
  console.log(`wrote ${path.relative(process.cwd(), artifactPath)} (${artifact.status})`);
  if (errors.length > 0) {
    for (const error of errors) console.error(`round-e: ${error}`);
    process.exit(1);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

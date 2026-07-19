"use strict";

const PRESETS = Object.freeze([
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
]);

const PROOF_WINDOWS = Object.freeze({
  matte: [0.218, 0.302, 0.20, 0.20],
  plastic: [0.411, 0.302, 0.20, 0.20],
  metal: [0.599, 0.302, 0.18, 0.20],
  rough_metal: [0.776, 0.302, 0.18, 0.20],
  chrome: [0.218, 0.485, 0.20, 0.22],
  brushed_steel: [0.412, 0.485, 0.28, 0.16],
  clearcoat_plastic: [0.600, 0.485, 0.20, 0.22],
  satin: [0.776, 0.485, 0.22, 0.18],
  leather: [0.218, 0.657, 0.23, 0.16],
  clear_glass: [0.412, 0.657, 0.20, 0.18],
  frosted_glass: [0.600, 0.657, 0.20, 0.18],
  rubber: [0.776, 0.657, 0.20, 0.16],
});

const NEIGHBOR_PAIRS = Object.freeze([
  ["metal", "rough_metal"],
  ["metal", "chrome"],
  ["chrome", "plastic"],
  ["clearcoat_plastic", "plastic"],
  ["clear_glass", "frosted_glass"],
  ["rubber", "plastic"],
]);

const GLASS_TRANSMISSION_REGION = Object.freeze({
  left: 0.34,
  top: 0.30,
  right: 0.72,
  bottom: 0.70,
});

const ANISOTROPY_EPSILON = 0.01;

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
    const valueMatch = line.match(/^([A-Za-z0-9_]+)\s*=\s*([-+]?\d+(?:\.\d+)?)$/);
    if (section && valueMatch) thresholds[section][valueMatch[1]] = Number(valueMatch[2]);
  }
  return thresholds;
}

function thresholdsForSurface(thresholds, surface) {
  const resolved = {};
  for (const [section, values] of Object.entries(thresholds)) {
    if (!section.includes(".")) resolved[section] = { ...values };
  }
  const prefix = `${String(surface).replaceAll("-", "_")}.`;
  for (const [section, values] of Object.entries(thresholds)) {
    if (!section.startsWith(prefix)) continue;
    const metricSection = section.slice(prefix.length);
    resolved[metricSection] = { ...(resolved[metricSection] || {}), ...values };
  }
  return resolved;
}

function assertImage(image, label = "image") {
  if (!image || !Number.isInteger(image.width) || !Number.isInteger(image.height)) {
    throw new Error(`${label} must provide integer width and height`);
  }
  const data = image.data || image.rgba;
  if (!data || data.length !== image.width * image.height * 4) {
    throw new Error(`${label} must provide width*height*4 RGBA8 bytes`);
  }
  return { width: image.width, height: image.height, data };
}

function cropImage(image, normalizedWindow) {
  const source = assertImage(image);
  const [cx, cy, widthFraction, heightFraction] = normalizedWindow;
  const x0 = Math.max(0, Math.floor(source.width * (cx - widthFraction / 2)));
  const y0 = Math.max(0, Math.floor(source.height * (cy - heightFraction / 2)));
  const width = Math.min(source.width - x0, Math.max(1, Math.ceil(source.width * widthFraction)));
  const height = Math.min(source.height - y0, Math.max(1, Math.ceil(source.height * heightFraction)));
  const data = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    const sourceStart = ((y0 + y) * source.width + x0) * 4;
    data.set(source.data.subarray(sourceStart, sourceStart + width * 4), y * width * 4);
  }
  return { width, height, data, crop: { x: x0, y: y0, width, height } };
}

function cropRoundEMaterialTiles(frame) {
  const tiles = {};
  for (const preset of PRESETS) {
    tiles[preset] = cropImage(frame, PROOF_WINDOWS[preset]);
  }
  return tiles;
}

function estimateBackgroundRgb(image) {
  const { width, height, data } = assertImage(image);
  const cornerSize = Math.min(8, Math.max(1, Math.floor(Math.min(width, height) / 4)));
  const sum = [0, 0, 0];
  let count = 0;
  for (const [x0, y0] of [
    [0, 0],
    [width - cornerSize, 0],
    [0, height - cornerSize],
    [width - cornerSize, height - cornerSize],
  ]) {
    for (let y = y0; y < y0 + cornerSize; y += 1) {
      for (let x = x0; x < x0 + cornerSize; x += 1) {
        const offset = (y * width + x) * 4;
        sum[0] += data[offset];
        sum[1] += data[offset + 1];
        sum[2] += data[offset + 2];
        count += 1;
      }
    }
  }
  return sum.map((value) => value / Math.max(1, count));
}

function isForegroundPixel(data, offset, background) {
  return data[offset + 3] > 8 && Math.hypot(
    data[offset] - background[0],
    data[offset + 1] - background[1],
    data[offset + 2] - background[2],
  ) > 7;
}

function centeredForegroundComponent(mask, width, height) {
  const visited = new Uint8Array(mask.length);
  const targetX = (width - 1) / 2;
  const targetY = (height - 1) / 2;
  let best = null;
  for (let start = 0; start < mask.length; start += 1) {
    if (!mask[start] || visited[start]) continue;
    const queue = [start];
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
      for (const neighbor of [index - 1, index + 1, index - width, index + width]) {
        if (neighbor < 0 || neighbor >= mask.length || visited[neighbor] || !mask[neighbor]) continue;
        const nx = neighbor % width;
        if (neighbor === index - 1 && nx !== x - 1) continue;
        if (neighbor === index + 1 && nx !== x + 1) continue;
        visited[neighbor] = 1;
        queue.push(neighbor);
      }
    }
    const distance = Math.hypot(sumX / count - targetX, sumY / count - targetY);
    const score = distance - Math.log2(Math.max(1, count)) * 1.5;
    if (!best || score < best.score) best = { count, minX, minY, maxX, maxY, score };
  }
  return best;
}

function sampleBilinear(source, x, y, channel) {
  const x0 = Math.max(0, Math.min(source.width - 1, Math.floor(x)));
  const y0 = Math.max(0, Math.min(source.height - 1, Math.floor(y)));
  const x1 = Math.min(source.width - 1, x0 + 1);
  const y1 = Math.min(source.height - 1, y0 + 1);
  const tx = x - Math.floor(x);
  const ty = y - Math.floor(y);
  const at = (px, py) => source.data[(py * source.width + px) * 4 + channel];
  const top = at(x0, y0) * (1 - tx) + at(x1, y0) * tx;
  const bottom = at(x0, y1) * (1 - tx) + at(x1, y1) * tx;
  return Math.round(top * (1 - ty) + bottom * ty);
}

function normalizeForegroundImage(image, size = 128, isolateCenterComponent = true) {
  const source = assertImage(image);
  const background = estimateBackgroundRgb(source);
  const mask = new Uint8Array(source.width * source.height);
  let bounds = { count: 0, minX: source.width, minY: source.height, maxX: -1, maxY: -1 };
  for (let y = 0; y < source.height; y += 1) {
    for (let x = 0; x < source.width; x += 1) {
      const pixel = y * source.width + x;
      if (!isForegroundPixel(source.data, pixel * 4, background)) continue;
      mask[pixel] = 1;
      bounds.count += 1;
      bounds.minX = Math.min(bounds.minX, x);
      bounds.minY = Math.min(bounds.minY, y);
      bounds.maxX = Math.max(bounds.maxX, x);
      bounds.maxY = Math.max(bounds.maxY, y);
    }
  }
  if (isolateCenterComponent && bounds.count >= 64) {
    const component = centeredForegroundComponent(mask, source.width, source.height);
    if (component && component.count >= 64) bounds = component;
  }
  if (bounds.count < 64 || bounds.maxX < bounds.minX || bounds.maxY < bounds.minY) {
    bounds = { minX: 0, minY: 0, maxX: source.width - 1, maxY: source.height - 1 };
  } else {
    const pad = Math.max(4, Math.round(Math.min(source.width, source.height) * 0.04));
    bounds.minX = Math.max(0, bounds.minX - pad);
    bounds.minY = Math.max(0, bounds.minY - pad);
    bounds.maxX = Math.min(source.width - 1, bounds.maxX + pad);
    bounds.maxY = Math.min(source.height - 1, bounds.maxY + pad);
  }
  const cropWidth = bounds.maxX - bounds.minX + 1;
  const cropHeight = bounds.maxY - bounds.minY + 1;
  const data = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const sourceX = bounds.minX + ((x + 0.5) * cropWidth) / size - 0.5;
      const sourceY = bounds.minY + ((y + 0.5) * cropHeight) / size - 0.5;
      const offset = (y * size + x) * 4;
      for (let channel = 0; channel < 4; channel += 1) {
        data[offset + channel] = sampleBilinear(source, sourceX, sourceY, channel);
      }
    }
  }
  return { width: size, height: size, data };
}

function srgbToLinear(value) {
  const c = value / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function rgbToLab(r, g, b) {
  const rl = srgbToLinear(r);
  const gl = srgbToLinear(g);
  const bl = srgbToLinear(b);
  const pivot = (value) => value > 0.008856 ? Math.cbrt(value) : 7.787 * value + 16 / 116;
  const x = pivot((0.4124564 * rl + 0.3575761 * gl + 0.1804375 * bl) / 0.95047);
  const y = pivot(0.2126729 * rl + 0.7151522 * gl + 0.072175 * bl);
  const z = pivot((0.0193339 * rl + 0.119192 * gl + 0.9503041 * bl) / 1.08883);
  return [116 * y - 16, 500 * (x - y), 200 * (y - z)];
}

const degrees = (value) => (value * 180) / Math.PI;
const radians = (value) => (value * Math.PI) / 180;

function deltaE2000(lab1, lab2) {
  const [l1, a1, b1] = lab1;
  const [l2, a2, b2] = lab2;
  const c1 = Math.hypot(a1, b1);
  const c2 = Math.hypot(a2, b2);
  const cBar = (c1 + c2) / 2;
  const g = 0.5 * (1 - Math.sqrt(cBar ** 7 / (cBar ** 7 + 25 ** 7)));
  const ap1 = (1 + g) * a1;
  const ap2 = (1 + g) * a2;
  const cp1 = Math.hypot(ap1, b1);
  const cp2 = Math.hypot(ap2, b2);
  const hp = (a, b) => {
    if (a === 0 && b === 0) return 0;
    const value = degrees(Math.atan2(b, a));
    return value >= 0 ? value : value + 360;
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
  let hpBar;
  if (cp1 * cp2 === 0) hpBar = hp1 + hp2;
  else if (Math.abs(hp1 - hp2) <= 180) hpBar = (hp1 + hp2) / 2;
  else if (hp1 + hp2 < 360) hpBar = (hp1 + hp2 + 360) / 2;
  else hpBar = (hp1 + hp2 - 360) / 2;
  const t = 1 - 0.17 * Math.cos(radians(hpBar - 30)) +
    0.24 * Math.cos(radians(2 * hpBar)) +
    0.32 * Math.cos(radians(3 * hpBar + 6)) -
    0.20 * Math.cos(radians(4 * hpBar - 63));
  const deltaTheta = 30 * Math.exp(-(((hpBar - 275) / 25) ** 2));
  const rc = 2 * Math.sqrt(cpBar ** 7 / (cpBar ** 7 + 25 ** 7));
  const sl = 1 + (0.015 * (lpBar - 50) ** 2) / Math.sqrt(20 + (lpBar - 50) ** 2);
  const sc = 1 + 0.045 * cpBar;
  const sh = 1 + 0.015 * cpBar * t;
  const rt = -Math.sin(radians(2 * deltaTheta)) * rc;
  const dl = dlp / sl;
  const dc = dcp / sc;
  const dhTerm = dh / sh;
  return Math.sqrt(dl ** 2 + dc ** 2 + dhTerm ** 2 + rt * dc * dhTerm);
}

function normalizedForMetric(image, isolateCenterComponent = true) {
  return image.normalized
    ? assertImage(image.normalized)
    : normalizeForegroundImage(image, 128, isolateCenterComponent);
}

function meanDeltaE2000(aImage, bImage, isolateCenterComponent = true) {
  const a = normalizedForMetric(aImage, isolateCenterComponent);
  const b = normalizedForMetric(bImage, isolateCenterComponent);
  const aBackground = estimateBackgroundRgb(a);
  const bBackground = estimateBackgroundRgb(b);
  let sum = 0;
  let count = 0;
  for (let offset = 0; offset < a.data.length && offset < b.data.length; offset += 4) {
    if (!isForegroundPixel(a.data, offset, aBackground) &&
        !isForegroundPixel(b.data, offset, bBackground)) continue;
    sum += deltaE2000(
      rgbToLab(a.data[offset], a.data[offset + 1], a.data[offset + 2]),
      rgbToLab(b.data[offset], b.data[offset + 1], b.data[offset + 2]),
    );
    count += 1;
  }
  return count === 0 ? Infinity : sum / count;
}

function luminanceRgb(r, g, b) {
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function percentile(values, fraction) {
  if (values.length === 0) return 0;
  return values[Math.min(values.length - 1, Math.max(0, Math.floor((values.length - 1) * fraction)))];
}

function luminanceValues(image) {
  const normalized = normalizedForMetric(image);
  const background = estimateBackgroundRgb(normalized);
  const values = [];
  for (let offset = 0; offset < normalized.data.length; offset += 4) {
    if (!isForegroundPixel(normalized.data, offset, background)) continue;
    values.push(luminanceRgb(
      normalized.data[offset],
      normalized.data[offset + 1],
      normalized.data[offset + 2],
    ));
  }
  values.sort((a, b) => a - b);
  return values;
}

function luminancePercentiles(image) {
  const values = luminanceValues(image);
  return {
    p05: percentile(values, 0.05),
    p10: percentile(values, 0.10),
    p50: percentile(values, 0.50),
    p95: percentile(values, 0.95),
    p99: percentile(values, 0.99),
  };
}

function luminanceStddev(image) {
  const values = luminanceValues(image);
  if (values.length === 0) return 0;
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  return Math.sqrt(values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / values.length);
}

function localTextureVariance(image) {
  const normalized = normalizedForMetric(image);
  const background = estimateBackgroundRgb(normalized);
  const luminance = new Float32Array(normalized.width * normalized.height);
  const foreground = new Uint8Array(normalized.width * normalized.height);
  for (let y = 0; y < normalized.height; y += 1) {
    for (let x = 0; x < normalized.width; x += 1) {
      const pixel = y * normalized.width + x;
      const offset = pixel * 4;
      luminance[pixel] = luminanceRgb(
        normalized.data[offset],
        normalized.data[offset + 1],
        normalized.data[offset + 2],
      );
      foreground[pixel] = isForegroundPixel(normalized.data, offset, background) ? 1 : 0;
    }
  }
  const residuals = [];
  const smoothingRadius = 2;
  const erosionRadius = 5;
  for (let y = erosionRadius; y < normalized.height - erosionRadius; y += 1) {
    for (let x = erosionRadius; x < normalized.width - erosionRadius; x += 1) {
      let sum = 0;
      let count = 0;
      let interior = true;
      for (let dy = -erosionRadius; dy <= erosionRadius && interior; dy += 1) {
        for (let dx = -erosionRadius; dx <= erosionRadius; dx += 1) {
          const pixel = (y + dy) * normalized.width + x + dx;
          if (!foreground[pixel]) {
            interior = false;
            break;
          }
        }
      }
      if (!interior) continue;
      for (let dy = -smoothingRadius; dy <= smoothingRadius; dy += 1) {
        for (let dx = -smoothingRadius; dx <= smoothingRadius; dx += 1) {
          sum += luminance[(y + dy) * normalized.width + x + dx];
          count += 1;
        }
      }
      residuals.push(luminance[y * normalized.width + x] - sum / count);
    }
  }
  if (residuals.length === 0) return 0;
  const mean = residuals.reduce((sum, value) => sum + value, 0) / residuals.length;
  return Math.sqrt(
    residuals.reduce((sum, value) => sum + (value - mean) ** 2, 0) / residuals.length,
  ) / 255;
}

function highlightAspectRatio(image) {
  const normalized = normalizedForMetric(image);
  const background = estimateBackgroundRgb(normalized);
  const foreground = [];
  const luminance = [];
  for (let y = 0; y < normalized.height; y += 1) {
    for (let x = 0; x < normalized.width; x += 1) {
      const offset = (y * normalized.width + x) * 4;
      if (!isForegroundPixel(normalized.data, offset, background)) continue;
      const lum = luminanceRgb(normalized.data[offset], normalized.data[offset + 1], normalized.data[offset + 2]);
      foreground.push({ x, y, lum });
      luminance.push(lum);
    }
  }
  if (foreground.length < 16) return 0;
  luminance.sort((a, b) => a - b);
  const cutoff = percentile(luminance, 0.95);
  const highlight = foreground.filter((pixel) => pixel.lum >= cutoff);
  if (highlight.length < 8) return 0;
  const weight = (pixel) => Math.max(1, pixel.lum);
  const totalWeight = highlight.reduce((sum, pixel) => sum + weight(pixel), 0);
  const meanX = highlight.reduce((sum, pixel) => sum + pixel.x * weight(pixel), 0) / totalWeight;
  const meanY = highlight.reduce((sum, pixel) => sum + pixel.y * weight(pixel), 0) / totalWeight;
  let xx = 0;
  let yy = 0;
  let xy = 0;
  for (const pixel of highlight) {
    const dx = pixel.x - meanX;
    const dy = pixel.y - meanY;
    xx += weight(pixel) * dx * dx;
    yy += weight(pixel) * dy * dy;
    xy += weight(pixel) * dx * dy;
  }
  xx /= totalWeight;
  yy /= totalWeight;
  xy /= totalWeight;
  const trace = xx + yy;
  const discriminant = Math.sqrt(Math.max(0, trace ** 2 / 4 - (xx * yy - xy ** 2)));
  return Math.sqrt((trace / 2 + discriminant) / Math.max(1e-6, trace / 2 - discriminant));
}

function darkTargetOffset(image) {
  const source = assertImage(image);
  const background = estimateBackgroundRgb(source);
  const backgroundLuminance = luminanceRgb(...background);
  let count = 0;
  let sumX = 0;
  let sumY = 0;
  for (let y = 0; y < source.height; y += 1) {
    for (let x = 0; x < source.width; x += 1) {
      const offset = (y * source.width + x) * 4;
      if (source.data[offset + 3] <= 8) continue;
      if (luminanceRgb(source.data[offset], source.data[offset + 1], source.data[offset + 2]) >=
          backgroundLuminance - 18) continue;
      count += 1;
      sumX += x;
      sumY += y;
    }
  }
  if (count === 0) return { count: 0, centerX: null, centerY: null, offsetPx: 0 };
  const centerX = sumX / count;
  const centerY = sumY / count;
  return {
    count,
    centerX,
    centerY,
    offsetPx: Math.hypot(centerX - source.width / 2, centerY - source.height / 2),
  };
}

function sobelEdgeEnergy(image, region = GLASS_TRANSMISSION_REGION) {
  const source = assertImage(image);
  const gray = new Float32Array(source.width * source.height);
  for (let offset = 0; offset < source.data.length; offset += 4) {
    gray[offset / 4] = luminanceRgb(source.data[offset], source.data[offset + 1], source.data[offset + 2]);
  }
  const minX = Math.max(1, Math.floor(source.width * region.left));
  const maxX = Math.min(source.width - 2, Math.ceil(source.width * region.right));
  const minY = Math.max(1, Math.floor(source.height * region.top));
  const maxY = Math.min(source.height - 2, Math.ceil(source.height * region.bottom));
  let sum = 0;
  let count = 0;
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const gx = -gray[(y - 1) * source.width + x - 1] + gray[(y - 1) * source.width + x + 1] -
        2 * gray[y * source.width + x - 1] + 2 * gray[y * source.width + x + 1] -
        gray[(y + 1) * source.width + x - 1] + gray[(y + 1) * source.width + x + 1];
      const gy = -gray[(y - 1) * source.width + x - 1] - 2 * gray[(y - 1) * source.width + x] -
        gray[(y - 1) * source.width + x + 1] + gray[(y + 1) * source.width + x - 1] +
        2 * gray[(y + 1) * source.width + x] + gray[(y + 1) * source.width + x + 1];
      sum += Math.hypot(gx, gy) / 255;
      count += 1;
    }
  }
  return count === 0 ? 0 : sum / count;
}

function rounded(value, digits = 3) {
  return Number(value.toFixed(digits));
}

function error(errors, code, message, details = {}) {
  errors.push({ code, message, ...details });
}

function evaluateRoundEMaterialTiles({
  surface,
  tiles,
  references = null,
  thresholds,
  requireReferenceDelta = Boolean(references),
}) {
  if (!surface) throw new Error("surface is required");
  if (!thresholds) throw new Error("thresholds are required");
  thresholds = thresholdsForSurface(thresholds, surface);
  const errors = [];
  const perMaterial = {};
  for (const preset of PRESETS) {
    if (!tiles || !tiles[preset]) {
      error(errors, "missing_live_tile", `${surface} is missing live ${preset} pixels`, { preset });
      continue;
    }
    assertImage(tiles[preset], `${surface}/${preset}`);
    const metrics = {};
    if (references && references[preset]) {
      const delta = meanDeltaE2000(
        tiles[preset],
        references[preset],
        !preset.includes("glass"),
      );
      const maximum = thresholds[preset]?.delta_e2000_max ?? thresholds.global?.reference_delta_e2000_max;
      metrics.delta_e2000_vs_reference = rounded(delta);
      metrics.delta_e2000_max = maximum;
      metrics.passed_reference_delta = Number.isFinite(delta) && delta <= maximum;
      if (requireReferenceDelta && !metrics.passed_reference_delta) {
        error(errors, "reference_delta", `${preset} DeltaE2000 ${metrics.delta_e2000_vs_reference} > ${maximum}`, { preset });
      }
    } else if (requireReferenceDelta) {
      error(errors, "missing_reference", `${surface} is missing ${preset} reference pixels`, { preset });
    }
    perMaterial[preset] = metrics;
  }

  if (tiles.chrome && perMaterial.chrome) {
    const luminance = luminancePercentiles(tiles.chrome);
    const dynamicRange = luminance.p99 / Math.max(1, luminance.p10);
    Object.assign(perMaterial.chrome, {
      luminance_p05: rounded(luminance.p05),
      luminance_p99: rounded(luminance.p99),
      specular_dynamic_range: rounded(dynamicRange),
    });
    if (!(dynamicRange >= thresholds.chrome.specular_dynamic_range)) {
      error(errors, "chrome_specular_dynamic_range", "chrome reflection dynamic range is below threshold", { preset: "chrome" });
    }
    if (!(luminance.p05 <= thresholds.chrome.dark_reflection_luminance_p05_max)) {
      error(errors, "chrome_dark_reflection", "chrome dark reflection is above threshold", { preset: "chrome" });
    }
    if (!(luminance.p99 >= thresholds.chrome.bright_reflection_luminance_p99_min)) {
      error(errors, "chrome_bright_reflection", "chrome bright reflection is below threshold", { preset: "chrome" });
    }
  }

  if (tiles.brushed_steel && perMaterial.brushed_steel) {
    const aspect = highlightAspectRatio(tiles.brushed_steel);
    perMaterial.brushed_steel.anisotropy_aspect_ratio_ibl = rounded(aspect);
    if (!(Number.isFinite(aspect) && aspect + ANISOTROPY_EPSILON >= thresholds.brushed_steel.anisotropy_aspect_ratio_ibl)) {
      error(errors, "brushed_steel_anisotropy", "brushed steel highlight is not anisotropic", { preset: "brushed_steel" });
    }
  }

  if (tiles.clearcoat_plastic && tiles.plastic && perMaterial.clearcoat_plastic) {
    const plastic = luminancePercentiles(tiles.plastic).p99;
    const clearcoat = luminancePercentiles(tiles.clearcoat_plastic).p99;
    const delta = Math.max(0, (clearcoat - plastic) / 255);
    perMaterial.clearcoat_plastic.clearcoat_lobe_delta = rounded(delta);
    if (!(delta >= thresholds.clearcoat_plastic.clearcoat_lobe_delta)) {
      error(errors, "clearcoat_lobe", "clearcoat highlight lobe is missing", { preset: "clearcoat_plastic" });
    }
  }

  for (const [preset, legacyThresholdKey, code] of [
    ["leather", "texture_variance_min", "leather_texture_variance"],
    ["rubber", "roughness_variance_min", "rubber_roughness_variance"],
  ]) {
    if (!tiles[preset] || !perMaterial[preset]) continue;
    const variance = localTextureVariance(tiles[preset]);
    const minimum = thresholds[preset].local_texture_variance_min ??
      thresholds[preset][legacyThresholdKey];
    perMaterial[preset].local_texture_variance = rounded(variance);
    perMaterial[preset].local_texture_variance_min = minimum;
    if (!(variance >= minimum)) {
      error(errors, code, `${preset} texture variance is below threshold`, { preset });
    }
  }

  if (tiles.satin && perMaterial.satin) {
    const luminance = luminancePercentiles(tiles.satin);
    const width = Math.max(0, (luminance.p95 - luminance.p50) / 255);
    perMaterial.satin.sheen_width = rounded(width);
    if (!(width >= thresholds.satin.sheen_width_min)) {
      error(errors, "satin_sheen_width", "satin sheen width is below threshold", { preset: "satin" });
    }
  }

  if (tiles.clear_glass && perMaterial.clear_glass) {
    const target = darkTargetOffset(tiles.clear_glass);
    Object.assign(perMaterial.clear_glass, {
      dark_target_pixel_count: target.count,
      refraction_offset_px: rounded(target.offsetPx),
    });
    if (!(target.count >= 64 && target.offsetPx >= thresholds.clear_glass.refraction_offset_min)) {
      error(errors, "clear_glass_refraction", "clear glass lacks measurable transmission/refraction", { preset: "clear_glass" });
    }
  }

  if (tiles.clear_glass && tiles.frosted_glass && perMaterial.frosted_glass) {
    const clearEdge = sobelEdgeEnergy(tiles.clear_glass);
    const frostedEdge = sobelEdgeEnergy(tiles.frosted_glass);
    const reduction = clearEdge > 0 ? 1 - frostedEdge / clearEdge : 0;
    Object.assign(perMaterial.frosted_glass, {
      clear_glass_edge_energy: rounded(clearEdge, 4),
      frosted_glass_edge_energy: rounded(frostedEdge, 4),
      high_frequency_contrast_reduction: rounded(reduction),
    });
    if (!(Number.isFinite(reduction) && reduction >= thresholds.frosted_glass.high_frequency_contrast_reduction_min)) {
      error(errors, "frosted_glass_transmission", "frosted glass does not reduce transmitted high-frequency contrast", { preset: "frosted_glass" });
    }
  }

  const neighborPairs = [];
  for (const [left, right] of NEIGHBOR_PAIRS) {
    if (!tiles[left] || !tiles[right]) continue;
    const delta = meanDeltaE2000(tiles[left], tiles[right]);
    const passed = Number.isFinite(delta) && delta >= thresholds.global.neighbor_delta_e2000_min;
    neighborPairs.push({ pair: [left, right], delta_e2000: rounded(delta), passed });
    if (!passed) {
      error(errors, "neighbor_delta", `${left}/${right} material neighbors are indistinguishable`, { pair: [left, right] });
    }
  }

  return {
    proof_class: "round-e-shared-material-threshold-evaluator",
    evaluator_version: 1,
    surface,
    status: errors.length === 0 ? "pass" : "fail",
    thresholds,
    per_material: perMaterial,
    neighbor_pairs: neighborPairs,
    errors,
  };
}

module.exports = {
  ANISOTROPY_EPSILON,
  GLASS_TRANSMISSION_REGION,
  NEIGHBOR_PAIRS,
  PRESETS,
  PROOF_WINDOWS,
  cropImage,
  cropRoundEMaterialTiles,
  deltaE2000,
  evaluateRoundEMaterialTiles,
  highlightAspectRatio,
  meanDeltaE2000,
  normalizeForegroundImage,
  parseThresholds,
  thresholdsForSurface,
};

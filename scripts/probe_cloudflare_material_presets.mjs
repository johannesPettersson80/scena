#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import roundEMaterialEvaluator from "./round_e_material_evaluator.cjs";
import releaseProvenance from "../tests/release/release_artifact_provenance.js";

const require = createRequire(import.meta.url);
const {
  collectBrowserGpuEvidence,
  launchHardwareBrowser,
} = require("../tests/browser/hardware_browser.js");

const {
  PRESETS: presets,
  PROOF_WINDOWS,
  evaluateRoundEMaterialTiles,
} = roundEMaterialEvaluator;
const { attachReleaseArtifactProvenance } = releaseProvenance;

const DEFAULT_URL = "https://scena-demo.pages.dev/proof/?sample=material-presets";
const url = process.argv[2] || process.env.SCENA_MATERIAL_PROOF_URL || DEFAULT_URL;
const fixturePath = "tests/visual/references/round_e_material_fixture.toml";
const thresholdsPath = "tests/visual/references/round_e_material_thresholds.toml";
const outDir = path.resolve("target/gate-artifacts/round-e-cloudflare-material-proof");
const artifactPath = path.resolve("target/gate-artifacts/round-e-cloudflare-material-proof.json");

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

const proofWindows = new Map(Object.entries(PROOF_WINDOWS));

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
  const { browser, engine } = await launchHardwareBrowser("webgl2");
  const browserGpu = await collectBrowserGpuEvidence(browser, engine).catch((error) => ({
    source: "chromium-cdp-system-info-error",
    error: error.message,
  }));
  console.error(`[cloudflare-materials] browser GPU: ${JSON.stringify(browserGpu)}`);
  const infrastructureErrors = [];
  let evaluation = {
    proof_class: "round-e-shared-material-threshold-evaluator",
    evaluator_version: 1,
    surface: "live-webgl2-chromium",
    status: "fail",
    per_material: {},
    neighbor_pairs: [],
    errors: [{ code: "evaluation_not_run", message: "live WebGL2 evaluation did not run" }],
  };
  let cacheProof = null;
  try {
    cacheProof = await cacheAndWasmProof(url);
    if (!cacheProof.bumped) {
      infrastructureErrors.push(
        `cache buster mismatch: local main ${cacheProof.local_main_buster ?? "missing"}, live main ${cacheProof.live_main_buster ?? "missing"}, local wasm ${cacheProof.local_wasm_buster ?? "missing"}, live wasm ${cacheProof.live_wasm_buster ?? "missing"}`,
      );
    }
    if (!cacheProof.wasm.checksum_matches_build) {
      infrastructureErrors.push(
        `wasm checksum mismatch: local ${cacheProof.wasm.local_sha256 ?? "missing"}, remote ${cacheProof.wasm.remote_sha256 ?? "missing"}`,
      );
    }
  } catch (error) {
    infrastructureErrors.push(`cache/wasm proof failed: ${error.message}`);
  }
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 960 } });
    const decoderPage = await browser.newPage();
    page.on("pageerror", (error) => infrastructureErrors.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() === "error") infrastructureErrors.push(`console: ${message.text()}`);
    });
    page.on("requestfailed", (request) => {
      infrastructureErrors.push(
        `requestfailed: ${request.url()} ${request.failure()?.errorText || "unknown"}`,
      );
    });
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 90000 });
    // Playwright's signature is waitForFunction(pageFunction, arg, options).
    // Passing the options object second made it the `arg`, so the intended
    // 120s budget silently fell back to Playwright's 30s default.
    await page.waitForFunction(
      () =>
        /rendered/i.test(document.getElementById("status-detail")?.textContent || "") &&
        Number(document.getElementById("metric-frame")?.textContent || "0") >= 1,
      undefined,
      { timeout: 120000 },
    );
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    const canvasBox = await page.locator("#canvas").boundingBox();
    if (!canvasBox) {
      throw new Error("material proof canvas was not visible");
    }
    const canvasPath = path.join(outDir, "canvas.png");
    await page.locator("#canvas").screenshot({ path: canvasPath });
    const liveTiles = {};
    const referenceTiles = {};
    const artifactMetadata = {};
    for (const preset of presets) {
      const cropPath = path.join(outDir, `${preset}.png`);
      const crop = await cropMaterial(page, canvasBox, preset, cropPath);
      const referencePath = fixture[preset]?.reference_path;
      if (!referencePath || !fs.existsSync(referencePath)) {
        infrastructureErrors.push(`${preset} reference missing at ${referencePath}`);
        continue;
      }
      const isolateCenterComponent = !preset.includes("glass");
      liveTiles[preset] = await readRgbaImage(decoderPage, cropPath);
      liveTiles[preset].normalized = {
        width: 128,
        height: 128,
        data: await readNormalizedForegroundRgba(decoderPage, cropPath, 128, {
          isolateCenterComponent,
        }),
      };
      referenceTiles[preset] = await readRgbaImage(decoderPage, referencePath);
      referenceTiles[preset].normalized = {
        width: 128,
        height: 128,
        data: await readNormalizedForegroundRgba(decoderPage, referencePath, 128, {
          isolateCenterComponent,
        }),
      };
      artifactMetadata[preset] = {
        crop_path: path.relative(process.cwd(), cropPath),
        reference_path: referencePath,
        reference_sha256: sha256HexFile(referencePath),
        crop_window: crop,
        reference_delta_gate: "hard",
      };
    }
    evaluation = evaluateRoundEMaterialTiles({
      surface: "live-webgl2-chromium",
      tiles: liveTiles,
      references: referenceTiles,
      thresholds,
      requireReferenceDelta: true,
    });
    for (const preset of presets) {
      evaluation.per_material[preset] = {
        ...artifactMetadata[preset],
        ...evaluation.per_material[preset],
      };
    }
    await decoderPage.close();
  } finally {
    await browser.close();
  }

  const errors = [
    ...infrastructureErrors,
    ...evaluation.errors.map((entry) => `${entry.code}: ${entry.message}`),
  ];

  const firstPreset = fixture[presets[0]] || {};
  const artifact = attachReleaseArtifactProvenance({
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
    browser_gpu: browserGpu,
    threshold_evaluator: {
      proof_class: evaluation.proof_class,
      evaluator_version: evaluation.evaluator_version,
      surface: evaluation.surface,
    },
    live_frame: {
      path: path.relative(process.cwd(), path.join(outDir, "canvas.png")),
      sha256: sha256HexFile(path.join(outDir, "canvas.png")),
    },
    thresholds,
    per_material: evaluation.per_material,
    neighbor_pairs: evaluation.neighbor_pairs,
    status: errors.length === 0 ? "passed" : "failed",
    errors,
  }, {
    root: process.cwd(),
    schema: "scena.q02.round_e_webgl2_material_proof.v1",
    producer: "node scripts/probe_cloudflare_material_presets.mjs",
    sourcePaths: [
      "scripts/probe_cloudflare_material_presets.mjs",
      "scripts/round_e_material_evaluator.cjs",
      "tests/visual/references/round_e_material_fixture.toml",
      "tests/visual/references/round_e_material_thresholds.toml",
      "tests/release/release_artifact_provenance.js",
    ],
  });
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

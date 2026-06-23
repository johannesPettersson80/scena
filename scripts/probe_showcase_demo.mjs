#!/usr/bin/env node

import { existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const baseUrl = new URL(process.argv[2] || "http://127.0.0.1:18133/");
const outDir = path.resolve("target/gate-artifacts/showcase-demo");
const connectorOnly = process.env.SCENA_SHOWCASE_CONNECTOR_ONLY === "1";
const HARDWARE_SECTION_ACTIVATION_BUDGET_MS = 800;
const CONSTRAINED_HARDWARE_SECTION_ACTIVATION_BUDGET_MS = 2000;
const SOFTWARE_SECTION_ACTIVATION_BUDGET_MS = 2000;
const configuredSectionActivationBudgetMs = process.env.SCENA_SHOWCASE_SECTION_BUDGET_MS
  ? Number(process.env.SCENA_SHOWCASE_SECTION_BUDGET_MS)
  : null;
let sectionActivationBudgetMs =
  configuredSectionActivationBudgetMs ?? HARDWARE_SECTION_ACTIVATION_BUDGET_MS;
mkdirSync(outDir, { recursive: true });

function chromiumExecutablePath() {
  if (process.env.CHROMIUM) return process.env.CHROMIUM;
  const candidates = [
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    path.join(
      process.env.HOME || "",
      ".cache/ms-playwright/chromium-1217/chrome-linux64/chrome",
    ),
  ];
  return candidates.find((candidate) => candidate && existsSync(candidate)) || candidates[0];
}

function urlFor(route) {
  return new URL(route, baseUrl).toString();
}

async function controllerSnapshot(page, scene) {
  return page.evaluate(
    (name) =>
      window.__scenaShowcaseProbe
        ?.controllers()
        ?.find((candidate) => candidate.scene === name) || null,
    scene,
  );
}

async function waitForController(page, scene, options = {}) {
  const minActivationGeneration = options.minActivationGeneration ?? -1;
  await page.waitForFunction(
    ({ name, minGeneration }) => {
      const entry = window.__scenaShowcaseProbe
        ?.controllers()
        ?.find((candidate) => candidate.scene === name);
      if (!entry?.loaded) return false;
      if (/failed|error/i.test(entry.status)) return true;
      if (!entry.active) return false;
      if (entry.activationGeneration <= minGeneration) return false;
      if (!entry.renderedForActivation) return false;
      return /rendered|assembled|mating connectors|browser-rendered WebGL2 material showcase/i.test(
        entry.status,
      );
    },
    { name: scene, minGeneration: minActivationGeneration },
    { timeout: 90000 },
  );
  const status = (await controllerSnapshot(page, scene))?.status || "";
  if (/failed|error/i.test(status)) {
    throw new Error(`${scene} controller failed: ${status}`);
  }
}

async function waitForPreparedControllers(page, scenes, timeout = 5000) {
  await page.waitForFunction(
    (requiredScenes) => {
      const controllers = window.__scenaShowcaseProbe?.controllers?.() || [];
      return requiredScenes.every((scene) => {
        const controller = controllers.find((candidate) => candidate.scene === scene);
        return controller?.loaded === true;
      });
    },
    scenes,
    { timeout },
  );
}

async function browserWebglRendererInfo(page) {
  return page.evaluate(() => {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl2") || canvas.getContext("webgl");
    if (!gl) {
      return { vendor: "", renderer: "no-webgl" };
    }
    const debug = gl.getExtension("WEBGL_debug_renderer_info");
    const loseContext = gl.getExtension("WEBGL_lose_context");
    const release = (info) => {
      loseContext?.loseContext();
      return info;
    };
    if (!debug) {
      return release({
        vendor: gl.getParameter(gl.VENDOR) || "",
        renderer: gl.getParameter(gl.RENDERER) || "",
      });
    }
    return release({
      vendor: gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) || "",
      renderer: gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) || "",
    });
  });
}

function isSoftwareWebglRenderer(info) {
  return /swiftshader|llvmpipe|software|mesa offscreen|lavapipe/i.test(
    `${info.vendor} ${info.renderer}`,
  );
}

function isConstrainedHardwareWebglRenderer(info) {
  return /broadcom|v3d/i.test(`${info.vendor} ${info.renderer}`);
}

async function configureSectionActivationBudget(page) {
  const webgl = await browserWebglRendererInfo(page);
  if (configuredSectionActivationBudgetMs == null) {
    if (isSoftwareWebglRenderer(webgl)) {
      sectionActivationBudgetMs = SOFTWARE_SECTION_ACTIVATION_BUDGET_MS;
    } else if (isConstrainedHardwareWebglRenderer(webgl)) {
      sectionActivationBudgetMs = CONSTRAINED_HARDWARE_SECTION_ACTIVATION_BUDGET_MS;
    }
  }
  return webgl;
}

async function canvasStats(page, selector) {
  return page.evaluate((canvasSelector) => {
    const canvas = document.querySelector(canvasSelector);
    if (!canvas) throw new Error(`missing canvas ${canvasSelector}`);
    const rect = canvas.getBoundingClientRect();
    const sample = document.createElement("canvas");
    sample.width = 96;
    sample.height = 96;
    const ctx = sample.getContext("2d", { willReadFrequently: true });
    ctx.drawImage(canvas, 0, 0, sample.width, sample.height);
    const image = ctx.getImageData(0, 0, sample.width, sample.height);
    const pixels = image.data;
    const cornerSamples = [];
    const cornerSize = 10;
    for (const [x0, y0] of [
      [0, 0],
      [sample.width - cornerSize, 0],
      [0, sample.height - cornerSize],
      [sample.width - cornerSize, sample.height - cornerSize],
    ]) {
      for (let y = y0; y < y0 + cornerSize; y += 1) {
        for (let x = x0; x < x0 + cornerSize; x += 1) {
          const i = (y * sample.width + x) * 4;
          cornerSamples.push([pixels[i], pixels[i + 1], pixels[i + 2]]);
        }
      }
    }
    const background = cornerSamples
      .reduce((sum, samplePixel) => {
        sum[0] += samplePixel[0];
        sum[1] += samplePixel[1];
        sum[2] += samplePixel[2];
        return sum;
      }, [0, 0, 0])
      .map((value) => value / Math.max(1, cornerSamples.length));
    let foreground = 0;
    let bright = 0;
    let luminanceSum = 0;
    let luminanceSquareSum = 0;
    const count = sample.width * sample.height;
    for (let i = 0; i < pixels.length; i += 4) {
      const r = pixels[i];
      const g = pixels[i + 1];
      const b = pixels[i + 2];
      const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
      const distance = Math.hypot(r - background[0], g - background[1], b - background[2]);
      if (distance > 14) foreground += 1;
      if (luminance > 42) bright += 1;
      luminanceSum += luminance;
      luminanceSquareSum += luminance * luminance;
    }
    const mean = luminanceSum / Math.max(1, count);
    const variance = luminanceSquareSum / Math.max(1, count) - mean * mean;
    return {
      backingWidth: canvas.width,
      backingHeight: canvas.height,
      cssWidth: rect.width,
      cssHeight: rect.height,
      renderScale: rect.width > 0 ? canvas.width / rect.width : 0,
      foregroundRatio: foreground / Math.max(1, count),
      brightRatio: bright / Math.max(1, count),
      luminanceStdDev: Math.sqrt(Math.max(0, variance)),
    };
  }, selector);
}

async function sampledCanvasPixels(page, selector, label) {
  return page.evaluate(
    ({ canvasSelector, label }) => {
      const canvas = document.querySelector(canvasSelector);
      if (!canvas) throw new Error(`missing canvas ${canvasSelector}`);
      const sample = document.createElement("canvas");
      sample.width = 128;
      sample.height = 128;
      const ctx = sample.getContext("2d", { willReadFrequently: true });
      ctx.drawImage(canvas, 0, 0, sample.width, sample.height);
      const image = ctx.getImageData(0, 0, sample.width, sample.height);
      return {
        label,
        width: sample.width,
        height: sample.height,
        rgba: Array.from(image.data),
      };
    },
    { canvasSelector: selector, label },
  );
}

function canvasDiffStats(before, after) {
  if (before.width !== after.width || before.height !== after.height) {
    throw new Error(
      `canvas snapshots have different dimensions: ${before.width}x${before.height} vs ${after.width}x${after.height}`,
    );
  }
  let changed = 0;
  let sumSq = 0;
  let maxDelta = 0;
  let changedMinX = before.width;
  let changedMinY = before.height;
  let changedMaxX = -1;
  let changedMaxY = -1;
  const pixels = before.width * before.height;
  for (let i = 0; i < before.rgba.length; i += 4) {
    const dr = after.rgba[i] - before.rgba[i];
    const dg = after.rgba[i + 1] - before.rgba[i + 1];
    const db = after.rgba[i + 2] - before.rgba[i + 2];
    const delta = Math.hypot(dr, dg, db);
    sumSq += dr * dr + dg * dg + db * db;
    maxDelta = Math.max(maxDelta, delta);
    if (delta > 24) {
      const pixel = i / 4;
      const x = pixel % before.width;
      const y = Math.floor(pixel / before.width);
      changed += 1;
      changedMinX = Math.min(changedMinX, x);
      changedMinY = Math.min(changedMinY, y);
      changedMaxX = Math.max(changedMaxX, x);
      changedMaxY = Math.max(changedMaxY, y);
    }
  }
  return {
    from: before.label,
    to: after.label,
    changedRatio: changed / Math.max(1, pixels),
    rmse: Math.sqrt(sumSq / Math.max(1, pixels * 3)),
    maxDelta,
    changedBBox:
      changed > 0
        ? {
            x: changedMinX,
            y: changedMinY,
            width: changedMaxX - changedMinX + 1,
            height: changedMaxY - changedMinY + 1,
          }
        : null,
  };
}

async function waitForConnectorStatus(page, pattern, timeout = 90000) {
  await page.waitForFunction(
    (source) => {
      const regex = new RegExp(source, "i");
      const entry = window.__scenaShowcaseProbe
        ?.controllers()
        ?.find((candidate) => candidate.scene === "connector");
      return Boolean(entry?.loaded && entry?.active && regex.test(entry.status || ""));
    },
    pattern.source,
    { timeout },
  );
}

async function assertConnectorReplayMovesCanvas(page) {
  const selector = ".stage[data-scene='connector'] canvas";
  await waitForConnectorStatus(page, /assembled/);
  const assembledBefore = await sampledCanvasPixels(page, selector, "connector-assembled-before");
  await page.locator(".stage[data-scene='connector'] .replay").click();
  await waitForConnectorStatus(page, /mating connectors/);
  await page.waitForTimeout(180);
  const replaying = await sampledCanvasPixels(page, selector, "connector-replay-in-flight");
  await waitForConnectorStatus(page, /assembled/);
  const assembledAfter = await sampledCanvasPixels(page, selector, "connector-assembled-after");
  const resetDiff = canvasDiffStats(assembledBefore, replaying);
  const finishDiff = canvasDiffStats(replaying, assembledAfter);
  const assembledDiff = canvasDiffStats(assembledBefore, assembledAfter);
  const minChangedRatio = 0.0025;
  const minRmse = 2.0;
  for (const diff of [resetDiff, finishDiff]) {
    if (diff.changedRatio < minChangedRatio || diff.rmse < minRmse) {
      throw new Error(
        `connector replay changed DOM state without moving the rendered WebGL part: ${JSON.stringify(diff)}`,
      );
    }
  }
  return {
    reset: resetDiff,
    finish: finishDiff,
    assembled_repeatability: assembledDiff,
  };
}

async function assertCanvasVisible(page, selector, label, options = {}) {
  const stats = await canvasStats(page, selector);
  const minForegroundRatio = options.minForegroundRatio ?? 0.008;
  const minLuminanceStdDev = options.minLuminanceStdDev ?? 4.0;
  const minRenderScale = options.minRenderScale ?? 1.0;
  if (stats.renderScale < minRenderScale) {
    throw new Error(
      `${label} renderScale ${stats.renderScale.toFixed(2)} < ${minRenderScale}; ${JSON.stringify(stats)}`,
    );
  }
  if (stats.foregroundRatio < minForegroundRatio || stats.luminanceStdDev < minLuminanceStdDev) {
    throw new Error(
      `${label} canvas appears blank/flat: foregroundRatio=${stats.foregroundRatio.toFixed(4)} luminanceStdDev=${stats.luminanceStdDev.toFixed(2)} stats=${JSON.stringify(stats)}`,
    );
  }
  return stats;
}

async function assertNoErrors(errors, label) {
  if (errors.length > 0) {
    throw new Error(`${label} emitted browser errors:\n${errors.join("\n")}`);
  }
}

async function activateSection(page, scene, sectionSelector, label) {
  const previous = await controllerSnapshot(page, scene);
  const startedAt = performance.now();
  await page.locator(sectionSelector).scrollIntoViewIfNeeded();
  await waitForController(page, scene, {
    minActivationGeneration: previous?.activationGeneration ?? -1,
  });
  const durationMs = performance.now() - startedAt;
  if (durationMs > sectionActivationBudgetMs) {
    throw new Error(
      `${label} activation took ${Math.round(durationMs)}ms, budget is ${sectionActivationBudgetMs}ms`,
    );
  }
  return durationMs;
}

async function wireErrorCapture(page, errors) {
  page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(`console: ${message.text()}`);
  });
}

const browser = await chromium.launch({
  headless: true,
  executablePath: chromiumExecutablePath(),
  args: [
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--ignore-gpu-blocklist",
    "--enable-gpu",
    "--use-angle=gles",
  ],
});

try {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
  });
  const errors = [];
  await wireErrorCapture(page, errors);

  await page.goto(urlFor("/"), { waitUntil: "domcontentloaded" });
  await waitForController(page, "hero");
  const webgl = await configureSectionActivationBudget(page);
  await assertCanvasVisible(page, ".stage[data-scene='hero'] canvas", "hero showcase", {
    minForegroundRatio: 0.006,
    minLuminanceStdDev: 3.0,
    minRenderScale: 1.25,
  });

  const title = await page.title();
  if (title !== "scena 1.5 live showcase") {
    throw new Error(`unexpected showcase title: ${title}`);
  }
  const sectionCount = await page.locator("main > section").count();
  if (sectionCount !== 7) {
    throw new Error(`showcase should expose 7 public sections, found ${sectionCount}`);
  }
  if ((await page.locator("#sample-list").count()) !== 0) {
    throw new Error("technical sample sidebar leaked onto the public showcase page");
  }
  if ((await page.locator("a[href='/proof/']").count()) !== 1) {
    throw new Error("public showcase must link to the technical proof harness");
  }
  await page.screenshot({ path: path.join(outDir, "root.png"), fullPage: false });
  await assertNoErrors(errors, "root showcase");
  await waitForPreparedControllers(page, ["material", "model", "connector"], 5000);

  if (connectorOnly) {
    const connectorActivationMs = await activateSection(
      page,
      "connector",
      "#connectors",
      "connector showcase",
    );
    await assertCanvasVisible(page, ".stage[data-scene='connector'] canvas", "connector showcase", {
      minForegroundRatio: 0.006,
      minLuminanceStdDev: 3.0,
      minRenderScale: 1.25,
    });
    const connectorReplayCanvasMovement = await assertConnectorReplayMovesCanvas(page);
    await page.screenshot({ path: path.join(outDir, "connectors.png"), fullPage: false });
    await assertNoErrors(errors, "connector showcase");
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "connector-only",
          outDir,
          activation_ms: {
            connector: Math.round(connectorActivationMs),
          },
          connector_replay_canvas_movement: connectorReplayCanvasMovement,
          section_activation_budget_ms: sectionActivationBudgetMs,
          webgl,
        },
        null,
        2,
      ),
    );
  } else {

    const materialActivationMs = await activateSection(page, "material", "#materials", "materials showcase");
    await assertCanvasVisible(page, ".stage[data-scene='material'] canvas", "materials showcase", {
      minForegroundRatio: 0.03,
      minLuminanceStdDev: 7.0,
      minRenderScale: 1.35,
    });
    await page.locator("[data-material='leather']").click();
    await page.waitForFunction(() => window.__scenaShowcaseProbe?.materialSelection() === "leather", {
      timeout: 30000,
    });
    await waitForController(page, "material");
    await assertCanvasVisible(page, ".stage[data-scene='material'] canvas", "materials showcase", {
      minForegroundRatio: 0.03,
      minLuminanceStdDev: 7.0,
      minRenderScale: 1.35,
    });
    const materialCode = await page.locator("#material-code").textContent();
    if (!materialCode.includes("assets.material_presets().leather().await?")) {
      throw new Error(`material code did not follow thumbnail selection: ${materialCode}`);
    }
    await page.screenshot({ path: path.join(outDir, "materials.png"), fullPage: false });
    await assertNoErrors(errors, "materials showcase");

    const modelActivationMs = await activateSection(page, "model", "#model", "model showcase");
    await assertCanvasVisible(page, ".stage[data-scene='model'] canvas", "model showcase", {
      minForegroundRatio: 0.006,
      minLuminanceStdDev: 3.0,
      minRenderScale: 1.25,
    });
    await page.screenshot({ path: path.join(outDir, "model.png"), fullPage: false });
    await assertNoErrors(errors, "model showcase");

    const connectorActivationMs = await activateSection(page, "connector", "#connectors", "connector showcase");
    await assertCanvasVisible(page, ".stage[data-scene='connector'] canvas", "connector showcase", {
      minForegroundRatio: 0.006,
      minLuminanceStdDev: 3.0,
      minRenderScale: 1.25,
    });
    const connectorReplayCanvasMovement = await assertConnectorReplayMovesCanvas(page);
    await page.screenshot({ path: path.join(outDir, "connectors.png"), fullPage: false });
    await assertNoErrors(errors, "connector showcase");

    const proofErrors = [];
    const proof = await browser.newPage({ viewport: { width: 1366, height: 820 } });
    await wireErrorCapture(proof, proofErrors);
    await proof.goto(urlFor("/proof/"), { waitUntil: "domcontentloaded" });
    await proof.waitForFunction(() => document.querySelector("#sample-list"), { timeout: 30000 });
    const proofTitle = await proof.title();
    if (proofTitle !== "scena proof harness") {
      throw new Error(`unexpected proof harness title: ${proofTitle}`);
    }
    if ((await proof.locator("#sample-list").count()) !== 1) {
      throw new Error("technical proof harness did not expose the sample list");
    }
    await proof.waitForFunction(() => /rendered|ready|select/i.test(document.body.textContent || ""), {
      timeout: 90000,
    });
    await proof.screenshot({ path: path.join(outDir, "proof.png"), fullPage: false });
    await assertNoErrors(proofErrors, "proof harness");

    console.log(
      JSON.stringify(
        {
          ok: true,
          outDir,
          activation_ms: {
            material: Math.round(materialActivationMs),
            model: Math.round(modelActivationMs),
            connector: Math.round(connectorActivationMs),
          },
          connector_replay_canvas_movement: connectorReplayCanvasMovement,
          section_activation_budget_ms: sectionActivationBudgetMs,
          webgl,
        },
        null,
        2,
      ),
    );
  }
} finally {
  await browser.close();
}

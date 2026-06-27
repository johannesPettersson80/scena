#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const { chromium } = require("playwright");

const url = process.argv[2] || "http://127.0.0.1:18104/index.html";
const outDir = path.resolve("target/gate-artifacts/cloudflare-demo");
const CANVAS_OPERATION_TIMEOUT_MS = 90000;

fs.mkdirSync(outDir, { recursive: true });
for (const file of fs.readdirSync(outDir)) {
  if (file.endsWith(".png")) fs.unlinkSync(path.join(outDir, file));
}

function assertForegroundCoverage(stats, label, minWidthFraction, minHeightFraction) {
  const rect = stats.foregroundRect;
  if (!rect) throw new Error(`${label} has no foreground pixels`);
  const widthFraction = rect.width / rect.imageWidth;
  const heightFraction = rect.height / rect.imageHeight;
  if (widthFraction < minWidthFraction || heightFraction < minHeightFraction) {
    throw new Error(
      `${label} foreground coverage is too small: ${JSON.stringify({
        rect,
        widthFraction,
        heightFraction,
      })}`,
    );
  }
  return { rect, widthFraction, heightFraction };
}

function writePngDataUrl(file, dataUrl) {
  const prefix = "data:image/png;base64,";
  if (!dataUrl.startsWith(prefix)) {
    throw new Error(`canvas did not produce a PNG data URL for ${file}`);
  }
  fs.writeFileSync(file, Buffer.from(dataUrl.slice(prefix.length), "base64"));
}

function statusFailed(status) {
  return /failed|error|panic/i.test(status || "");
}

async function showcaseControllers(page) {
  return page.evaluate(() => window.__scenaShowcaseProbe?.controllers?.() ?? []);
}

async function assertNoFailedController(page, phase) {
  const controllers = await showcaseControllers(page);
  const failed = controllers.filter((controller) => statusFailed(controller.status));
  if (failed.length) {
    throw new Error(`${phase} has failed showcase controllers: ${JSON.stringify(failed)}`);
  }
  return controllers;
}

async function waitForSceneRendered(page, scene, timeout = 90000) {
  await page.locator(`.stage[data-scene='${scene}']`).scrollIntoViewIfNeeded();
  await page.waitForFunction(
    (sceneName) => {
      const controller = window.__scenaShowcaseProbe
        ?.controllers?.()
        .find((candidate) => candidate.scene === sceneName);
      if (!controller) return false;
      if (/failed|error|panic/i.test(controller.status || "")) {
        throw new Error(`${sceneName} failed: ${controller.status}`);
      }
      return controller.active && controller.renderedForActivation;
    },
    scene,
    { timeout },
  );
  return assertNoFailedController(page, `after rendering ${scene}`);
}

async function captureSceneCanvas(page, scene, minWidthFraction, minHeightFraction) {
  const file = path.join(outDir, `${scene}-canvas.png`);
  const canvas = page.locator(`.stage[data-scene='${scene}'] canvas`);
  const capture = await canvas.evaluate(
    (source) => {
      const width = source.width;
      const height = source.height;
      const sampleWidth = Math.max(1, Math.min(320, width));
      const sampleHeight = Math.max(1, Math.min(240, height));
      const canvas = document.createElement("canvas");
      canvas.width = sampleWidth;
      canvas.height = sampleHeight;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      context.drawImage(source, 0, 0, sampleWidth, sampleHeight);
      const data = context.getImageData(0, 0, sampleWidth, sampleHeight).data;
      const bg = [data[0], data[1], data[2]];
      let sum = 0;
      let sumSq = 0;
      let minX = sampleWidth;
      let minY = sampleHeight;
      let maxX = -1;
      let maxY = -1;
      for (let y = 0; y < sampleHeight; y += 1) {
        for (let x = 0; x < sampleWidth; x += 1) {
          const i = (y * sampleWidth + x) * 4;
          const gray =
            (data[i] * 0.2126 + data[i + 1] * 0.7152 + data[i + 2] * 0.0722) / 255;
          sum += gray;
          sumSq += gray * gray;
          const delta =
            Math.abs(data[i] - bg[0]) +
            Math.abs(data[i + 1] - bg[1]) +
            Math.abs(data[i + 2] - bg[2]);
          if (data[i + 3] > 16 && delta > 18) {
            minX = Math.min(minX, x);
            minY = Math.min(minY, y);
            maxX = Math.max(maxX, x);
            maxY = Math.max(maxY, y);
          }
        }
      }
      const count = sampleWidth * sampleHeight;
      const mean = sum / count;
      const variance = Math.max(0, sumSq / count - mean * mean);
      const foregroundRect =
        maxX >= minX && maxY >= minY
          ? {
              minX,
              minY,
              maxX,
              maxY,
              width: maxX - minX + 1,
              height: maxY - minY + 1,
              imageWidth: sampleWidth,
              imageHeight: sampleHeight,
            }
          : null;
      return {
        pngDataUrl: canvas.toDataURL("image/png"),
        stats: {
          sourceWidth: width,
          sourceHeight: height,
          sampleWidth,
          sampleHeight,
          mean,
          deviation: Math.sqrt(variance),
          foregroundRect,
        },
      };
    },
    undefined,
    { timeout: CANVAS_OPERATION_TIMEOUT_MS },
  );
  const { pngDataUrl, stats } = capture;
  writePngDataUrl(file, pngDataUrl);
  if (stats.mean < 0.003 || stats.deviation < 0.002) {
    throw new Error(`${scene} canvas looks blank: ${JSON.stringify(stats)}`);
  }
  const coverage = assertForegroundCoverage(
    stats,
    `${scene} canvas`,
    minWidthFraction,
    minHeightFraction,
  );
  return { file, stats, coverage };
}

async function sampleSceneCanvasPixels(page, scene) {
  const canvas = page.locator(`.stage[data-scene='${scene}'] canvas`);
  return canvas.evaluate(
    (source) => {
      const width = source.width;
      const height = source.height;
      const sampleWidth = Math.max(1, Math.min(320, width));
      const sampleHeight = Math.max(1, Math.min(240, height));
      const scratch = document.createElement("canvas");
      scratch.width = sampleWidth;
      scratch.height = sampleHeight;
      const context = scratch.getContext("2d", { willReadFrequently: true });
      context.drawImage(source, 0, 0, sampleWidth, sampleHeight);
      const data = context.getImageData(0, 0, sampleWidth, sampleHeight).data;
      return {
        width: sampleWidth,
        height: sampleHeight,
        pixels: Array.from(data),
      };
    },
    undefined,
    { timeout: CANVAS_OPERATION_TIMEOUT_MS },
  );
}

function renderedPixelMotion(before, after) {
  if (before.width !== after.width || before.height !== after.height) {
    throw new Error(
      `cannot compare connector replay samples with different sizes: ${before.width}x${before.height} vs ${after.width}x${after.height}`,
    );
  }
  let totalDelta = 0;
  let changedPixels = 0;
  let maxDelta = 0;
  for (let index = 0; index < before.pixels.length; index += 4) {
    const delta =
      Math.abs(before.pixels[index] - after.pixels[index]) +
      Math.abs(before.pixels[index + 1] - after.pixels[index + 1]) +
      Math.abs(before.pixels[index + 2] - after.pixels[index + 2]);
    totalDelta += delta;
    maxDelta = Math.max(maxDelta, delta);
    if (delta > 24) changedPixels += 1;
  }
  return {
    changedPixels,
    meanRgbDelta: totalDelta / (before.width * before.height * 3),
    maxRgbDelta: maxDelta,
  };
}

async function assertConnectorRenderedPixelsMoveDuringReplay(page) {
  const before = await sampleSceneCanvasPixels(page, "connector");
  const replayButton = page.locator(".stage[data-scene='connector'] .replay");
  await replayButton.click();
  await page.waitForTimeout(350);
  const mid = await sampleSceneCanvasPixels(page, "connector");
  await page.waitForTimeout(350);
  const later = await sampleSceneCanvasPixels(page, "connector");
  const beforeToMid = renderedPixelMotion(before, mid);
  const midToLater = renderedPixelMotion(mid, later);
  const moved =
    beforeToMid.changedPixels >= 128 ||
    midToLater.changedPixels >= 128 ||
    beforeToMid.meanRgbDelta >= 0.08 ||
    midToLater.meanRgbDelta >= 0.08;
  if (!moved) {
    throw new Error(
      `connector replay marker motion is not enough; rendered connector pixels did not move: ${JSON.stringify({ beforeToMid, midToLater })}`,
    );
  }
  return { beforeToMid, midToLater };
}

async function assertConnectorMarkers(page) {
  await page.waitForFunction(
    () => document.querySelectorAll(".connector-marker[data-visible='true']").length >= 2,
    { timeout: 30000 },
  );
  const markers = await page.evaluate(() =>
    Array.from(document.querySelectorAll(".connector-marker")).map((marker) => ({
      connector: marker.dataset.connector,
      visible: marker.dataset.visible,
      left: marker.style.left,
      top: marker.style.top,
      rect: {
        x: marker.getBoundingClientRect().x,
        y: marker.getBoundingClientRect().y,
        width: marker.getBoundingClientRect().width,
        height: marker.getBoundingClientRect().height,
      },
    })),
  );
  for (const marker of markers) {
    if (marker.visible !== "true") {
      throw new Error(`connector marker is hidden: ${JSON.stringify(markers)}`);
    }
    if (!Number.isFinite(marker.rect.x) || !Number.isFinite(marker.rect.y)) {
      throw new Error(`connector marker is not positioned: ${JSON.stringify(markers)}`);
    }
  }
  return markers;
}

function chromiumLaunchOptions() {
  const options = {
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  };
  if (process.env.CHROMIUM) {
    options.executablePath = process.env.CHROMIUM;
  }
  return options;
}

async function fetchBytes(resourceUrl) {
  const response = await fetch(resourceUrl);
  if (!response.ok) {
    throw new Error(`failed to fetch ${resourceUrl}: HTTP ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

async function assertDeploymentBundleConsistency(pageUrl) {
  const manifestUrl = new URL("./pkg/scena_bg.wasm.size.json", pageUrl).toString();
  const wasmUrl = new URL("./pkg/scena_bg.wasm", pageUrl).toString();
  const manifest = JSON.parse((await fetchBytes(manifestUrl)).toString("utf8"));
  const wasmBytes = await fetchBytes(wasmUrl);
  const actualSha256 = crypto.createHash("sha256").update(wasmBytes).digest("hex");
  if (manifest.sha256 !== actualSha256) {
    throw new Error(
      `deployed WASM checksum mismatch: ${JSON.stringify({
        manifest: manifestUrl,
        wasm: wasmUrl,
        expected: manifest.sha256,
        actual: actualSha256,
      })}`,
    );
  }
  if (manifest.raw_bytes !== wasmBytes.length) {
    throw new Error(
      `deployed WASM byte length mismatch: ${JSON.stringify({
        manifest: manifestUrl,
        wasm: wasmUrl,
        expected: manifest.raw_bytes,
        actual: wasmBytes.length,
      })}`,
    );
  }
  return {
    manifest: manifestUrl,
    wasm: wasmUrl,
    sha256: actualSha256,
    rawBytes: wasmBytes.length,
    checksumMatchesManifest: true,
  };
}

async function assertImagesLoad(page) {
  const imageCount = await page.locator("img").count();
  for (let index = 0; index < imageCount; index += 1) {
    await page.waitForFunction(
      (i) => {
        const image = document.images[i];
        return image?.complete && image.naturalWidth > 0 && image.naturalHeight > 0;
      },
      index,
      { timeout: 30000 },
    );
  }
  return page.evaluate(() =>
    Array.from(document.images).map((image) => ({
      src: image.getAttribute("src"),
      complete: image.complete,
      naturalWidth: image.naturalWidth,
      naturalHeight: image.naturalHeight,
    })),
  );
}

(async () => {
  const errors = [];
  const deploymentBundle = await assertDeploymentBundleConsistency(url);
  const browser = await chromium.launch(chromiumLaunchOptions());
  try {
    const page = await browser.newPage({ viewport: { width: 1366, height: 820 } });
    page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(`console: ${message.text()}`);
    });

    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => window.__scenaShowcaseProbe?.controllers?.().some(
        (controller) => controller.scene === "hero" && controller.renderedForActivation,
      ),
      { timeout: 90000 },
    );
    await assertNoFailedController(page, "after hero render");

    const hero = await captureSceneCanvas(page, "hero", 0.28, 0.22);
    await waitForSceneRendered(page, "material");
    const material = await captureSceneCanvas(page, "material", 0.18, 0.18);
    await waitForSceneRendered(page, "model");
    const model = await captureSceneCanvas(page, "model", 0.20, 0.22);
    await waitForSceneRendered(page, "connector");
    const connectorMarkers = await assertConnectorMarkers(page);
    const connector = await captureSceneCanvas(page, "connector", 0.28, 0.18);

    const replayButton = page.locator(".stage[data-scene='connector'] .replay");
    if ((await replayButton.count()) !== 1 || !(await replayButton.isVisible())) {
      throw new Error("connector replay button is missing or hidden");
    }
    const connectorReplayMotion = await assertConnectorRenderedPixelsMoveDuringReplay(page);
    await page.waitForFunction(
      () => {
        const controller = window.__scenaShowcaseProbe
          ?.controllers?.()
          .find((candidate) => candidate.scene === "connector");
        return controller && !/failed|error|panic/i.test(controller.status || "");
      },
      { timeout: 30000 },
    );
    await assertConnectorMarkers(page);

    const images = await assertImagesLoad(page);
    await page.screenshot({ path: path.join(outDir, "desktop-page.png"), fullPage: true });

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => window.__scenaShowcaseProbe?.controllers?.().some(
        (controller) => controller.scene === "hero" && controller.renderedForActivation,
      ),
      { timeout: 90000 },
    );
    await waitForSceneRendered(page, "connector");
    await assertConnectorMarkers(page);
    const mobileConnector = await captureSceneCanvas(page, "connector", 0.30, 0.16);
    await page.screenshot({ path: path.join(outDir, "mobile-page.png"), fullPage: true });

    if (errors.length) throw new Error(errors.join("\n"));

    console.log(
      JSON.stringify(
        {
          url,
          deploymentBundle,
          controllers: await showcaseControllers(page),
          images,
          connectorMarkers,
          connectorReplayMotion,
          captures: {
            hero,
            material,
            model,
            connector,
            mobileConnector,
          },
          screenshots: outDir,
        },
        null,
        2,
      ),
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});

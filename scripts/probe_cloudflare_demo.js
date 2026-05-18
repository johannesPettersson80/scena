#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const childProcess = require("child_process");
const { chromium } = require("playwright");

const url = process.argv[2] || "http://127.0.0.1:18104/index.html";
const outDir = path.resolve("target/gate-artifacts/cloudflare-demo");
const rawImageMaxBuffer = 64 * 1024 * 1024;
fs.mkdirSync(outDir, { recursive: true });
for (const file of fs.readdirSync(outDir)) {
  if (file.endsWith(".png")) fs.unlinkSync(path.join(outDir, file));
}

function readRgba(file, extraArgs = []) {
  return childProcess.execFileSync(
    "magick",
    [file, ...extraArgs, "-depth", "8", "rgba:-"],
    { maxBuffer: rawImageMaxBuffer },
  );
}

function imageStats(file) {
  const mean = Number(
    childProcess.execFileSync("magick", [
      file,
      "-colorspace",
      "Gray",
      "-format",
      "%[fx:mean]",
      "info:",
    ]),
  );
  const deviation = Number(
    childProcess.execFileSync("magick", [
      file,
      "-colorspace",
      "Gray",
      "-format",
      "%[fx:standard_deviation]",
      "info:",
    ]),
  );
  return { mean, deviation };
}

function imageSize(file) {
  const output = String(childProcess.execFileSync("identify", ["-format", "%w %h", file]));
  const [width, height] = output.trim().split(/\s+/).map(Number);
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    throw new Error(`could not read image size for ${file}: ${output}`);
  }
  return { width, height };
}

function redDominantPixelsInRegion(file, region) {
  const { width, height } = imageSize(file);
  const crop = {
    x: Math.max(0, Math.floor(width * region.x)),
    y: Math.max(0, Math.floor(height * region.y)),
    width: Math.max(1, Math.floor(width * region.width)),
    height: Math.max(1, Math.floor(height * region.height)),
  };
  crop.width = Math.min(crop.width, width - crop.x);
  crop.height = Math.min(crop.height, height - crop.y);
  const rgba = readRgba(file, [
    "-crop",
    `${crop.width}x${crop.height}+${crop.x}+${crop.y}`,
    "+repage",
  ]);
  let count = 0;
  for (let i = 0; i + 3 < rgba.length; i += 4) {
    const r = rgba[i];
    const g = rgba[i + 1];
    const b = rgba[i + 2];
    const a = rgba[i + 3];
    if (a > 16 && r > 70 && r > g * 1.25 && r > b * 1.25) {
      count += 1;
    }
  }
  return { count, crop };
}

function foregroundRect(file, threshold = 18) {
  const { width, height } = imageSize(file);
  const rgba = readRgba(file);
  const bg = [rgba[0], rgba[1], rgba[2]];
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const i = (y * width + x) * 4;
      const delta =
        Math.abs(rgba[i] - bg[0]) +
        Math.abs(rgba[i + 1] - bg[1]) +
        Math.abs(rgba[i + 2] - bg[2]);
      if (rgba[i + 3] > 16 && delta > threshold) {
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }
  if (maxX < minX || maxY < minY) return null;
  return { minX, minY, maxX, maxY, width: maxX - minX + 1, height: maxY - minY + 1, imageWidth: width, imageHeight: height };
}

function assertForegroundCoverage(file, label, minWidthFraction, minHeightFraction) {
  const rect = foregroundRect(file);
  if (!rect) throw new Error(`${label} has no foreground pixels`);
  const widthFraction = rect.width / rect.imageWidth;
  const heightFraction = rect.height / rect.imageHeight;
  if (widthFraction < minWidthFraction || heightFraction < minHeightFraction) {
    throw new Error(
      `${label} foreground coverage is too small: ${JSON.stringify({ rect, widthFraction, heightFraction })}`,
    );
  }
  return rect;
}

(async () => {
  const errors = [];
  const unexpectedConsole = [];
  const browser = await chromium.launch({
    headless: true,
    executablePath: process.env.CHROMIUM || "/usr/bin/chromium",
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  try {
    const page = await browser.newPage({ viewport: { width: 1366, height: 820 } });
    page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() === "error") {
        errors.push(`console: ${message.text()}`);
      } else if (["warning", "info", "log"].includes(message.type())) {
        unexpectedConsole.push(`${message.type()}: ${message.text()}`);
      }
    });

    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => Number(document.getElementById("metric-frame")?.textContent || "0") >= 1,
      { timeout: 90000 },
    );

    const tagline = await page.locator(".brand p").textContent();
    if (
      tagline.trim() !==
      "Three.js ergonomics, Rust types, running in your browser. Drop a model or snap authored connectors."
    ) {
      throw new Error(`demo tagline is stale: ${tagline}`);
    }

    const mateSnippet = await page.locator("#code-snippet").textContent();
    if (!mateSnippet.includes('scene.mate(&drive, "shaft", &load, "hub")?;')) {
      throw new Error("default connector sample did not expose the mate snippet");
    }
    const connectorClaim = await page.locator("#connector-claim").textContent();
    if (
      !connectorClaim.includes("authored connectors") ||
      !connectorClaim.includes("no coordinates")
    ) {
      throw new Error(`connector claim does not explain authored metadata mating: ${connectorClaim}`);
    }
    if ((await page.locator('.connector-marker[data-connector="shaft"]:visible').count()) !== 1) {
      throw new Error("connector shaft marker must be visible on the default connector page");
    }
    if ((await page.locator('.connector-marker[data-connector="hub"]:visible').count()) !== 1) {
      throw new Error("connector hub marker must be visible on the default connector page");
    }
    const initialMarkers = await connectorMarkerSnapshot(page);
    assertConnectorMarkersOnCanvas(initialMarkers, "initial connector markers");
    if (Math.abs(initialMarkers.shaft.y - initialMarkers.hub.y) > 56) {
      throw new Error(
        `connector before-state markers must be horizontally separated, not vertically scattered: ${JSON.stringify(initialMarkers)}`,
      );
    }
    if (initialMarkers.hub.x - initialMarkers.shaft.x < 24) {
      throw new Error(
        `connector before-state markers must show the drive shaft left of the load hub: ${JSON.stringify(initialMarkers)}`,
      );
    }
    const connectorResult = await page.locator("#connector-result").textContent();
    if (!connectorResult.includes("Before snap")) {
      throw new Error(`connector default state must explain the disassembled before state: ${connectorResult}`);
    }
    if ((await page.locator("textarea, #code-snippet[contenteditable='true']").count()) > 0) {
      throw new Error("code panel must be a static synced display, not an editor");
    }

    const diagnostics = page.locator("#diagnostics");
    if ((await diagnostics.count()) !== 1) {
      throw new Error("diagnostics must be grouped in a collapsed details element");
    }
    if (await diagnostics.evaluate((node) => node.hasAttribute("open"))) {
      throw new Error("diagnostics must be closed by default");
    }

    const replayButton = page.locator("#replay-button");
    if ((await replayButton.count()) !== 1 || !(await replayButton.isVisible())) {
      throw new Error("Replay snap action must be visible on the connector default page");
    }

    const defaultStatus = await page.locator("#status-detail").textContent();
    if (/replay|aligned/i.test(defaultStatus || "")) {
      throw new Error("connector default page must start in the separated before state");
    }
    if (/frame\s+\d+/i.test(defaultStatus || "")) {
      throw new Error(`connector public status must not expose frame-counter text: ${defaultStatus}`);
    }

    const canvasPath = path.join(outDir, "connector-snap-canvas.png");
    await page.locator("#canvas").screenshot({ path: canvasPath });
    const pagePath = path.join(outDir, "connector-snap-page.png");
    await page.screenshot({ path: pagePath, fullPage: true });
    const connectorStats = imageStats(canvasPath);
    if (connectorStats.mean < 0.005 || connectorStats.deviation < 0.002) {
      throw new Error(`connector canvas looks blank: ${JSON.stringify(connectorStats)}`);
    }
    assertForegroundCoverage(canvasPath, "connector canvas", 0.45, 0.24);

    await replayButton.click();
    await page.waitForFunction(
      () =>
        /running scene\.mate\(\)/i.test(document.getElementById("status-detail")?.textContent || "") &&
        document.querySelector("#mate-line")?.classList.contains("is-active") &&
        Number(document.getElementById("metric-frame")?.textContent || "0") >= 4,
      { timeout: 30000 },
    );
    const replayPath = path.join(outDir, "connector-snap-replay-page.png");
    await page.screenshot({ path: replayPath, fullPage: true });
    await page.waitForFunction(
      () => /Aligned via authored connectors/.test(document.getElementById("connector-result")?.textContent || ""),
      { timeout: 30000 },
    );
    const alignedStatus = await page.locator("#status-detail").textContent();
    if (/frame\s+\d+/i.test(alignedStatus || "")) {
      throw new Error(`aligned public status must not expose frame-counter text: ${alignedStatus}`);
    }
    const postReplayPath = path.join(outDir, "connector-snap-post-replay-page.png");
    await page.screenshot({ path: postReplayPath, fullPage: true });
    const postReplayMarkers = await connectorMarkerSnapshot(page);
    assertConnectorMarkersOnCanvas(postReplayMarkers, "post-replay connector markers");

    const frameBeforeOrbit = await frameMetric(page);
    await page.mouse.move(640, 360);
    await page.mouse.down();
    await page.mouse.move(760, 420, { steps: 6 });
    await page.mouse.up();
    await page.mouse.wheel(0, -300);
    await page.waitForFunction(
      (startFrame) => Number(document.getElementById("metric-frame")?.textContent || "0") > startFrame,
      frameBeforeOrbit,
      { timeout: 30000 },
    );
    const orbitMarkers = await connectorMarkerSnapshot(page);
    assertConnectorMarkersOnCanvas(orbitMarkers, "orbited connector markers");
    const projectedOrbitMarkers = await page.evaluate(() => window.__scenaDemoProbe?.connectorMarkerPositions?.() ?? null);
    assertConnectorMarkerAnchorsMatchProjection(orbitMarkers, projectedOrbitMarkers, "orbited connector markers");
    const markerMotion =
      Math.hypot(orbitMarkers.shaft.x - postReplayMarkers.shaft.x, orbitMarkers.shaft.y - postReplayMarkers.shaft.y) +
      Math.hypot(orbitMarkers.hub.x - postReplayMarkers.hub.x, orbitMarkers.hub.y - postReplayMarkers.hub.y);
    if (markerMotion < 8) {
      throw new Error(
        `connector markers must follow camera orbit/zoom instead of staying fixed in the overlay: ${JSON.stringify({ postReplayMarkers, orbitMarkers })}`,
      );
    }
    const orbitPath = path.join(outDir, "connector-snap-orbit-page.png");
    await page.screenshot({ path: orbitPath, fullPage: true });

    const drivePath = await captureSample(page, outDir, "Drive unit", "drive-unit", "/samples/connector-snap/drive_unit.glb");
    const loadPath = await captureSample(page, outDir, "Load unit", "load-unit", "/samples/connector-snap/load_unit.glb");
    const waterBottlePaths = await captureSample(page, outDir, "Khronos PBR", "water-bottle", "/samples/khronos/WaterBottle.glb");
    const toyCarPath = await captureSample(page, outDir, "Khronos vehicle", "toy-car", "/samples/khronos/ToyCar.glb");
    const waterBottlePath = waterBottlePaths.canvasPath;
    const waterBottlePagePath = waterBottlePaths.pagePath;
    const waterBottleStats = imageStats(waterBottlePath);
    if (waterBottleStats.mean < 0.005 || waterBottleStats.deviation < 0.002) {
      throw new Error(`WaterBottle canvas looks blank: ${JSON.stringify(waterBottleStats)}`);
    }
    assertForegroundCoverage(waterBottlePath, "WaterBottle canvas", 0.25, 0.45);
    const waterBottleLogo = redDominantPixelsInRegion(waterBottlePath, {
      x: 0.25,
      y: 0.30,
      width: 0.50,
      height: 0.36,
    });
    if (waterBottleLogo.count < 350) {
      throw new Error(
        `WaterBottle default view must face the red label/logo: ${JSON.stringify(waterBottleLogo)}`,
      );
    }

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => Number(document.getElementById("metric-frame")?.textContent || "0") >= 1,
      { timeout: 90000 },
    );
    const mobilePath = path.join(outDir, "connector-snap-mobile-page.png");
    await page.screenshot({ path: mobilePath, fullPage: true });
    const mobileSize = imageSize(mobilePath);
    if (mobileSize.height > 980) {
      throw new Error(`mobile full-page screenshot has excessive dead space: ${JSON.stringify(mobileSize)}`);
    }
    const mobileCanvasPath = path.join(outDir, "connector-snap-mobile-canvas.png");
    await page.locator("#canvas").screenshot({ path: mobileCanvasPath });
    assertForegroundCoverage(mobileCanvasPath, "mobile connector canvas", 0.52, 0.24);
    if ((await page.locator("#replay-button").count()) !== 1) {
      throw new Error("mobile connector page must keep the replay action available");
    }
    const mobileReplayBox = await page.locator("#replay-button").boundingBox();
    if (!mobileReplayBox || mobileReplayBox.y > 844) {
      throw new Error(`mobile replay action must appear in the first viewport: ${JSON.stringify(mobileReplayBox)}`);
    }
    if (await page.locator("#diagnostics").evaluate((node) => node.hasAttribute("open"))) {
      throw new Error("mobile diagnostics must stay closed by default");
    }

    if (errors.length) {
      throw new Error(errors.join("\n"));
    }
    if (unexpectedConsole.length) {
      throw new Error(`public demo emitted console noise:\n${unexpectedConsole.slice(0, 20).join("\n")}`);
    }

    console.log(
      JSON.stringify(
        {
          url,
          connectorStats,
          waterBottleStats,
          screenshots: [
            pagePath,
            canvasPath,
            replayPath,
            postReplayPath,
            orbitPath,
            drivePath.pagePath,
            loadPath.pagePath,
            waterBottlePagePath,
            waterBottlePath,
            toyCarPath.pagePath,
            mobilePath,
            mobileCanvasPath,
          ],
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

async function captureSample(page, outDir, label, fileBase, expectedPath) {
  await page.getByText(label).click();
  await page.waitForFunction(
    (expected) => document.getElementById("status-title")?.textContent === expected,
    label,
    { timeout: 30000 },
  );
  await page.waitForFunction(
    () => Number(document.getElementById("metric-frame")?.textContent || "0") >= 1,
    { timeout: 90000 },
  );
  await page.waitForFunction(
    () => document.getElementById("status-detail")?.textContent === "rendered",
    { timeout: 90000 },
  );
  const subtitle = await page.locator("#code-subtitle").textContent();
  if (!subtitle.includes(expectedPath)) {
    throw new Error(`${label} code subtitle does not match canvas asset: ${subtitle}`);
  }
  const snippet = await page.locator("#code-snippet").textContent();
  if (!snippet.includes(`load_scene("${expectedPath}")`)) {
    throw new Error(`${label} code snippet does not match canvas asset`);
  }
  const canvasPath = path.join(outDir, `${fileBase}-canvas.png`);
  await page.locator("#canvas").screenshot({ path: canvasPath });
  const pagePath = path.join(outDir, `${fileBase}-page.png`);
  await page.screenshot({ path: pagePath, fullPage: true });
  const stats = imageStats(canvasPath);
  if (stats.mean < 0.005 || stats.deviation < 0.002) {
    throw new Error(`${label} canvas looks blank: ${JSON.stringify(stats)}`);
  }
  return { pagePath, canvasPath };
}

async function connectorMarkerSnapshot(page) {
  return page.evaluate(() => {
    const read = (name) => {
      const marker = document.querySelector(`.connector-marker[data-connector="${name}"]`);
      if (!marker) return null;
      const rect = marker.getBoundingClientRect();
      const canvasRect = document.getElementById("canvas").getBoundingClientRect();
      return {
        x: rect.left + rect.width * 0.5 - canvasRect.left,
        y: rect.top + rect.height * 0.5 - canvasRect.top,
        anchorX: Number.parseFloat(marker.style.left || "NaN"),
        anchorY: Number.parseFloat(marker.style.top || "NaN"),
        visible: marker.dataset.visible === "true",
      };
    };
    const canvasRect = document.getElementById("canvas").getBoundingClientRect();
    return {
      canvas: { width: canvasRect.width, height: canvasRect.height },
      shaft: read("shaft"),
      hub: read("hub"),
    };
  });
}

function assertConnectorMarkerAnchorsMatchProjection(snapshot, projected, label) {
  if (!projected) throw new Error(`${label}: missing projected marker positions`);
  for (const name of ["shaft", "hub"]) {
    const marker = snapshot[name];
    const expected = projected[name];
    if (!marker || !expected?.visible) {
      throw new Error(`${label}: ${name} missing marker/projection: ${JSON.stringify({ snapshot, projected })}`);
    }
    const distance = Math.hypot(marker.anchorX - expected.x, marker.anchorY - expected.y);
    if (!Number.isFinite(distance) || distance > 2.0) {
      throw new Error(`${label}: ${name} marker anchor detached from projected connector: ${JSON.stringify({ marker, expected, distance })}`);
    }
  }
}

async function frameMetric(page) {
  return Number(await page.locator("#metric-frame").textContent()) || 0;
}

function assertConnectorMarkersOnCanvas(snapshot, label) {
  for (const name of ["shaft", "hub"]) {
    const marker = snapshot[name];
    if (!marker || !marker.visible) {
      throw new Error(`${label}: ${name} marker is not projected and visible: ${JSON.stringify(snapshot)}`);
    }
    if (
      marker.x < 0 ||
      marker.x > snapshot.canvas.width ||
      marker.y < 0 ||
      marker.y > snapshot.canvas.height
    ) {
      throw new Error(`${label}: ${name} marker is outside the canvas: ${JSON.stringify(snapshot)}`);
    }
  }
}

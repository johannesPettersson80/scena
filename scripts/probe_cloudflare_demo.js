#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const childProcess = require("child_process");
const { chromium } = require("playwright");

const url = process.argv[2] || "http://127.0.0.1:18104/index.html";
const outDir = path.resolve("target/gate-artifacts/cloudflare-demo");
fs.mkdirSync(outDir, { recursive: true });

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
      () => /frame\s+\d+/.test(document.getElementById("status-detail")?.textContent || ""),
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
    if (/replay/i.test(defaultStatus || "")) {
      throw new Error("connector default page must render the assembled rest state before replay is clicked");
    }

    const canvasPath = path.join(outDir, "connector-snap-canvas.png");
    await page.locator("#canvas").screenshot({ path: canvasPath });
    const pagePath = path.join(outDir, "connector-snap-page.png");
    await page.screenshot({ path: pagePath, fullPage: true });
    const connectorStats = imageStats(canvasPath);
    if (connectorStats.mean < 0.005 || connectorStats.deviation < 0.002) {
      throw new Error(`connector canvas looks blank: ${JSON.stringify(connectorStats)}`);
    }

    await replayButton.click();
    await page.waitForFunction(
      () =>
        /replay/i.test(document.getElementById("status-detail")?.textContent || "") &&
        Number(document.getElementById("metric-frame")?.textContent || "0") >= 4,
      { timeout: 30000 },
    );
    const replayPath = path.join(outDir, "connector-snap-replay-page.png");
    await page.screenshot({ path: replayPath, fullPage: true });

    await page.mouse.move(640, 360);
    await page.mouse.down();
    await page.mouse.move(760, 420, { steps: 6 });
    await page.mouse.up();
    await page.mouse.wheel(0, -1);
    await page.waitForFunction(
      () => Number(document.getElementById("metric-frame")?.textContent || "0") >= 3,
      { timeout: 30000 },
    );

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

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => /frame\s+\d+/.test(document.getElementById("status-detail")?.textContent || ""),
      { timeout: 90000 },
    );
    const mobilePath = path.join(outDir, "connector-snap-mobile-page.png");
    await page.screenshot({ path: mobilePath, fullPage: true });
    if ((await page.locator("#replay-button").count()) !== 1) {
      throw new Error("mobile connector page must keep the replay action available");
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
            drivePath.pagePath,
            loadPath.pagePath,
            waterBottlePagePath,
            waterBottlePath,
            toyCarPath.pagePath,
            mobilePath,
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
    () => /frame\s+\d+/.test(document.getElementById("status-detail")?.textContent || ""),
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

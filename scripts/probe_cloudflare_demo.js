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
  const browser = await chromium.launch({
    headless: true,
    executablePath: process.env.CHROMIUM || "/usr/bin/chromium",
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  try {
    const page = await browser.newPage({ viewport: { width: 1366, height: 820 } });
    page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (
        message.type() === "error" &&
        !message.text().includes("Failed to load resource: the server responded with a status of 404")
      ) {
        errors.push(`console: ${message.text()}`);
      }
    });

    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => /frame\s+\d+/.test(document.getElementById("status-detail")?.textContent || ""),
      { timeout: 90000 },
    );

    const mateSnippet = await page.locator("#code-snippet").textContent();
    if (!mateSnippet.includes('scene.mate(&drive, "shaft", &load, "hub")?;')) {
      throw new Error("default connector sample did not expose the mate snippet");
    }

    const canvasPath = path.join(outDir, "connector-snap-canvas.png");
    await page.locator("#canvas").screenshot({ path: canvasPath });
    const pagePath = path.join(outDir, "connector-snap-page.png");
    await page.screenshot({ path: pagePath, fullPage: true });
    const connectorStats = imageStats(canvasPath);
    if (connectorStats.mean < 0.005 || connectorStats.deviation < 0.002) {
      throw new Error(`connector canvas looks blank: ${JSON.stringify(connectorStats)}`);
    }

    await page.mouse.move(640, 360);
    await page.mouse.down();
    await page.mouse.move(760, 420, { steps: 6 });
    await page.mouse.up();
    await page.mouse.wheel(0, -1);
    await page.waitForFunction(
      () => Number(document.getElementById("metric-frame")?.textContent || "0") >= 3,
      { timeout: 30000 },
    );

    await page.getByText("Khronos PBR").click();
    await page.waitForFunction(
      () => document.getElementById("status-title")?.textContent === "Khronos PBR",
      { timeout: 30000 },
    );
    await page.waitForFunction(
      () => /frame\s+\d+/.test(document.getElementById("status-detail")?.textContent || ""),
      { timeout: 90000 },
    );
    const waterBottlePath = path.join(outDir, "water-bottle-canvas.png");
    await page.locator("#canvas").screenshot({ path: waterBottlePath });
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

    if (errors.length) {
      throw new Error(errors.join("\n"));
    }

    console.log(
      JSON.stringify(
        {
          url,
          connectorStats,
          waterBottleStats,
          screenshots: [pagePath, canvasPath, waterBottlePath, mobilePath],
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

#!/usr/bin/env node

import { mkdirSync } from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const baseUrl = new URL(process.argv[2] || "http://127.0.0.1:18133/");
const outDir = path.resolve("target/gate-artifacts/showcase-demo");
mkdirSync(outDir, { recursive: true });

function urlFor(route) {
  return new URL(route, baseUrl).toString();
}

async function waitForController(page, scene) {
  await page.waitForFunction(
    (name) => {
      const entry = window.__scenaShowcaseProbe
        ?.controllers()
        ?.find((candidate) => candidate.scene === name);
      if (!entry?.loaded) return false;
      if (/failed|error/i.test(entry.status)) return true;
      return /rendered|assembled|mating connectors|browser-rendered WebGL2 material showcase/i.test(
        entry.status,
      );
    },
    scene,
    { timeout: 90000 },
  );
  const status = await page.evaluate(
    (name) =>
      window.__scenaShowcaseProbe
        ?.controllers()
        ?.find((candidate) => candidate.scene === name)?.status || "",
    scene,
  );
  if (/failed|error/i.test(status)) {
    throw new Error(`${scene} controller failed: ${status}`);
  }
}

async function assertNoErrors(errors, label) {
  if (errors.length > 0) {
    throw new Error(`${label} emitted browser errors:\n${errors.join("\n")}`);
  }
}

async function wireErrorCapture(page, errors) {
  page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(`console: ${message.text()}`);
  });
}

const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.CHROMIUM || "/usr/bin/chromium",
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});

try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  await wireErrorCapture(page, errors);

  await page.goto(urlFor("/"), { waitUntil: "domcontentloaded" });
  await waitForController(page, "hero");

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

  await page.locator("#materials").scrollIntoViewIfNeeded();
  await waitForController(page, "material");
  await page.locator("[data-material='leather']").click();
  await page.waitForFunction(() => window.__scenaShowcaseProbe?.materialSelection() === "leather", {
    timeout: 30000,
  });
  await waitForController(page, "material");
  const materialCode = await page.locator("#material-code").textContent();
  if (!materialCode.includes("assets.material_presets().leather().await?")) {
    throw new Error(`material code did not follow thumbnail selection: ${materialCode}`);
  }
  await page.screenshot({ path: path.join(outDir, "materials.png"), fullPage: false });
  await assertNoErrors(errors, "materials showcase");

  await page.locator("#model").scrollIntoViewIfNeeded();
  await waitForController(page, "model");
  await page.screenshot({ path: path.join(outDir, "model.png"), fullPage: false });
  await assertNoErrors(errors, "model showcase");

  await page.locator("#connectors").scrollIntoViewIfNeeded();
  await waitForController(page, "connector");
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

  console.log(JSON.stringify({ ok: true, outDir }, null, 2));
} finally {
  await browser.close();
}

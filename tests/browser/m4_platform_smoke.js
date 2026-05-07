const fs = require("fs");
const http = require("http");
const path = require("path");

function loadPlaywright() {
  try {
    return require("playwright");
  } catch (_) {
    return require("/home/johannes/.npm/_npx/420ff84f11983ee5/node_modules/playwright");
  }
}

function serve(root) {
  const server = http.createServer((request, response) => {
    const url = request.url === "/" ? "/m4_platform_smoke.html" : request.url;
    const file = path.join(root, path.normalize(url));
    if (!file.startsWith(root)) {
      response.writeHead(403);
      response.end("forbidden");
      return;
    }
    fs.readFile(file, (error, body) => {
      if (error) {
        response.writeHead(404);
        response.end("not found");
        return;
      }
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      response.end(body);
    });
  });

  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      resolve({
        server,
        url: `http://127.0.0.1:${server.address().port}/m4_platform_smoke.html`,
      });
    });
  });
}

async function main() {
  const { chromium } = loadPlaywright();
  const root = __dirname;
  const artifactDir = path.join(process.cwd(), "target", "gate-artifacts");
  fs.mkdirSync(artifactDir, { recursive: true });

  const { server, url } = await serve(root);
  const browser = await chromium.launch({
    headless: true,
    args: [
      "--enable-unsafe-webgpu",
      "--enable-features=Vulkan,WebGPU",
      "--ignore-gpu-blocklist",
      "--use-angle=swiftshader",
    ],
  });

  const results = [];
  try {
    for (const backend of ["webgl2", "webgpu"]) {
      const page = await browser.newPage({ viewport: { width: 96, height: 96 } });
      page.on("pageerror", (error) => {
        throw error;
      });
      await page.goto(url);
      const result = await page.evaluate((name) => window.scenaM4PlatformSmoke(name), backend);
      await page.close();
      results.push(result);
      if (result.status !== "passed") {
        throw new Error(`${backend} M4 platform smoke did not pass: ${JSON.stringify(result)}`);
      }
    }
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }

  const artifact = {
    gate: "m4-platform-browser-smoke",
    status: "passed",
    width: 64,
    height: 64,
    capabilities: results.map((result) => result.capabilities),
    loss: results.map((result) => ({ backend: result.backend, ...result.loss })),
    results,
  };
  const artifactPath = path.join(artifactDir, "m4-platform-browser-smoke.json");
  fs.writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`);
  console.log(JSON.stringify(artifact, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

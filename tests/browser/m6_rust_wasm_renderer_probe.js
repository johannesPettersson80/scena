const fs = require("fs");
const http = require("http");
const path = require("path");

function loadPlaywright() {
  return require("playwright");
}

function contentType(file) {
  if (file.endsWith(".wasm")) return "application/wasm";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".json")) return "application/json; charset=utf-8";
  if (file.endsWith(".html")) return "text/html; charset=utf-8";
  if (file.endsWith(".gltf")) return "model/gltf+json";
  return "application/octet-stream";
}

function serve(browserRoot, pkgRoot, fixtureRoot) {
  const server = http.createServer((request, response) => {
    const url = request.url === "/" ? "/m6_rust_wasm_renderer_probe.html" : request.url;
    let base = browserRoot;
    let relative = url.slice(1);
    if (url.startsWith("/pkg/")) {
      base = pkgRoot;
      relative = url.slice("/pkg/".length);
    } else if (url.startsWith("/fixtures/")) {
      base = fixtureRoot;
      relative = url.slice("/fixtures/".length);
    }
    const file = path.join(base, path.normalize(relative));
    if (!file.startsWith(base)) {
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
      response.writeHead(200, { "Content-Type": contentType(file) });
      response.end(body);
    });
  });

  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      resolve({
        server,
        url: `http://127.0.0.1:${server.address().port}/m6_rust_wasm_renderer_probe.html`,
      });
    });
  });
}

const STATE_LIFECYCLE_EVENTS = [
  "resource-lifetime",
  "idle-render-skipped",
  "dirty-transform",
  "dirty-material",
  "dirty-instance",
  "dirty-camera",
  "dirty-resize-dpr",
  "dirty-hover-selection",
  "dirty-animation-mixer",
  "context-recovery",
];

function assertStateLifecycleProbe(backend, result) {
  const events = new Set(result.event_sequence || []);
  for (const event of STATE_LIFECYCLE_EVENTS) {
    if (!events.has(event)) {
      throw new Error(
        `${backend} state lifecycle probe did not record required event ${event}: ${JSON.stringify(result)}`,
      );
    }
  }
  if (!result.resource_lifetime || result.resource_lifetime.pending_returned_to_baseline !== true) {
    throw new Error(
      `${backend} state lifecycle probe did not prove resource-lifetime baseline recovery: ${JSON.stringify(result)}`,
    );
  }
  if (
    !result.allocation_steady_state ||
    result.allocation_steady_state.idle_render_skipped !== true
  ) {
    throw new Error(
      `${backend} state lifecycle probe did not prove idle-render-skipped behavior: ${JSON.stringify(result)}`,
    );
  }
}

async function main() {
  const { chromium } = loadPlaywright();
  const browserRoot = __dirname;
  const pkgRoot = path.join(process.cwd(), "target", "m6-browser-pkg");
  const fixtureRoot = path.join(process.cwd(), "tests", "assets");
  const artifactDir = path.join(process.cwd(), "target", "gate-artifacts");
  fs.mkdirSync(artifactDir, { recursive: true });

  const { server, url } = await serve(browserRoot, pkgRoot, fixtureRoot);
  const browser = await chromium.launch({
    headless: true,
    args: [
      "--enable-unsafe-webgpu",
      "--enable-features=Vulkan,WebGPU",
      "--ignore-gpu-blocklist",
      "--use-angle=swiftshader",
    ],
  });

  const workflows = [
    "model-viewer",
    "instancing",
    "picking-selection",
    "animation",
    "labels-helpers",
    "industrial-static-scene",
    "camera-framing",
    "anchor-alignment",
    "coordinate-units",
    "static-batching",
    "layers-helper-on-top",
    "beginner-diagnostics",
    "material-textures",
    "asset-cache-reload",
  ];
  const results = [];
  try {
    for (const backend of ["webgl2", "webgpu"]) {
      const page = await browser.newPage({ viewport: { width: 96, height: 96 } });
      const consoleMessages = [];
      page.on("console", (message) => {
        consoleMessages.push(`${message.type()}: ${message.text()}`);
      });
      page.on("pageerror", (error) => {
        if (consoleMessages.length > 0) {
          error.message += `\nconsole:\n${consoleMessages.join("\n")}`;
        }
        throw error;
      });
      await page.goto(url);
      const result = await page.evaluate((name) => window.scenaM6RustWasmRendererProbe(name), backend);
      results.push(result);
      if (result.status !== "passed") {
        throw new Error(`${backend} Rust/WASM renderer probe failed: ${JSON.stringify(result)}`);
      }
      for (const workflow of workflows) {
        let workflowResult;
        try {
          workflowResult = await page.evaluate(
            ({ backend, workflow }) => window.scenaM6RustWasmWorkflowProbe(backend, workflow),
            { backend, workflow },
          );
        } catch (error) {
          throw new Error(`${backend} ${workflow}: ${error.message}`);
        }
        results.push(workflowResult);
        if (workflowResult.status !== "passed") {
          throw new Error(
            `${backend} ${workflow} Rust/WASM renderer probe failed: ${JSON.stringify(workflowResult)}`,
          );
        }
      }
      const lifecycleResult = await page.evaluate(
        (name) => window.scenaM6RustWasmLifecycleProbe(name),
        backend,
      );
      results.push(lifecycleResult);
      if (lifecycleResult.status !== "passed") {
        throw new Error(
          `${backend} surface/context lifecycle probe failed: ${JSON.stringify(lifecycleResult)}`,
        );
      }
      const benchmarkResult = await page.evaluate(
        (name) => window.scenaM6RustWasmBenchmarkProbe(name),
        backend,
      );
      results.push(benchmarkResult);
      if (benchmarkResult.status !== "passed") {
        throw new Error(
          `${backend} browser benchmark probe failed: ${JSON.stringify(benchmarkResult)}`,
        );
      }
      const stateLifecycleResult = await page.evaluate(
        (name) => window.scenaM6RustWasmStateLifecycleProbe(name),
        backend,
      );
      results.push(stateLifecycleResult);
      if (stateLifecycleResult.status !== "passed") {
        throw new Error(
          `${backend} browser state lifecycle probe failed: ${JSON.stringify(stateLifecycleResult)}`,
        );
      }
      assertStateLifecycleProbe(backend, stateLifecycleResult);
      await page.close();
    }
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }

  const artifact = {
    gate: "m6-rust-wasm-renderer-probe",
    status: "passed",
    renderer: "scena Rust/WASM",
    results,
  };
  const artifactPath = path.join(artifactDir, "m6-rust-wasm-renderer-probe.json");
  fs.writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`);
  console.log(JSON.stringify(artifact, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

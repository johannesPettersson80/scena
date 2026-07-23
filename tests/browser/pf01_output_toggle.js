const childProcess = require("child_process");
const fs = require("fs");
const http = require("http");
const path = require("path");
const { validateOutputToggleResult } = require("./pf01_output_toggle_validation.js");
const { evaluateRequiredHardwareAdapter } = require("./required_gpu_parity.js");
const {
  collectBrowserGpuEvidence,
  launchHardwareBrowser,
} = require("./hardware_browser.js");

const WIDTH = 320;
const HEIGHT = 240;
const PKG_DIR = path.join(process.cwd(), "target", "pf01-output-toggle-browser-pkg");
const ARTIFACT_DIR = path.join(process.cwd(), "target", "gate-artifacts", "pf01-output-toggle", "browser");
const ASSET_URL = "/assets/gltf/exploded_view_assembly.gltf";

function configuredBackends() {
  return (process.env.SCENA_BROWSER_BACKENDS || "webgpu,webgl2")
    .split(",")
    .map((backend) => backend.trim().toLowerCase())
    .filter(Boolean);
}

function buildPackage() {
  const command = [
    "rustup", "run", process.env.RUST_TOOLCHAIN || "1.93.1", "wasm-pack", "build", ".",
    "--dev", "--target", "web", "--out-dir", path.relative(process.cwd(), PKG_DIR),
    "--out-name", "scena", "--features", "scene-host",
  ];
  if (process.env.SCENA_SKIP_WASM_BUILD !== "1") {
    childProcess.execFileSync(command[0], command.slice(1), {
      cwd: process.cwd(),
      env: { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || "2" },
      stdio: "inherit",
    });
  }
  return command.join(" ");
}

function contentType(file) {
  if (file.endsWith(".wasm")) return "application/wasm";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".gltf")) return "model/gltf+json";
  if (file.endsWith(".bin")) return "application/octet-stream";
  return "application/octet-stream";
}

function serve() {
  const assetRoot = path.join(process.cwd(), "tests", "assets");
  const server = http.createServer((request, response) => {
    if (request.url === "/") {
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      response.end("<!doctype html><meta charset=utf-8><canvas id=scene></canvas>");
      return;
    }
    if (request.url === "/favicon.ico") {
      response.writeHead(204).end();
      return;
    }
    const mapping = request.url.startsWith("/pkg/")
      ? [PKG_DIR, request.url.slice(5)]
      : request.url.startsWith("/assets/")
        ? [assetRoot, request.url.slice(8)]
        : null;
    if (!mapping) {
      response.writeHead(404).end();
      return;
    }
    const root = path.resolve(mapping[0]);
    const file = path.resolve(root, path.normalize(mapping[1]));
    if (file !== root && !file.startsWith(`${root}${path.sep}`)) {
      response.writeHead(403).end();
      return;
    }
    fs.readFile(file, (error, body) => {
      if (error) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, { "Content-Type": contentType(file) });
      response.end(body);
    });
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      resolve({ server, url: `http://127.0.0.1:${server.address().port}/` });
    });
  });
}

async function captureBackend(page, backend) {
  return page.evaluate(async ({ backend, width, height, assetUrl }) => {
    let stage = "module import";
    const describeThrown = (error) => {
      if (error instanceof Error) {
        return `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ""}`;
      }
      const properties = {};
      try {
        for (const key of Object.getOwnPropertyNames(Object(error))) {
          try {
            properties[key] = error[key];
          } catch (propertyError) {
            properties[key] = `<unreadable: ${String(propertyError)}>`;
          }
        }
      } catch (propertyListError) {
        properties.property_list_error = String(propertyListError);
      }
      let serialized = "";
      try {
        serialized = JSON.stringify(
          properties,
          (_key, value) => typeof value === "bigint" ? value.toString() : value,
        );
      } catch (serializationError) {
        serialized = `<unserializable: ${String(serializationError)}>`;
      }
      return `${String(error)} ${serialized}`.trim();
    };
    try {
      const mark = (next) => {
        stage = next;
        console.info(`PF01 ${backend}: ${next}`);
      };

      mark(stage);
      const mod = await import("/pkg/scena.js");
      mark("Wasm initialization");
      await mod.default("/pkg/scena_bg.wasm");
      const canvas = document.createElement("canvas");
      document.body.appendChild(canvas);
      mark("SceneHost construction");
      const host = backend === "webgpu"
        ? await mod.SceneHost.newWebgpu(canvas, width, height, 1)
        : await mod.SceneHost.newWebgl2(canvas, width, height, 1);
      mark("capability report");
      const capabilityReport = JSON.parse(host.capabilitiesJson());
      mark("asset import");
      const imported = await host.instantiateUrl(assetUrl);
      const roots = host.importRoots(imported);
      if (roots.length === 0) throw new Error(`${backend} fixture produced no import root`);
      mark("scene configuration");
      host.setNodeTint(roots[0], 4.0, 2.0, 0.8, 1.0);
      host.frameNodeProductView(roots[0]);

      const nextPresent = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const hash = (bytes) => {
        let value = 0xcbf29ce484222325n;
        for (const byte of bytes) {
          value ^= BigInt(byte);
          value = (value * 0x100000001b3n) & 0xffffffffffffffffn;
        }
        return value.toString(16).padStart(16, "0");
      };
      const resourceSignature = (stats) => [
        stats.buffers, stats.gpu_textures, stats.render_targets,
        stats.pipelines, stats.bind_groups, stats.shader_modules,
      ];
      const aaEdgeMetrics = (rgba8) => {
        const luma = new Uint8Array(width * height);
        let minLuma = 255;
        let maxLuma = 0;
        let intermediateLumaPixels = 0;
        for (let index = 0, offset = 0; index < luma.length; index += 1, offset += 4) {
          const value = Math.floor(
            (rgba8[offset] * 54 + rgba8[offset + 1] * 183 + rgba8[offset + 2] * 19) / 256,
          );
          luma[index] = value;
          minLuma = Math.min(minLuma, value);
          maxLuma = Math.max(maxLuma, value);
          if (value > 8 && value < 247) intermediateLumaPixels += 1;
        }
        const lumaRange = maxLuma - minLuma;
        const hardThreshold = lumaRange / 2;
        let relativeHardTransitions = 0;
        let squaredEdgeEnergy = 0;
        for (let y = 0; y < height; y += 1) {
          for (let x = 0; x < width; x += 1) {
            const index = y * width + x;
            const neighbors = [];
            if (x + 1 < width) neighbors.push(index + 1);
            if (y + 1 < height) neighbors.push(index + width);
            for (const neighbor of neighbors) {
              const delta = Math.abs(luma[index] - luma[neighbor]);
              squaredEdgeEnergy += delta * delta;
              if (delta >= hardThreshold) relativeHardTransitions += 1;
            }
          }
        }
        return {
          intermediate_luma_pixels: intermediateLumaPixels,
          relative_hard_transitions: relativeHardTransitions,
          squared_edge_energy: squaredEdgeEnergy,
          normalized_squared_edge_energy: squaredEdgeEnergy / Math.max(1, lumaRange * lumaRange),
          luma_range: lumaRange,
        };
      };
      const capture = async (id, { fxaa, bloom }) => {
        mark(`${id}: configure output`);
        host.setAntiAliasing(fxaa ? "fxaa" : "none");
        host.setBloom(bloom
          ? JSON.stringify({ threshold_srgb: 96, intensity: 0.75, radius_px: 4 })
          : null);
        host.setAmbientOcclusion(null);
        mark(`${id}: prepare`);
        host.prepare();
        const before = JSON.parse(host.statsJson());
        mark(`${id}: render`);
        const renderOutcome = JSON.parse(host.render());
        mark(`${id}: present`);
        await nextPresent();
        const after = JSON.parse(host.statsJson());
        mark(`${id}: renderer capture`);
        const raw = await host.captureAsync();
        const descriptor = JSON.parse(raw.descriptorJson);
        const rgba8 = new Uint8Array(raw.rgba8);
        let nonblack = 0;
        let nonzeroAlpha = 0;
        let opaque = 0;
        let maxRgb = 0;
        for (let offset = 0; offset < rgba8.length; offset += 4) {
          if (rgba8[offset] || rgba8[offset + 1] || rgba8[offset + 2]) nonblack += 1;
          if (rgba8[offset + 3] !== 0) nonzeroAlpha += 1;
          if (rgba8[offset + 3] === 255) opaque += 1;
          maxRgb = Math.max(maxRgb, rgba8[offset], rgba8[offset + 1], rgba8[offset + 2]);
        }
        const diagnostic = {
          id,
          render_outcome: renderOutcome,
          descriptor_pixels: descriptor.pixels,
          rgba8_length: rgba8.length,
          nonblack,
          nonzero_alpha: nonzeroAlpha,
          opaque,
          max_rgb: maxRgb,
          fnv1a64: hash(rgba8),
          aa_edge_metrics: aaEdgeMetrics(rgba8),
        };
        console.info(`PF01 ${backend}: ${id}: pixel diagnostic ${JSON.stringify(diagnostic)}`);
        const proofCanvas = document.createElement("canvas");
        proofCanvas.width = width;
        proofCanvas.height = height;
        proofCanvas.getContext("2d").putImageData(
          new ImageData(new Uint8ClampedArray(rgba8), width, height), 0, 0,
        );
        return {
          id,
          rgba8: Array.from(rgba8),
          fnv1a64: hash(rgba8),
          pixel_source: "scene-host-capture",
          nonblack,
          aa_edge_metrics: diagnostic.aa_edge_metrics,
          diagnostic,
          resources_before_render: resourceSignature(before),
          resources_after_render: resourceSignature(after),
          png_data_url: proofCanvas.toDataURL("image/png"),
        };
      };

      const off = await capture("off", { fxaa: false, bloom: false });
      const bloomOnly = await capture("bloom_only", { fxaa: false, bloom: true });
      const fxaaOnly = await capture("fxaa_only", { fxaa: true, bloom: false });
      const on = await capture("on", { fxaa: true, bloom: true });
      const offAgain = await capture("off_again", { fxaa: false, bloom: false });
      mark("capture complete");
      return {
        backend,
        capability_report: capabilityReport,
        phases: {
          off,
          bloom_only: bloomOnly,
          fxaa_only: fxaaOnly,
          on,
          off_again: offAgain,
        },
      };
    } catch (error) {
      throw new Error(`${backend} failed during ${stage}: ${describeThrown(error)}`);
    }
  }, { backend, width: WIDTH, height: HEIGHT, assetUrl: ASSET_URL });
}

function writeArtifacts(results, buildCommand, requiredHardware, completeBackendSet) {
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
  const compact = results.map((result) => {
    for (const phase of Object.values(result.phases)) {
      const png = Buffer.from(phase.png_data_url.split(",", 2)[1], "base64");
      fs.writeFileSync(path.join(ARTIFACT_DIR, `${result.backend}-${phase.id}.png`), png);
      delete phase.png_data_url;
      delete phase.rgba8;
    }
    return result;
  });
  const report = {
    schema: "scena.pf01.browser_output_toggle.v1",
    generated_at: new Date().toISOString(),
    status: "passed",
    release_evidence: requiredHardware && completeBackendSet,
    build_command: buildCommand,
    required_hardware: requiredHardware,
    complete_backend_set: completeBackendSet,
    asset_url: ASSET_URL,
    backends: compact,
  };
  fs.writeFileSync(
    path.join(ARTIFACT_DIR, "browser-output-toggle.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  process.stdout.write(`${JSON.stringify(report)}\n`);
}

async function main() {
  const backends = configuredBackends();
  const completeBackendSet =
    backends.length === 2 && backends.includes("webgpu") && backends.includes("webgl2");
  const allowPartial = process.env.SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS === "1";
  if (!completeBackendSet && !allowPartial) {
    throw new Error("PF01 hardware proof requires SCENA_BROWSER_BACKENDS=webgpu,webgl2");
  }
  const requiredHardware = process.env.SCENA_REQUIRE_PARITY === "1";
  const buildCommand = buildPackage();
  const { server, url } = await serve();
  try {
    const results = [];
    for (const backend of backends) {
      const { browser, engine } = await launchHardwareBrowser(backend);
      try {
        process.stderr.write(`PF01 ${backend} (${engine}): start\n`);
        const browserGpu = await collectBrowserGpuEvidence(browser, engine);
        const page = await browser.newPage();
        const httpFailures = [];
        page.on("console", (message) => process.stderr.write(`${message.text()}\n`));
        page.on("response", (response) => {
          if (response.status() >= 400) {
            httpFailures.push({ kind: "response", status: response.status(), url: response.url() });
          }
        });
        page.on("requestfailed", (request) => {
          httpFailures.push({
            kind: "requestfailed",
            url: request.url(),
            failure: request.failure()?.errorText || "unknown request failure",
          });
        });
        await page.goto(url);
        const result = await captureBackend(page, backend);
        validateOutputToggleResult(result);
        if (httpFailures.length !== 0) {
          throw new Error(`${backend} unexpected HTTP failures: ${JSON.stringify(httpFailures)}`);
        }
        result.browser_engine = engine;
        result.browser_gpu = browserGpu;
        const capabilities = result.capability_report.capabilities || {};
        result.hardware_evidence = evaluateRequiredHardwareAdapter({
          required: requiredHardware,
          requestedBackend: backend,
          actualBackend: capabilities.backend,
          gpuDevice: capabilities.gpu_device,
          surfaceAttached: capabilities.surface_attached,
          adapter: result.capability_report.adapter,
          browserGpu,
        });
        if (result.hardware_evidence.status === "failed") {
          throw new Error(
            `${backend} required hardware proof failed: ${JSON.stringify({
              hardware_evidence: result.hardware_evidence,
              browser_gpu: browserGpu,
              capability_report: result.capability_report,
            })}`,
          );
        }
        process.stderr.write(`PF01 ${backend} (${engine}): passed\n`);
        results.push(result);
        await page.close();
      } finally {
        await browser.close();
      }
    }
    writeArtifacts(results, buildCommand, requiredHardware, completeBackendSet);
  } finally {
    server.close();
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exitCode = 1;
});

const childProcess = require("child_process");
const fs = require("fs");
const http = require("http");
const path = require("path");
const { evaluateRequiredHardwareAdapter } = require("./required_gpu_parity.js");
const {
  collectBrowserGpuEvidence,
  launchHardwareBrowser,
} = require("./hardware_browser.js");

const SCHEMA = "scena.fr06_semantic_aov_browser_proof.v1";
const WIDTH = 320;
const HEIGHT = 240;
const PKG_DIR = path.join(process.cwd(), "target", "fr06-semantic-aov-browser-pkg");
const ARTIFACT_DIR = path.join(
  process.cwd(),
  "target",
  "gate-artifacts",
  "fr06-semantic-aov",
  "browser",
);
const ASSET_URL = "/assets/gltf/mesh_material_vertex_color_scene.gltf";

function configuredBackends() {
  return (process.env.SCENA_BROWSER_BACKENDS || "webgpu,webgl2")
    .split(",")
    .map((backend) => backend.trim().toLowerCase())
    .filter(Boolean);
}

function buildPackage() {
  const command = [
    "rustup",
    "run",
    process.env.RUST_TOOLCHAIN || "1.93.1",
    "wasm-pack",
    "build",
    ".",
    "--dev",
    "--target",
    "web",
    "--out-dir",
    path.relative(process.cwd(), PKG_DIR),
    "--out-name",
    "scena",
    "--features",
    "scene-host",
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
  if (file.endsWith(".png")) return "image/png";
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
    let root;
    let relative;
    if (request.url.startsWith("/pkg/")) {
      root = PKG_DIR;
      relative = request.url.slice(5);
    } else if (request.url.startsWith("/assets/")) {
      root = assetRoot;
      relative = request.url.slice(8);
    } else {
      response.writeHead(404).end();
      return;
    }
    const resolvedRoot = path.resolve(root);
    const file = path.resolve(resolvedRoot, path.normalize(relative));
    if (file !== resolvedRoot && !file.startsWith(`${resolvedRoot}${path.sep}`)) {
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
  return page.evaluate(
    async ({ backend, width, height, assetUrl }) => {
      let stage = "module import";
      try {
        const mark = (next) => {
          stage = next;
          console.info(`FR06 ${backend}: ${next}`);
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
        const capabilityReport = JSON.parse(host.capabilitiesJson());
        if (typeof host.setSemanticAovCaptureEnabled !== "function"
            || typeof host.captureSemanticAovs !== "function") {
          throw new Error(`${backend} semantic AOV bindings are missing`);
        }
        mark("semantic AOV enable");
        host.setSemanticAovCaptureEnabled(true);
        mark("asset import");
        const imported = await host.instantiateUrl(assetUrl);
        const roots = host.importRoots(imported);
        mark("camera framing");
        if (roots.length > 0) host.frameNodeProductView(roots[0]);
        mark("prepare");
        host.prepare();
        mark("beauty render");
        const renderOutcome = JSON.parse(host.render());
        mark("beauty capture");
        const beautyCapture = await host.captureAsync();
        const beautyDescriptor = JSON.parse(beautyCapture.descriptorJson);
        const beautyRgba8 = new Uint8Array(beautyCapture.rgba8);
        mark("first capture");
        const first = await host.captureSemanticAovs();
        mark("repeat capture");
        const second = await host.captureSemanticAovs();
        mark("capture validation");
        const metadata = JSON.parse(first.metadataJson);
        const ids = Array.from(first.idIndices);
        const repeatedIds = Array.from(second.idIndices);
        if (first.beautyIdIndices === null || second.beautyIdIndices === null) {
          throw new Error(`${backend} capture is missing the same-pass beauty semantic witness`);
        }
        const beautyIds = Array.from(first.beautyIdIndices);
        const repeatedBeautyIds = Array.from(second.beautyIdIndices);
        const depths = Array.from(first.depthMeters);
        const normals = Array.from(first.worldNormals);
        const idRgba8 = new Uint8ClampedArray(first.idRgba8);
      if (ids.length !== width * height || depths.length !== width * height) {
        throw new Error(`${backend} AOV dimensions do not match metadata`);
      }
      if (beautyIds.length !== ids.length) {
        throw new Error(`${backend} beauty witness dimensions do not match the AOV`);
      }
      if (normals.length !== width * height * 3) {
        throw new Error(`${backend} normal payload has the wrong length`);
      }
      if (!ids.some((id) => id !== 0)) {
        throw new Error(`${backend} ID AOV contains no attributed hit`);
      }
      if (!ids.every((id, index) => id === repeatedIds[index])) {
        throw new Error(`${backend} repeated ID capture is not deterministic`);
      }
      if (!beautyIds.every((id, index) => id === repeatedBeautyIds[index])) {
        throw new Error(`${backend} repeated beauty witness is not deterministic`);
      }
      const hitCount = ids.filter((id) => id !== 0).length;
      const beautyHitCount = beautyIds.filter((id) => id !== 0).length;
      const beautyMatchCount = ids.filter(
        (id, index) => id !== 0 && beautyIds[index] === id,
      ).length;
      const samePassBeautyAgreement = beautyMatchCount / Math.max(1, hitCount);
      if (beautyHitCount === 0 || samePassBeautyAgreement < 0.8) {
        throw new Error(
          `${backend} same-pass beauty agreement failed: `
          + `${beautyMatchCount}/${hitCount} (${samePassBeautyAgreement})`,
        );
      }
      const uniqueBeautyRgba = new Set();
      let beautyNonblack = 0;
      for (let offset = 0; offset < beautyRgba8.length; offset += 4) {
        uniqueBeautyRgba.add(
          `${beautyRgba8[offset]},${beautyRgba8[offset + 1]},`
          + `${beautyRgba8[offset + 2]},${beautyRgba8[offset + 3]}`,
        );
        if (beautyRgba8[offset] || beautyRgba8[offset + 1] || beautyRgba8[offset + 2]) {
          beautyNonblack += 1;
        }
      }
      const beautyDiagnostic = {
        descriptor: {
          width: beautyDescriptor.width,
          height: beautyDescriptor.height,
          pixel_format: beautyDescriptor.pixel_format,
          payload: beautyDescriptor.payload,
          pixels: beautyDescriptor.pixels,
        },
        rgba8_length: beautyRgba8.length,
        nonblack_pixels: beautyNonblack,
        unique_rgba_count: uniqueBeautyRgba.size,
        render_outcome: renderOutcome,
        witness_hit_count: beautyHitCount,
        witness_matching_aov_hits: beautyMatchCount,
        witness_agreement: samePassBeautyAgreement,
      };
      console.info(`FR06 ${backend}: beauty diagnostic ${JSON.stringify(beautyDiagnostic)}`);
      if (beautyDescriptor.width !== width
          || beautyDescriptor.height !== height
          || beautyRgba8.length !== width * height * 4
          || beautyNonblack === 0
          || uniqueBeautyRgba.size < 4) {
        throw new Error(
          `${backend} beauty frame is empty or structurally flat: `
          + JSON.stringify(beautyDiagnostic),
        );
      }
      const finiteDepthCount = depths.filter((depth, index) => ids[index] !== 0 && Number.isFinite(depth)).length;
      const proofCanvas = document.createElement("canvas");
      proofCanvas.width = width;
      proofCanvas.height = height;
      proofCanvas.getContext("2d").putImageData(new ImageData(idRgba8, width, height), 0, 0);
        return {
          backend,
          metadata,
          ids,
          depths,
          normals,
          hit_count: hitCount,
          finite_depth_count: finiteDepthCount,
          deterministic_repeat: true,
          render_outcome: renderOutcome,
          beauty_frame: {
            nonblack_pixels: beautyNonblack,
            unique_rgba_count: uniqueBeautyRgba.size,
          },
          beauty_witness: {
            hit_count: beautyHitCount,
            matching_aov_hits: beautyMatchCount,
            same_pass_beauty_agreement: samePassBeautyAgreement,
            deterministic_repeat: true,
          },
          capability_report: capabilityReport,
          id_png_data_url: proofCanvas.toDataURL("image/png"),
        };
      } catch (error) {
        const details = error instanceof Error
          ? `${error.name}: ${error.message}`
          : JSON.stringify(error);
        throw new Error(`${backend} failed during ${stage}: ${details}`);
      }
    },
    { backend, width: WIDTH, height: HEIGHT, assetUrl: ASSET_URL },
  );
}

function compare(left, right) {
  let maskMatches = 0;
  let commonHits = 0;
  let identityMatches = 0;
  let maxDepthError = 0;
  let minNormalDot = 1;
  let firstCommonSample = null;
  for (let index = 0; index < left.ids.length; index += 1) {
    const leftHit = left.ids[index] !== 0;
    const rightHit = right.ids[index] !== 0;
    if (leftHit === rightHit) maskMatches += 1;
    if (!leftHit || !rightHit) continue;
    commonHits += 1;
    if (firstCommonSample === null) {
      const offset = index * 3;
      firstCommonSample = {
        index,
        left_id: left.ids[index],
        right_id: right.ids[index],
        left_depth: left.depths[index],
        right_depth: right.depths[index],
        left_normal: left.normals.slice(offset, offset + 3),
        right_normal: right.normals.slice(offset, offset + 3),
      };
    }
    if (left.ids[index] === right.ids[index]) identityMatches += 1;
    maxDepthError = Math.max(maxDepthError, Math.abs(left.depths[index] - right.depths[index]));
    const offset = index * 3;
    const dot = left.normals[offset] * right.normals[offset]
      + left.normals[offset + 1] * right.normals[offset + 1]
      + left.normals[offset + 2] * right.normals[offset + 2];
    minNormalDot = Math.min(minNormalDot, dot);
  }
  const result = {
    mask_agreement: maskMatches / left.ids.length,
    identity_agreement_on_common_hits: commonHits === 0 ? 0 : identityMatches / commonHits,
    max_depth_error_meters: maxDepthError,
    min_normal_dot: minNormalDot,
    common_hits: commonHits,
    first_common_sample: firstCommonSample,
  };
  if (result.mask_agreement < 0.98
      || result.identity_agreement_on_common_hits < 0.995
      || result.max_depth_error_meters > 0.005
      || result.min_normal_dot < 0.98) {
    throw new Error(`FR06 WebGPU/WebGL2 parity failed: ${JSON.stringify(result)}`);
  }
  return result;
}

function writeArtifacts(results, parity, buildCommand, requiredHardware) {
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
  for (const result of results) {
    const png = Buffer.from(result.id_png_data_url.split(",", 2)[1], "base64");
    fs.writeFileSync(path.join(ARTIFACT_DIR, `${result.backend}-id.png`), png);
  }
  const compactResults = results.map(({ ids, depths, normals, id_png_data_url, ...result }) => result);
  const report = {
    schema: SCHEMA,
    generated_at: new Date().toISOString(),
    build_command: buildCommand,
    required_hardware: requiredHardware,
    complete_backend_set:
      results.length === 2
      && results.some((result) => result.backend === "webgpu")
      && results.some((result) => result.backend === "webgl2"),
    release_evidence:
      requiredHardware
      && results.length === 2
      && results.some((result) => result.backend === "webgpu")
      && results.some((result) => result.backend === "webgl2"),
    backends: compactResults,
    parity,
  };
  fs.writeFileSync(
    path.join(ARTIFACT_DIR, "semantic-aov-browser-proof.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  return report;
}

async function main() {
  const backends = configuredBackends();
  const requiredHardware = process.env.SCENA_REQUIRE_PARITY === "1";
  if (backends.some((backend) => !["webgpu", "webgl2"].includes(backend))) {
    throw new Error(`unsupported FR06 backend selection: ${backends.join(",")}`);
  }
  const buildCommand = buildPackage();
  const { server, url } = await serve();
  try {
    const results = [];
    for (const backend of backends) {
      const { browser, engine } = await launchHardwareBrowser(backend);
      try {
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
            error: request.failure() ? request.failure().errorText : "unknown",
          });
        });
        await page.goto(url);
        const result = await captureBackend(page, backend);
        if (httpFailures.length !== 0) {
          throw new Error(`${backend} unexpected HTTP failures: ${JSON.stringify(httpFailures)}`);
        }
        result.browser_engine = engine;
        result.browser_gpu = browserGpu;
        const capabilityReport = result.capability_report || {};
        const capabilities = capabilityReport.capabilities || {};
        result.hardware_evidence = evaluateRequiredHardwareAdapter({
          required: requiredHardware,
          requestedBackend: backend,
          actualBackend: capabilities.backend,
          gpuDevice: capabilities.gpu_device,
          surfaceAttached: capabilities.surface_attached,
          adapter: capabilityReport.adapter,
          browserGpu,
        });
        if (result.hardware_evidence.status === "failed") {
          throw new Error(
            `FR06 ${backend} required hardware proof failed: ${JSON.stringify({
              hardware_evidence: result.hardware_evidence,
              browser_gpu: browserGpu,
            })}`,
          );
        }
        results.push(result);
        await page.close();
      } finally {
        await browser.close();
      }
    }
    const parity = results.length === 2 ? compare(results[0], results[1]) : null;
    const report = writeArtifacts(results, parity, buildCommand, requiredHardware);
    process.stdout.write(`${JSON.stringify(report)}\n`);
  } finally {
    server.close();
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exitCode = 1;
});

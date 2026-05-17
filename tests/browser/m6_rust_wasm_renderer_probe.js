const fs = require("fs");
const crypto = require("crypto");
const http = require("http");
const path = require("path");

const MODEL_VIEWER_FIXTURE = "/fixtures/gltf/non_ndc_camera_scene.gltf";

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

function configuredBackends() {
  return (process.env.SCENA_BROWSER_BACKENDS || "webgl2,webgpu")
    .split(",")
    .map((backend) => backend.trim())
    .filter(Boolean);
}

function chromiumLaunchArgs(backends) {
  const args = [
    "--enable-unsafe-webgpu",
    "--enable-features=Vulkan,WebGPU",
    "--ignore-gpu-blocklist",
  ];
  if (!backends.includes("webgpu")) {
    args.push("--use-angle=swiftshader");
  }
  return args;
}

function unavailableResult(backend, error) {
  return {
    backend,
    status: "unavailable",
    error: String(error && error.message ? error.message : error),
  };
}

function fixturePath(fixtureRoot, source) {
  if (!source || !source.startsWith("/fixtures/")) {
    throw new Error(`fixture source must use /fixtures/ prefix, got ${source}`);
  }
  const root = path.resolve(fixtureRoot);
  const relative = path.normalize(source.slice("/fixtures/".length));
  const file = path.resolve(root, relative);
  if (!file.startsWith(`${root}${path.sep}`)) {
    throw new Error(`fixture source escapes fixture root: ${source}`);
  }
  return file;
}

function fixtureSha256(fixtureRoot, source) {
  return crypto.createHash("sha256").update(fs.readFileSync(fixturePath(fixtureRoot, source))).digest("hex");
}

function attachFixtureHash(fixtureRoot, result) {
  const source = result.metadata && result.metadata.source;
  if (!source) {
    return;
  }
  const fixture_sha256 = fixtureSha256(fixtureRoot, source);
  result.fixture_sha256 = fixture_sha256;
  result.screenshot_metadata = result.screenshot_metadata || {};
  result.screenshot_metadata.fixture_sha256 = fixture_sha256;
}

function isAllowedUnavailable(backend, error) {
  if (process.env.SCENA_BROWSER_ALLOW_UNAVAILABLE !== "1") {
    return false;
  }
  const message = String(error && error.message ? error.message : error);
  if (backend !== "webgpu") {
    return false;
  }
  if (message.includes("NoAdapter")) {
    return true;
  }
  return (
    message.includes('"status":"failed"') &&
    message.includes('"gpu_device":true') &&
    message.includes('"nonblack":0')
  );
}

function assertNoScenaGpuValidationErrors(backend, consoleMessages) {
  const validationErrors = consoleMessages.filter(
    (message) =>
      message.includes("scena wgpu uncaptured error") ||
      message.includes("Error while parsing WGSL") ||
      message.includes("Invalid ShaderModule") ||
      message.includes("Invalid RenderPipeline"),
  );
  if (validationErrors.length > 0) {
    throw new Error(
      `${backend} browser GPU validation errors were reported:\n${validationErrors.join("\n")}`,
    );
  }
}

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

function assertSurfaceLifecycleProbe(backend, result) {
  const events = new Set(result.event_sequence || []);
  for (const event of [
    "context-lost",
    "context-restored",
    "recover-context",
    "render-after-context-recovery",
    "device-lost",
    "recover-device",
    "render-after-device-recovery",
    "final-render",
  ]) {
    if (!events.has(event)) {
      throw new Error(
        `${backend} surface lifecycle probe did not record ${event}: ${JSON.stringify(result)}`,
      );
    }
  }
  if (!result.stats || result.stats.material_texture_bindings < 5 || result.stats.textures < 2) {
    throw new Error(
      `${backend} surface lifecycle probe did not recover a textured material scene: ${JSON.stringify(result)}`,
    );
  }
  if (
    !result.context_recovered ||
    result.context_recovered.draw_calls <= 0 ||
    !result.device_recovered ||
    result.device_recovered.draw_calls <= 0
  ) {
    throw new Error(
      `${backend} surface lifecycle probe did not render after context/device recovery: ${JSON.stringify(result)}`,
    );
  }
}

function assertModelViewerProof(backend, result) {
  const metadata = result.metadata || {};
  if (metadata.source !== MODEL_VIEWER_FIXTURE) {
    throw new Error(
      `${backend} model-viewer proof used unexpected fixture ${metadata.source}: ${JSON.stringify(result)}`,
    );
  }
  if (metadata.proof_class !== "camera-framed-non-ndc" || metadata.framed !== true) {
    throw new Error(
      `${backend} model-viewer proof did not record camera-framed non-NDC metadata: ${JSON.stringify(result)}`,
    );
  }
  if (!/^[0-9a-f]{64}$/.test(result.fixture_sha256 || "")) {
    throw new Error(`${backend} model-viewer proof did not record fixture_sha256`);
  }
  if (
    !result.screenshot_metadata ||
    result.screenshot_metadata.fixture_sha256 !== result.fixture_sha256 ||
    result.screenshot_metadata.backend !== backend ||
    result.screenshot_metadata.workflow !== "model-viewer" ||
    result.screenshot_metadata.width <= 0 ||
    result.screenshot_metadata.height <= 0 ||
    result.screenshot_metadata.device_pixel_ratio <= 0
  ) {
    throw new Error(
      `${backend} model-viewer proof did not include complete screenshot_metadata: ${JSON.stringify(result)}`,
    );
  }
  if (
    typeof result.canvas_data_url !== "string" ||
    !result.canvas_data_url.startsWith("data:image/png;base64,") ||
    result.canvas_data_url.length < 100
  ) {
    throw new Error(`${backend} model-viewer proof did not capture a PNG canvas data URL`);
  }
  if (!result.pixels || result.pixels.nonblack <= 0 || !result.screenshot_metadata.pixel_statistics) {
    throw new Error(
      `${backend} model-viewer proof did not include nonblack pixel statistics: ${JSON.stringify(result)}`,
    );
  }
}

function assertDepthOverlapProof(backend, result) {
  const metadata = result.metadata || {};
  const center = result.pixels && result.pixels.center;
  if (metadata.proof_class !== "depth-overlap-near-wins" || !Array.isArray(center)) {
    throw new Error(
      `${backend} depth-overlap proof did not record required metadata and center pixel: ${JSON.stringify(result)}`,
    );
  }
  if (center[1] <= center[0] + 20) {
    throw new Error(
      `${backend} depth-overlap proof did not keep the nearer green triangle visible over later red geometry: ${JSON.stringify(result)}`,
    );
  }
}

function assertMaterialTextureProof(backend, result) {
  const metadata = result.metadata || {};
  if (
    metadata.decoded_base_color_texture !== true ||
    metadata.decoded_normal_texture !== true ||
    metadata.decoded_emissive_texture !== true ||
    metadata.texture_transform !== true
  ) {
    throw new Error(
      `${backend} material-textures proof did not use decoded Rust/WASM texture pixels: ${JSON.stringify(result)}`,
    );
  }
  if (!result.stats || result.stats.material_texture_bindings < 5) {
    throw new Error(
      `${backend} material-textures proof did not report material texture bindings: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0) {
    throw new Error(
      `${backend} material-textures proof did not render visible material pixels: ${JSON.stringify(result)}`,
    );
  }
}

function assertSourceGltfMaterialProof(backend, result) {
  const metadata = result.metadata || {};
  const pixels = result.pixels || {};
  const nonblack = (pixel) => Array.isArray(pixel) && (pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0);
  const diagnostics = result.diagnostics || [];
  if (
    metadata.proof_class !== "browser-source-gltf-material-comparison" ||
    metadata.construction !== "SceneAsset::nodes mesh.geometry mesh.material" ||
    metadata.source_base_color_decoded !== true ||
    metadata.source_texture_bindings < 1 ||
    metadata.load_warnings !== 0
  ) {
    throw new Error(
      `${backend} source-gltf-materials proof did not load decoded source material handles cleanly: ${JSON.stringify(result)}`,
    );
  }
  if (
    !result.stats ||
    result.stats.material_texture_bindings < 1 ||
    result.stats.material_textures_missing_decoded_pixels !== 0
  ) {
    throw new Error(
      `${backend} source-gltf-materials proof reported missing texture pixels or no material texture binding: ${JSON.stringify(result)}`,
    );
  }
  if (
    diagnostics.some((diagnostic) => diagnostic.code === "MaterialTextureMissingDecodedPixels")
  ) {
    throw new Error(
      `${backend} source-gltf-materials proof emitted missing-decoded-pixels diagnostics: ${JSON.stringify(result)}`,
    );
  }
  if (!nonblack(pixels.left) || !nonblack(pixels.center) || !nonblack(pixels.right)) {
    throw new Error(
      `${backend} source-gltf-materials did not render visible unlit/source/PBR comparison lanes: ${JSON.stringify(result)}`,
    );
  }
}

function assertPunctualLightProof(backend, result, channel, workflow) {
  const metadata = result.metadata || {};
  const center = result.pixels && result.pixels.center;
  const channelIndex = { red: 0, green: 1, blue: 2 }[channel];
  if (
    metadata.proof_class !== "browser-pbr-punctual-light" ||
    metadata.light_kind !== channel ||
    metadata.material_kind !== "pbr-metallic-roughness" ||
    !Array.isArray(center)
  ) {
    throw new Error(
      `${backend} ${workflow} proof did not record PBR punctual-light metadata and center pixel: ${JSON.stringify(result)}`,
    );
  }
  const otherChannels = [0, 1, 2].filter((index) => index !== channelIndex);
  const minDominance = 16;
  if (
    center[channelIndex] < center[otherChannels[0]] + minDominance ||
    center[channelIndex] < center[otherChannels[1]] + minDominance
  ) {
    throw new Error(
      `${backend} ${workflow} did not tint PBR output through the ${channel} light lane: ${JSON.stringify(result)}`,
    );
  }
}

function assertNormalMapProof(backend, result) {
  const metadata = result.metadata || {};
  const normalMapPixels = metadata.normal_map_pixels || {};
  if (
    metadata.proof_class !== "browser-pbr-normal-map" ||
    normalMapPixels.flat_normal !== true ||
    normalMapPixels.inverted_normal !== true ||
    !result.pixels ||
    !Array.isArray(result.pixels.flat) ||
    !Array.isArray(result.pixels.inverted)
  ) {
    throw new Error(
      `${backend} pbr-normal-map proof did not record normal-map metadata and sample pixels: ${JSON.stringify(result)}`,
    );
  }
  const flat = result.pixels.flat;
  const inverted = result.pixels.inverted;
  if (
    flat[0] <= inverted[0] + 20 ||
    flat[1] <= inverted[1] + 20 ||
    flat[2] <= inverted[2] + 20
  ) {
    throw new Error(
      `${backend} pbr-normal-map did not prove tangent-space normal texture changes PBR lighting: ${JSON.stringify(result)}`,
    );
  }
}

function assertEnvironmentLightProof(backend, result) {
  const metadata = result.metadata || {};
  const center = result.pixels && result.pixels.center;
  if (
    metadata.proof_class !== "browser-pbr-environment-light" ||
    metadata.environment_kind !== "inline-radiance-hdr" ||
    metadata.material_kind !== "pbr-metallic-roughness" ||
    !Array.isArray(center)
  ) {
    throw new Error(
      `${backend} pbr-environment proof did not record environment-light metadata and center pixel: ${JSON.stringify(result)}`,
    );
  }
  if (center[2] <= center[0] + 20 || center[2] <= center[1] + 10) {
    throw new Error(
      `${backend} pbr-environment did not tint PBR output through the active HDR environment: ${JSON.stringify(result)}`,
    );
  }
}

function assertShadowVisibilityProof(backend, result) {
  const metadata = result.metadata || {};
  const lit = result.pixels && result.pixels.flat;
  const shadowed = result.pixels && result.pixels.center;
  if (
    metadata.proof_class !== "browser-pbr-directional-shadow-visibility" ||
    metadata.shadow_source !== "prepared-visibility" ||
    metadata.material_kind !== "pbr-metallic-roughness" ||
    !Array.isArray(lit) ||
    !Array.isArray(shadowed)
  ) {
    throw new Error(
      `${backend} pbr-shadow-visibility proof did not record shadow metadata and sample pixels: ${JSON.stringify(result)}`,
    );
  }
  if (
    shadowed[0] + 15 >= lit[0] ||
    shadowed[1] + 15 >= lit[1] ||
    shadowed[2] + 15 >= lit[2]
  ) {
    throw new Error(
      `${backend} pbr-shadow-visibility did not darken the prepared shadow receiver: ${JSON.stringify(result)}`,
    );
  }
}

function assertTexturedConnectorViewerProof(backend, result) {
  const metadata = result.metadata || {};
  if (
    metadata.decoded_base_color_texture !== true ||
    metadata.connected !== true ||
    metadata.framed !== true ||
    metadata.picked !== true ||
    metadata.selected !== true ||
    !metadata.connection_line
  ) {
    throw new Error(
      `${backend} textured-connector-viewer did not prove load/place/connect/frame/pick/render workflow: ${JSON.stringify(result)}`,
    );
  }
  if (!result.stats || result.stats.material_texture_bindings < 1) {
    throw new Error(
      `${backend} textured-connector-viewer did not report a material texture binding: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0) {
    throw new Error(
      `${backend} textured-connector-viewer did not render visible textured assembly pixels: ${JSON.stringify(result)}`,
    );
  }
}

function renderedOutputFingerprint(result) {
  const readback = result && result.renderer_readback;
  if (readback && typeof readback.rgba8_fnv1a64 === "string") {
    return `renderer:${readback.rgba8_fnv1a64}`;
  }
  if (result && typeof result.canvas_data_url === "string") {
    return `canvas:${result.canvas_data_url}`;
  }
  return null;
}

async function main() {
  const { chromium } = loadPlaywright();
  const browserRoot = __dirname;
  const pkgRoot = path.join(process.cwd(), "target", "m6-browser-pkg");
  const fixtureRoot = path.join(process.cwd(), "tests", "assets");
  const artifactDir = path.join(process.cwd(), "target", "gate-artifacts");
  fs.mkdirSync(artifactDir, { recursive: true });

  const { server, url } = await serve(browserRoot, pkgRoot, fixtureRoot);
  const selectedBackends = configuredBackends();
  const browser = await chromium.launch({
    headless: true,
    args: chromiumLaunchArgs(selectedBackends),
  });

  const workflows = [
    "model-viewer",
    "instancing",
    "picking-selection",
    "animation",
    "labels-helpers",
    "industrial-static-scene",
    "depth-overlap",
    "pbr-point-light",
    "pbr-spot-light",
    "pbr-normal-map",
    "pbr-environment",
    "pbr-shadow-visibility",
    "camera-framing",
    "anchor-alignment",
    "connector-before",
    "connector-after",
    "coordinate-units",
    "static-batching",
    "layers-helper-on-top",
    "beginner-diagnostics",
    "material-textures",
    "source-gltf-materials",
    "textured-connector-viewer",
    "asset-cache-reload",
  ];
  const results = [];
  try {
    for (const backend of selectedBackends) {
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
      try {
        await page.goto(url);
        let result;
        try {
          result = await page.evaluate(
            (name) => window.scenaM6RustWasmRendererProbe(name),
            backend,
          );
        } catch (error) {
          if (consoleMessages.length > 0) {
            error.message += `\nconsole:\n${consoleMessages.join("\n")}`;
          }
          throw error;
        }
        results.push(result);
        if (result.status !== "passed") {
          const consoleSuffix =
            consoleMessages.length > 0 ? `\nconsole:\n${consoleMessages.join("\n")}` : "";
          throw new Error(
            `${backend} Rust/WASM renderer probe failed: ${JSON.stringify(result)}${consoleSuffix}`,
          );
        }
        const workflowResults = new Map();
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
          attachFixtureHash(fixtureRoot, workflowResult);
          results.push(workflowResult);
          if (workflowResult.status !== "passed") {
            throw new Error(
              `${backend} ${workflow} Rust/WASM renderer probe failed: ${JSON.stringify(workflowResult)}`,
            );
          }
          workflowResults.set(workflow, workflowResult);
        }
        assertModelViewerProof(backend, workflowResults.get("model-viewer"));
        assertDepthOverlapProof(backend, workflowResults.get("depth-overlap"));
        assertPunctualLightProof(
          backend,
          workflowResults.get("pbr-point-light"),
          "green",
          "pbr-point-light",
        );
        assertPunctualLightProof(
          backend,
          workflowResults.get("pbr-spot-light"),
          "blue",
          "pbr-spot-light",
        );
        assertNormalMapProof(backend, workflowResults.get("pbr-normal-map"));
        assertEnvironmentLightProof(backend, workflowResults.get("pbr-environment"));
        assertShadowVisibilityProof(backend, workflowResults.get("pbr-shadow-visibility"));
        assertMaterialTextureProof(backend, workflowResults.get("material-textures"));
        assertSourceGltfMaterialProof(backend, workflowResults.get("source-gltf-materials"));
        assertTexturedConnectorViewerProof(
          backend,
          workflowResults.get("textured-connector-viewer"),
        );
        const connectorBefore = workflowResults.get("connector-before");
        const connectorAfter = workflowResults.get("connector-after");
        const connectorBeforeFingerprint = renderedOutputFingerprint(connectorBefore);
        const connectorAfterFingerprint = renderedOutputFingerprint(connectorAfter);
        if (
          !connectorBefore ||
          !connectorAfter ||
          !connectorBeforeFingerprint ||
          !connectorAfterFingerprint ||
          connectorBeforeFingerprint === connectorAfterFingerprint
        ) {
          throw new Error(
            `${backend} connector before/after workflow did not change rendered output`,
          );
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
        assertSurfaceLifecycleProbe(backend, lifecycleResult);
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
        assertNoScenaGpuValidationErrors(backend, consoleMessages);
      } catch (error) {
        if (!isAllowedUnavailable(backend, error)) {
          throw error;
        }
        results.push(unavailableResult(backend, error));
      } finally {
        await page.close();
      }
    }
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }

  const artifact = {
    gate: "m6-rust-wasm-renderer-probe",
    status: results.some((result) => result.status === "unavailable") ? "unavailable" : "passed",
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

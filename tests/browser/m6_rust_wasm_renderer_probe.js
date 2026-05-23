const fs = require("fs");
const crypto = require("crypto");
const http = require("http");
const path = require("path");
const zlib = require("zlib");

const MODEL_VIEWER_FIXTURE = "/fixtures/gltf/non_ndc_camera_scene.gltf";
const MODEL_VIEWER_BUNDLE = "model-viewer.min.js";
const OVERSIZED_TEXTURE_DIMENSION = 2049;

function loadPlaywright() {
  return require("playwright");
}

function contentType(file) {
  if (file.endsWith(".wasm")) return "application/wasm";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".json")) return "application/json; charset=utf-8";
  if (file.endsWith(".html")) return "text/html; charset=utf-8";
  if (file.endsWith(".gltf")) return "model/gltf+json";
  if (file.endsWith(".glb")) return "model/gltf-binary";
  if (file.endsWith(".bin")) return "application/octet-stream";
  if (file.endsWith(".png")) return "image/png";
  if (file.endsWith(".jpg") || file.endsWith(".jpeg")) return "image/jpeg";
  if (file.endsWith(".webp")) return "image/webp";
  return "application/octet-stream";
}

let generatedOversizedTexturePng = null;
let generatedOversizedTextureScene = null;

function crc32(bytes) {
  if (!crc32.table) {
    crc32.table = Array.from({ length: 256 }, (_, index) => {
      let value = index;
      for (let bit = 0; bit < 8; bit += 1) {
        value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
      }
      return value >>> 0;
    });
  }
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc = crc32.table[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const typeBytes = Buffer.from(type, "ascii");
  const body = Buffer.concat([typeBytes, data]);
  const chunk = Buffer.alloc(12 + data.length);
  chunk.writeUInt32BE(data.length, 0);
  typeBytes.copy(chunk, 4);
  data.copy(chunk, 8);
  chunk.writeUInt32BE(crc32(body), 8 + data.length);
  return chunk;
}

function solidPng(width, height, rgba) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const row = Buffer.allocUnsafe(width * 4 + 1);
  row[0] = 0;
  for (let offset = 1; offset < row.length; offset += 4) {
    row[offset] = rgba[0];
    row[offset + 1] = rgba[1];
    row[offset + 2] = rgba[2];
    row[offset + 3] = rgba[3];
  }
  const raw = Buffer.allocUnsafe(row.length * height);
  for (let y = 0; y < height; y += 1) {
    row.copy(raw, y * row.length);
  }

  return Buffer.concat([
    signature,
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

function oversizedTexturePng() {
  if (!generatedOversizedTexturePng) {
    generatedOversizedTexturePng = solidPng(
      OVERSIZED_TEXTURE_DIMENSION,
      OVERSIZED_TEXTURE_DIMENSION,
      [220, 90, 40, 255],
    );
  }
  return generatedOversizedTexturePng;
}

function oversizedTextureSceneGltf() {
  if (generatedOversizedTextureScene) {
    return generatedOversizedTextureScene;
  }
  const buffer = Buffer.alloc(140);
  const positions = [
    -0.7, -0.7, 0.0,
     0.7, -0.7, 0.0,
     0.7,  0.7, 0.0,
    -0.7,  0.7, 0.0,
  ];
  const normals = [
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
  ];
  const texcoords = [
    0.0, 1.0,
    1.0, 1.0,
    1.0, 0.0,
    0.0, 0.0,
  ];
  const indices = [0, 1, 2, 0, 2, 3];
  positions.forEach((value, index) => buffer.writeFloatLE(value, index * 4));
  normals.forEach((value, index) => buffer.writeFloatLE(value, 48 + index * 4));
  texcoords.forEach((value, index) => buffer.writeFloatLE(value, 96 + index * 4));
  indices.forEach((value, index) => buffer.writeUInt16LE(value, 128 + index * 2));

  generatedOversizedTextureScene = `${JSON.stringify({
    asset: {
      version: "2.0",
      generator: "scena-m6-oversized-browser-texture-proof",
    },
    buffers: [
      {
        uri: `data:application/octet-stream;base64,${buffer.toString("base64")}`,
        byteLength: buffer.length,
      },
    ],
    bufferViews: [
      { buffer: 0, byteOffset: 0, byteLength: 48, target: 34962 },
      { buffer: 0, byteOffset: 48, byteLength: 48, target: 34962 },
      { buffer: 0, byteOffset: 96, byteLength: 32, target: 34962 },
      { buffer: 0, byteOffset: 128, byteLength: 12, target: 34963 },
    ],
    accessors: [
      {
        bufferView: 0,
        componentType: 5126,
        count: 4,
        type: "VEC3",
        min: [-0.7, -0.7, 0.0],
        max: [0.7, 0.7, 0.0],
      },
      {
        bufferView: 1,
        componentType: 5126,
        count: 4,
        type: "VEC3",
        min: [0.0, 0.0, 1.0],
        max: [0.0, 0.0, 1.0],
      },
      {
        bufferView: 2,
        componentType: 5126,
        count: 4,
        type: "VEC2",
        min: [0.0, 0.0],
        max: [1.0, 1.0],
      },
      {
        bufferView: 3,
        componentType: 5123,
        count: 6,
        type: "SCALAR",
        min: [0],
        max: [3],
      },
    ],
    images: [{ uri: "oversized_texture.png", mimeType: "image/png" }],
    samplers: [{ minFilter: 9729, magFilter: 9729, wrapS: 10497, wrapT: 10497 }],
    textures: [{ source: 0, sampler: 0 }],
    materials: [
      {
        name: "OversizedTextureMaterial",
        doubleSided: true,
        pbrMetallicRoughness: {
          baseColorTexture: { index: 0 },
          metallicFactor: 0.0,
          roughnessFactor: 0.65,
        },
      },
    ],
    meshes: [
      {
        primitives: [
          {
            attributes: { POSITION: 0, NORMAL: 1, TEXCOORD_0: 2 },
            indices: 3,
            material: 0,
            mode: 4,
          },
        ],
      },
    ],
    nodes: [{ mesh: 0, name: "OversizedTextureQuad" }],
    scenes: [{ nodes: [0] }],
    scene: 0,
  }, null, 2)}\n`;
  return generatedOversizedTextureScene;
}

function serve(browserRoot, pkgRoot, fixtureRoot, modelViewerRoot, demoRoot) {
  const server = http.createServer((request, response) => {
    const url = request.url === "/" ? "/m6_rust_wasm_renderer_probe.html" : request.url;
    if (url === "/fixtures/generated/oversized_texture_scene.gltf") {
      response.writeHead(200, { "Content-Type": "model/gltf+json" });
      response.end(oversizedTextureSceneGltf());
      return;
    }
    if (url === "/fixtures/generated/oversized_texture.png") {
      response.writeHead(200, { "Content-Type": "image/png" });
      response.end(oversizedTexturePng());
      return;
    }
    let base = browserRoot;
    let relative = url.slice(1);
    if (url.startsWith("/pkg/")) {
      base = pkgRoot;
      relative = url.slice("/pkg/".length);
    } else if (url.startsWith("/fixtures/")) {
      base = fixtureRoot;
      relative = url.slice("/fixtures/".length);
    } else if (url.startsWith("/demo/")) {
      base = demoRoot;
      relative = url.slice("/demo/".length);
    } else if (url.startsWith("/samples/")) {
      base = path.join(demoRoot, "samples");
      relative = url.slice("/samples/".length);
    } else if (url.startsWith("/model-viewer/")) {
      base = modelViewerRoot;
      relative = url.slice("/model-viewer/".length);
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

function compressedAssetProofEnabled() {
  return process.env.SCENA_BROWSER_COMPRESSED_ASSETS === "1";
}

function oversizedTextureProofEnabled() {
  return process.env.SCENA_BROWSER_OVERSIZED_TEXTURE === "1";
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

function assertOversizedBrowserTextureProof(backend, result) {
  const metadata = result.metadata || {};
  const size = metadata.browser_texture_size || [];
  if (
    metadata.proof_class !== "browser-oversized-source-texture-clamp" ||
    metadata.fixture !== "/fixtures/generated/oversized_texture_scene.gltf" ||
    !Array.isArray(metadata.source_texture_size) ||
    metadata.source_texture_size[0] !== OVERSIZED_TEXTURE_DIMENSION ||
    metadata.source_texture_size[1] !== OVERSIZED_TEXTURE_DIMENSION ||
    metadata.max_browser_texture_dimension !== 2048 ||
    !Array.isArray(size) ||
    size[0] !== 2048 ||
    size[1] !== 2048 ||
    metadata.load_warnings !== 0
  ) {
    throw new Error(
      `${backend} oversized-browser-texture proof did not record the expected over-limit browser texture clamp: ${JSON.stringify(result)}`,
    );
  }
  if (!result.stats || result.stats.material_texture_bindings < 1) {
    throw new Error(
      `${backend} oversized-browser-texture proof did not bind the clamped source texture: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0) {
    throw new Error(
      `${backend} oversized-browser-texture proof did not render visible pixels: ${JSON.stringify(result)}`,
    );
  }
}

function writeOversizedBrowserTextureArtifact(artifactDir, backend, result) {
  const artifact = {
    gate: "m6-oversized-browser-texture-probe",
    status: "passed",
    backend,
    result,
  };
  fs.writeFileSync(
    path.join(artifactDir, "m6-oversized-browser-texture-probe.json"),
    `${JSON.stringify(artifact, null, 2)}\n`,
  );
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
  const missingDecodedPixels = result.stats && result.stats.material_textures_missing_decoded_pixels;
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
  if (missingDecodedPixels !== 0) {
    throw new Error(
      `${backend} pbr-normal-map direct load_texture normal maps must decode before GPU upload; missing decoded pixels=${missingDecodedPixels}: ${JSON.stringify(result)}`,
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

function assertMaterialExtensionProof(backend, result) {
  const metadata = result.metadata || {};
  const extensions = new Set(metadata.extensions || []);
  for (const extension of [
    "KHR_materials_clearcoat",
    "KHR_materials_sheen",
    "KHR_materials_anisotropy",
    "KHR_materials_iridescence",
    "KHR_materials_dispersion",
  ]) {
    if (!extensions.has(extension)) {
      throw new Error(
        `${backend} pbr-material-extensions proof missed ${extension}: ${JSON.stringify(result)}`,
      );
    }
  }
  if (
    metadata.proof_class !== "browser-pbr-material-extension-composite" ||
    metadata.readback !== "browser-screenshot-or-renderer-owned-copy"
  ) {
    throw new Error(
      `${backend} pbr-material-extensions proof did not record release readback metadata: ${JSON.stringify(result)}`,
    );
  }
  if (!result.stats || result.stats.material_texture_bindings < 5) {
    throw new Error(
      `${backend} pbr-material-extensions proof did not bind extension textures: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0) {
    throw new Error(
      `${backend} pbr-material-extensions proof did not render visible extension materials: ${JSON.stringify(result)}`,
    );
  }
  if (
    typeof result.canvas_data_url !== "string" ||
    !result.canvas_data_url.startsWith("data:image/png;base64,") ||
    !result.screenshot_metadata ||
    !result.screenshot_metadata.pixel_statistics
  ) {
    throw new Error(
      `${backend} pbr-material-extensions proof did not preserve screenshot/readback evidence: ${JSON.stringify(result)}`,
    );
  }
}

function assertMaterialPresetProof(backend, result) {
  const metadata = result.metadata || {};
  const presets = new Set(metadata.preset_names || []);
  for (const preset of [
    "matte",
    "plastic",
    "metal",
    "rough_metal",
    "chrome",
    "brushed_steel",
    "clearcoat_plastic",
    "satin",
    "leather",
    "clear_glass",
    "frosted_glass",
    "rubber",
  ]) {
    if (!presets.has(preset)) {
      throw new Error(
        `${backend} pbr-material-presets proof missed ${preset}: ${JSON.stringify(result)}`,
      );
    }
  }
  if (
    metadata.proof_class !== "browser-pbr-material-preset-expanded-set" ||
    metadata.glass_contract !== "scene-color-ior-thickness-rough-blur-sorted-transparency" ||
    metadata.environment_path !== "/demo/samples/environment/white_studio_03_1k.hdr"
  ) {
    throw new Error(
      `${backend} pbr-material-presets proof did not record preset contract metadata: ${JSON.stringify(result)}`,
    );
  }
  const geometry = new Set(metadata.showcase_geometry || []);
  for (const requiredGeometry of [
    "curved-panel",
    "curved-part",
    "brushed-plate",
    "folded-sheet",
    "strap-panel",
    "glass-block-grid",
    "glass-screen-grid",
    "gasket-foot",
  ]) {
    if (!geometry.has(requiredGeometry)) {
      throw new Error(
        `${backend} pbr-material-presets proof still allows a single-shape grid: ${JSON.stringify(result)}`,
      );
    }
  }
  const sourceSurfaces = new Map((metadata.source_surfaces || []).map((entry) => [entry.id, entry.surface]));
  for (const sourceBacked of ["satin", "leather", "rubber"]) {
    if (sourceSurfaces.get(sourceBacked) !== "Assets::material_presets()") {
      throw new Error(
        `${backend} pbr-material-presets proof conflated ${sourceBacked} with MaterialDesc shortcuts: ${JSON.stringify(result)}`,
      );
    }
  }
  if (backend === "webgl2" && metadata.webgl2_smooth_metal_sample_floor < 96) {
    throw new Error(
      `${backend} pbr-material-presets proof did not pin the raised smooth-metal sample floor: ${JSON.stringify(result)}`,
    );
  }
  if (!result.stats || result.stats.materials < 12 || result.primitives < 12) {
    throw new Error(
      `${backend} pbr-material-presets proof did not prepare the expanded preset set: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0) {
    throw new Error(
      `${backend} pbr-material-presets proof did not render visible preset output: ${JSON.stringify(result)}`,
    );
  }
  if (
    typeof result.canvas_data_url !== "string" ||
    !result.canvas_data_url.startsWith("data:image/png;base64,") ||
    !result.screenshot_metadata ||
    !result.screenshot_metadata.pixel_statistics
  ) {
    throw new Error(
      `${backend} pbr-material-presets proof did not preserve screenshot/readback evidence: ${JSON.stringify(result)}`,
    );
  }
}

function assertCompressedAssetProof(backend, result) {
  const metadata = result.metadata || {};
  const ktx2 = metadata.ktx2_probe || {};
  if (
    metadata.proof_class !== "browser-compressed-asset-runtime" ||
    metadata.meshopt_required_extension !== true ||
    metadata.meshopt_decoder !== "EXT_meshopt_compression bufferView expansion"
  ) {
    throw new Error(
      `${backend} compressed-assets proof did not record meshopt compressed metadata: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0 || result.primitives < 1) {
    throw new Error(
      `${backend} compressed-assets proof did not render visible meshopt browser output: ${JSON.stringify(result)}`,
    );
  }
  if (ktx2.status !== "fail-closed" || !String(ktx2.error || "").includes("KTX2/Basis")) {
    throw new Error(
      `${backend} compressed-assets proof did not record the current browser KTX2 fail-closed result: ${JSON.stringify(result)}`,
    );
  }
}

function writeCompressedAssetBrowserLaneArtifact(artifactDir, backend, result) {
  const lane = backend === "webgpu" ? "browser-webgpu" : "browser-webgl2";
  const root = path.join(artifactDir, "m8-compressed-assets");
  fs.mkdirSync(root, { recursive: true });
  const metadata = result.metadata || {};
  const ktx2 = metadata.ktx2_probe || {};
  const artifact = {
    schema: "scena.compressed_asset_backend_lane.v1",
    lane,
    status: "partial-meshopt-passed-ktx2-fail-closed",
    commit_sha: process.env.GITHUB_SHA || "local-checkout",
    release_evidence: false,
    reason:
      "browser production-assets runtime rendered EXT_meshopt_compression, but KTX2/Basis remains fail-closed on the sync wasm texture path",
    browser_runtime_evidence: true,
    workflow: result.workflow,
    backend: result.backend,
    meshopt: {
      status: "passed",
      proof_class: metadata.proof_class,
      pixels: result.pixels,
      stats: result.stats,
      screenshot_metadata: result.screenshot_metadata,
    },
    ktx2: {
      status: ktx2.status,
      error: ktx2.error || null,
      release_evidence: false,
    },
  };
  fs.writeFileSync(
    path.join(root, `${lane}-compressed-lane.json`),
    `${JSON.stringify(artifact, null, 2)}\n`,
  );
}

function writeCompressedAssetBrowserLaneUnavailable(artifactDir, backend, error) {
  const lane = backend === "webgpu" ? "browser-webgpu" : "browser-webgl2";
  const root = path.join(artifactDir, "m8-compressed-assets");
  fs.mkdirSync(root, { recursive: true });
  fs.writeFileSync(
    path.join(root, `${lane}-compressed-lane.json`),
    `${JSON.stringify(
      {
        schema: "scena.compressed_asset_backend_lane.v1",
        lane,
        status: "unavailable",
        commit_sha: process.env.GITHUB_SHA || "local-checkout",
        release_evidence: false,
        reason: String(error && error.message ? error.message : error),
        browser_runtime_evidence: true,
      },
      null,
      2,
    )}\n`,
  );
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

function assertConnectorMagnetPreviewProof(backend, result) {
  const metadata = result && result.metadata ? result.metadata : {};
  const sequence = metadata.magnet_sequence;
  if (
    metadata.proof_class !== "connector-magnet-preview" ||
    !Array.isArray(sequence) ||
    sequence.length !== 2
  ) {
    throw new Error(
      `${backend} connector-magnet-preview did not record the required magnet sequence: ${JSON.stringify(result)}`,
    );
  }
  const [outOfRange, ready] = sequence;
  if (
    outOfRange.visual_cue !== "scena-magnet-out-of-range" ||
    outOfRange.snap_ready !== false ||
    !(outOfRange.distance > outOfRange.tolerance)
  ) {
    throw new Error(
      `${backend} connector-magnet-preview did not prove the out-of-range cue: ${JSON.stringify(result)}`,
    );
  }
  if (
    ready.visual_cue !== "scena-magnet-ready" ||
    ready.snap_ready !== true ||
    !(ready.distance <= ready.tolerance)
  ) {
    throw new Error(
      `${backend} connector-magnet-preview did not prove the snap-ready cue: ${JSON.stringify(result)}`,
    );
  }
  if (!result.pixels || result.pixels.nonblack <= 0) {
    throw new Error(
      `${backend} connector-magnet-preview did not render visible browser pixels: ${JSON.stringify(result)}`,
    );
  }
}

function assertScenaViewerElementProof(result) {
  if (
    !result ||
    result.schema !== "scena.scena_viewer_element_browser_proof.v1" ||
    result.status !== "passed" ||
    !result.screenshot_metadata ||
    !/^[0-9a-f]{64}$/.test(result.screenshot_metadata.sha256 || "")
  ) {
    throw new Error(`<scena-viewer> browser proof did not pass: ${JSON.stringify(result)}`);
  }
  const checks = result.checks || {};
  for (const [key, value] of [
    ["host_role", "img"],
    ["host_label", "3D model viewer"],
    ["host_tabindex", "0"],
    ["host_roledescription", "interactive 3D model"],
    ["canvas_label", "scena 3D viewer canvas"],
    ["canvas_touch_action", "none"],
    ["progress_phase", "fetching"],
    ["progress_value_now", "42"],
    ["variant_change", "noon"],
    ["annotation_count", 1],
    ["annotation_visible", 1],
    ["annotation_update_visible", 1],
    ["inspector_overlay", "Diagnostics"],
    ["inspector_warnings", 1],
    ["inspector_fixture_schema", "scena.scena_viewer_inspector_snapshot.v1"],
    ["inspector_fixture_source", "scena-viewer-inspector-fixture"],
    ["keyboard_action", "orbit-left"],
  ]) {
    if (checks[key] !== value) {
      throw new Error(`<scena-viewer> proof expected ${key}=${value}: ${JSON.stringify(result)}`);
    }
  }
  if (
    !Array.isArray(checks.progress_sequence) ||
    checks.progress_sequence.length !== 2 ||
    checks.progress_sequence[0].phase !== "loading" ||
    checks.progress_sequence[0].valueNow !== null ||
    checks.progress_sequence[1].phase !== "fetching" ||
    checks.progress_sequence[1].valueNow !== "42" ||
    checks.progress_sequence[1].barTransform !== "scaleX(0.42)"
  ) {
    throw new Error(`<scena-viewer> proof did not exercise progress phase sequencing: ${JSON.stringify(result)}`);
  }
  if (
    checks.variant_render_status !== "passed" ||
    checks.variant_render_workflow !== "scena-viewer-material-variant-render" ||
    checks.variant_render_selected !== "noon" ||
    checks.variant_render_active !== "noon" ||
    checks.variant_render_green_dominant !== true ||
    !(checks.variant_render_pixels_nonblack > 0)
  ) {
    throw new Error(`<scena-viewer> proof did not render the selected material variant: ${JSON.stringify(result)}`);
  }
  if (
    !Array.isArray(checks.annotation_tracking_sequence) ||
    checks.annotation_tracking_sequence.length !== 2 ||
    checks.annotation_tracking_sequence[0] === checks.annotation_tracking_sequence[1]
  ) {
    throw new Error(`<scena-viewer> proof did not exercise annotation tracking updates: ${JSON.stringify(result)}`);
  }
  if (
    !Array.isArray(checks.drop_accepted_names) ||
    !checks.drop_accepted_names.includes("accepted-machine.glb") ||
    !Array.isArray(checks.drop_rejected_names) ||
    !checks.drop_rejected_names.includes("notes.txt") ||
    checks.drop_render_status !== "passed" ||
    checks.drop_render_workflow !== "scena-viewer-drop-render" ||
    checks.drop_render_file_name !== "accepted-machine.glb" ||
    !(checks.drop_render_roots > 0) ||
    !(checks.drop_render_pixels_nonblack > 0) ||
    checks.drop_render_auto_frame_status !== "passed" ||
    checks.drop_render_auto_frame_proof_class !== "viewer-level-auto-framing" ||
    checks.drop_render_auto_frame_inside_viewport !== true ||
    checks.drop_render_auto_frame_centered !== true ||
    !(checks.drop_render_auto_frame_fill_fraction > 0.2 && checks.drop_render_auto_frame_fill_fraction <= 0.75)
  ) {
    throw new Error(`<scena-viewer> proof did not exercise drag/drop render-after-drop with auto-framing: ${JSON.stringify(result)}`);
  }
}

function assertScenaViewerParityProof(result) {
  if (
    !result ||
    result.schema !== "scena.scena_viewer_model_viewer_parity_proof.v1" ||
    result.status !== "passed" ||
    result.proof_class !== "three_asset_side_by_side" ||
    result.visual_proof !== "side-by-side-screenshot" ||
    !result.screenshot_metadata ||
    !/^[0-9a-f]{64}$/.test(result.screenshot_metadata.sha256 || "") ||
    typeof result.model_viewer_package !== "string" ||
    !result.model_viewer_package.startsWith("@google/model-viewer@")
  ) {
    throw new Error(`<scena-viewer> parity proof did not pass: ${JSON.stringify(result)}`);
  }
  const expectedSources = new Set([
    "/fixtures/gltf/non_ndc_camera_scene.gltf",
    "/fixtures/gltf/khronos/MorphCube/AnimatedMorphCube.gltf",
    "/fixtures/gltf/khronos/WaterBottle/WaterBottle.gltf",
  ]);
  if (!Array.isArray(result.assets) || result.assets.length !== expectedSources.size) {
    throw new Error(`<scena-viewer> parity proof did not cover three assets: ${JSON.stringify(result)}`);
  }
  for (const asset of result.assets) {
    if (!expectedSources.delete(asset.source)) {
      throw new Error(`<scena-viewer> parity proof used unexpected or duplicate asset: ${JSON.stringify(asset)}`);
    }
    if (
      asset.side_by_side !== true ||
      asset.model_viewer_tag !== "MODEL-VIEWER" ||
      asset.scena_viewer_tag !== "SCENA-VIEWER" ||
      asset.model_viewer_loaded !== true ||
      asset.model_viewer_canvas_ready !== true ||
      asset.scena_render_status !== "passed" ||
      asset.scena_backend !== "webgl2" ||
      !(asset.scena_pixels_nonblack > 0) ||
      !(asset.model_viewer_width > 0) ||
      !(asset.model_viewer_height > 0)
    ) {
      throw new Error(`<scena-viewer> parity asset did not prove side-by-side rendering: ${JSON.stringify(asset)}`);
    }
  }
  if (expectedSources.size !== 0) {
    throw new Error(`<scena-viewer> parity proof missed assets: ${Array.from(expectedSources).join(", ")}`);
  }
}

function assertCameraControlKitProof(result) {
  if (
    !result ||
    result.schema !== "scena.m6.camera_control_kit_browser_proof.v1" ||
    result.status !== "passed" ||
    !result.screenshot_metadata ||
    !/^[0-9a-f]{64}$/.test(result.screenshot_metadata.sha256 || "")
  ) {
    throw new Error(`camera control kit browser proof did not pass: ${JSON.stringify(result)}`);
  }
  const actions = new Set(result.orbit && result.orbit.actions);
  for (const action of ["BeginOrbit", "Orbit", "Zoom", "End"]) {
    if (!actions.has(action)) {
      throw new Error(`camera control kit proof did not include orbit action ${action}: ${JSON.stringify(result)}`);
    }
  }
  if (result.orbit.distance_after_zoom >= result.orbit.initial_distance) {
    throw new Error(`camera control kit proof did not zoom closer: ${JSON.stringify(result)}`);
  }
  if (!result.follow || result.follow.camera_translation[1] <= result.follow.target_translation[1]) {
    throw new Error(`camera control kit proof did not place follow camera above target: ${JSON.stringify(result)}`);
  }
  if (!result.fly || result.fly.camera_translation[0] <= 0 || result.fly.camera_translation[2] >= 0) {
    throw new Error(`camera control kit proof did not move fly camera in local axes: ${JSON.stringify(result)}`);
  }
}

function assertScenaViewerMobileA11yProof(result) {
  if (!result || result.schema !== "scena.scena_viewer_mobile_a11y_browser_proof.v1") {
    throw new Error(`<scena-viewer> mobile proof returned unexpected schema: ${JSON.stringify(result)}`);
  }
  if (result.status !== "passed") {
    throw new Error(`<scena-viewer> mobile proof did not pass: ${JSON.stringify(result)}`);
  }
  const checks = result.checks || {};
  const expected = [
    ["viewer_overflows_x", false],
    ["host_role", "img"],
    ["host_tabindex", "0"],
    ["canvas_touch_action", "none"],
    ["pinch_action", "pinch-zoom"],
    ["pinch_pointers", 2],
    ["pinch_delta_positive", true],
    ["orbit_action", "orbit"],
    ["orbit_pointer_type", "touch"],
    ["orbit_delta_x", 26],
    ["orbit_delta_y", 14],
    ["wheel_action", "wheel-zoom"],
    ["wheel_delta_y", -120],
    ["keyboard_action", "reset-view"],
  ];
  for (const [key, value] of expected) {
    if (checks[key] !== value) {
      throw new Error(`<scena-viewer> mobile proof expected ${key}=${value}: ${JSON.stringify(result)}`);
    }
  }
  if (checks.viewport_width > 390 || checks.viewer_width > checks.viewport_width) {
    throw new Error(`<scena-viewer> mobile proof overflowed its viewport: ${JSON.stringify(result)}`);
  }
}

async function runCameraControlKitProof(page, artifactDir) {
  const result = await page.evaluate(() => window.scenaCameraControlKitProbe());
  const screenshotPath = path.join(artifactDir, "camera-control-kit-browser-proof.png");
  await page
    .locator("section[data-proof=\"camera-control-kit\"]")
    .screenshot({ path: screenshotPath });
  const screenshot = fs.readFileSync(screenshotPath);
  result.screenshot_metadata = {
    path: path.relative(process.cwd(), screenshotPath),
    mime: "image/png",
    sha256: crypto.createHash("sha256").update(screenshot).digest("hex"),
    bytes: screenshot.length,
  };
  assertCameraControlKitProof(result);
  return result;
}

async function runScenaViewerMobileA11yProof(page, artifactDir) {
  const result = await page.evaluate(() => window.scenaViewerMobileA11yProbe());
  const screenshotPath = path.join(artifactDir, "scena-viewer-mobile-a11y-browser-proof.png");
  await page
    .locator(result.screenshot_selector || "scena-viewer[data-proof=\"mobile-a11y\"]")
    .screenshot({ path: screenshotPath });
  const screenshot = fs.readFileSync(screenshotPath);
  result.screenshot_metadata = {
    path: path.relative(process.cwd(), screenshotPath),
    mime: "image/png",
    sha256: crypto.createHash("sha256").update(screenshot).digest("hex"),
    bytes: screenshot.length,
  };
  assertScenaViewerMobileA11yProof(result);
  return result;
}

async function runScenaViewerElementProof(page, artifactDir) {
  const result = await page.evaluate(() => window.scenaViewerElementProbe());
  const screenshotPath = path.join(artifactDir, "scena-viewer-element-browser-proof.png");
  await page
    .locator(result.screenshot_selector || "scena-viewer[data-proof=\"custom-element\"]")
    .screenshot({ path: screenshotPath });
  const screenshot = fs.readFileSync(screenshotPath);
  result.screenshot_metadata = {
    path: path.relative(process.cwd(), screenshotPath),
    mime: "image/png",
    sha256: crypto.createHash("sha256").update(screenshot).digest("hex"),
    bytes: screenshot.length,
  };
  assertScenaViewerElementProof(result);
  return result;
}

function modelViewerPackageVersion() {
  const modelViewerPackage = JSON.parse(
    fs.readFileSync(
      path.join(process.cwd(), "node_modules", "@google", "model-viewer", "package.json"),
      "utf8",
    ),
  );
  return `@google/model-viewer@${modelViewerPackage.version}`;
}

async function runScenaViewerParityProof(page, artifactDir) {
  const result = await page.evaluate(() => window.scenaViewerModelViewerParityProbe("webgl2"));
  result.model_viewer_package = modelViewerPackageVersion();
  const screenshotPath = path.join(
    artifactDir,
    "scena-viewer-model-viewer-parity-browser-proof.png",
  );
  await page
    .locator(result.screenshot_selector || "section[data-proof=\"scena-viewer-model-viewer-parity\"]")
    .screenshot({ path: screenshotPath });
  const screenshot = fs.readFileSync(screenshotPath);
  result.screenshot_metadata = {
    path: path.relative(process.cwd(), screenshotPath),
    mime: "image/png",
    sha256: crypto.createHash("sha256").update(screenshot).digest("hex"),
    bytes: screenshot.length,
  };
  assertScenaViewerParityProof(result);
  return result;
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

function assertDisplayP3OutputProof(backend, result) {
  const output = result.canvas_output_color_space || {};
  if (result.status !== "passed") {
    throw new Error(`${backend} Display P3 proof did not render: ${JSON.stringify(result)}`);
  }
  if (result.requested_output_color_space !== "DisplayP3") {
    throw new Error(`${backend} Display P3 proof did not use the RendererOptions output-color-space path: ${JSON.stringify(result)}`);
  }
  if (result.capabilities.wide_gamut_output !== "Supported") {
    throw new Error(`${backend} Display P3 proof did not report supported wide gamut: ${JSON.stringify(result)}`);
  }
  if (result.capabilities.output_stage !== "PbrNeutralDisplayP3") {
    throw new Error(`${backend} Display P3 proof did not switch output stage: ${JSON.stringify(result)}`);
  }
  if (result.capabilities.color_target_format !== "Rgba8UnormSrgb+DisplayP3Canvas") {
    throw new Error(`${backend} Display P3 proof did not record the canvas color target: ${JSON.stringify(result)}`);
  }
  if (
    output.requested !== "display-p3" ||
    output.configured !== true ||
    output.effective !== "display-p3" ||
    output.display_p3 !== true
  ) {
    throw new Error(`${backend} Display P3 canvas output was not configured end to end: ${JSON.stringify(result)}`);
  }
}

async function main() {
  const { chromium } = loadPlaywright();
  const browserRoot = __dirname;
  const pkgRoot = path.join(process.cwd(), "target", "m6-browser-pkg");
  const fixtureRoot = path.join(process.cwd(), "tests", "assets");
  const demoRoot = path.join(process.cwd(), "demo");
  const modelViewerRoot = path.join(
    process.cwd(),
    "node_modules",
    "@google",
    "model-viewer",
    "dist",
  );
  const artifactDir = path.join(process.cwd(), "target", "gate-artifacts");
  fs.mkdirSync(artifactDir, { recursive: true });

  const { server, url } = await serve(browserRoot, pkgRoot, fixtureRoot, modelViewerRoot, demoRoot);
  const selectedBackends = configuredBackends();
  const viewerElementOnly = process.env.SCENA_BROWSER_VIEWER_ELEMENT_ONLY === "1";
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
    "pbr-material-extensions",
    "pbr-material-presets",
    "camera-framing",
    "anchor-alignment",
    "connector-before",
    "connector-after",
    "connector-magnet-preview",
    "coordinate-units",
    "static-batching",
    "layers-helper-on-top",
    "beginner-diagnostics",
    "material-textures",
    "source-gltf-materials",
    "textured-connector-viewer",
    "asset-cache-reload",
  ];
  if (oversizedTextureProofEnabled()) {
    workflows.push("oversized-browser-texture");
  }
  if (compressedAssetProofEnabled()) {
    workflows.push("compressed-assets");
  }
  const results = [];
  try {
    const viewerElementPage = await browser.newPage({ viewport: { width: 480, height: 320 } });
    try {
      await viewerElementPage.goto(url);
      results.push(await runScenaViewerElementProof(viewerElementPage, artifactDir));
      results.push(await runCameraControlKitProof(viewerElementPage, artifactDir));
    } finally {
      await viewerElementPage.close();
    }
    const viewerParityPage = await browser.newPage({ viewport: { width: 960, height: 760 } });
    try {
      await viewerParityPage.goto(url);
      results.push(await runScenaViewerParityProof(viewerParityPage, artifactDir));
    } finally {
      await viewerParityPage.close();
    }
    const mobileA11yPage = await browser.newPage({ viewport: { width: 390, height: 640 }, isMobile: true, hasTouch: true });
    try {
      await mobileA11yPage.goto(url);
      results.push(await runScenaViewerMobileA11yProof(mobileA11yPage, artifactDir));
    } finally {
      await mobileA11yPage.close();
    }
    for (const backend of viewerElementOnly ? [] : selectedBackends) {
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
        const displayP3Result = await page.evaluate(
          (name) => window.scenaM6DisplayP3OutputProbe(name),
          backend,
        );
        results.push(displayP3Result);
        assertDisplayP3OutputProof(backend, displayP3Result);
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
        assertMaterialExtensionProof(backend, workflowResults.get("pbr-material-extensions"));
        assertMaterialPresetProof(backend, workflowResults.get("pbr-material-presets"));
        assertMaterialTextureProof(backend, workflowResults.get("material-textures"));
        assertSourceGltfMaterialProof(backend, workflowResults.get("source-gltf-materials"));
        if (oversizedTextureProofEnabled()) {
          const oversizedTexture = workflowResults.get("oversized-browser-texture");
          assertOversizedBrowserTextureProof(backend, oversizedTexture);
          writeOversizedBrowserTextureArtifact(artifactDir, backend, oversizedTexture);
        }
        if (compressedAssetProofEnabled()) {
          const compressedAssets = workflowResults.get("compressed-assets");
          assertCompressedAssetProof(backend, compressedAssets);
          writeCompressedAssetBrowserLaneArtifact(artifactDir, backend, compressedAssets);
        }
        assertTexturedConnectorViewerProof(
          backend,
          workflowResults.get("textured-connector-viewer"),
        );
        assertConnectorMagnetPreviewProof(
          backend,
          workflowResults.get("connector-magnet-preview"),
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
        if (compressedAssetProofEnabled()) {
          writeCompressedAssetBrowserLaneUnavailable(artifactDir, backend, error);
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

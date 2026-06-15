const childProcess = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const http = require("http");
const os = require("os");
const path = require("path");
const zlib = require("zlib");

const SCHEMA = "scena.scene_host_browser_proof.v1";
const BACKEND = "webgl2";
const VIEWPORT = { width: 640, height: 480, devicePixelRatio: 1.5 };
const ASSET_URL = "/assets/gltf/mesh_material_vertex_color_scene.gltf";
const PHASE4_ASSET_URL = "/assets/gltf/material_variants_scene.gltf";
const PHASE5_ANIMATED_ASSET_URL = "/assets/gltf/animated_triangle_scene.glb";
const ARTIFACT_DIR = path.join(
  process.cwd(),
  "target",
  "gate-artifacts",
  "scene-host-browser-proof",
);
const SCREENSHOT_PATH = path.join(ARTIFACT_DIR, "scene-host-browser-proof.png");
const ARTIFACT_PATH = path.join(ARTIFACT_DIR, "scene-host-browser-proof.json");
const PKG_DIR = path.join(process.cwd(), "target", "scene-host-browser-pkg");
const REQUIRED_BINDINGS = [
  ["static", "newWebgl2"],
  ["prototype", "addEmpty"],
  ["prototype", "removeNode"],
  ["prototype", "instantiateUrlUnder"],
  ["prototype", "instantiateUrlInstanced"],
  ["prototype", "instantiateUrlInstancedUnder"],
  ["prototype", "instantiateUrlUnderWithReportJson"],
  ["prototype", "setTransforms"],
  ["prototype", "setTransformsTyped"],
  ["prototype", "setTransformEased"],
  ["prototype", "setTransformsEased"],
  ["prototype", "setTransformsEasedTyped"],
  ["prototype", "setVisible"],
  ["prototype", "showOnly"],
  ["prototype", "isolate"],
  ["prototype", "ghost"],
  ["prototype", "fitSelection"],
  ["prototype", "setNodeTint"],
  ["prototype", "setNodeTintEased"],
  ["prototype", "clearNodeTintEased"],
  ["prototype", "animationInventoryJson"],
  ["prototype", "playAnimation"],
  ["prototype", "pauseAnimation"],
  ["prototype", "stopAnimation"],
  ["prototype", "seekAnimation"],
  ["prototype", "setAnimationSpeed"],
  ["prototype", "advance"],
  ["prototype", "setAntiAliasing"],
  ["prototype", "setBloom"],
  ["prototype", "setAmbientOcclusion"],
  ["prototype", "addProductGridFloorUnderNode"],
  ["prototype", "clearNodeTint"],
  ["prototype", "subtreeNodesJson"],
  ["prototype", "setSubtreeTint"],
  ["prototype", "clearSubtreeTint"],
  ["prototype", "applyPatch"],
  ["prototype", "worldDistance"],
  ["prototype", "nodeWorldBoundsJson"],
  ["prototype", "addDistanceMeasurement"],
  ["prototype", "prepare"],
  ["prototype", "render"],
  ["prototype", "inspectJson"],
  ["prototype", "annotationProjectionsJson"],
  ["prototype", "capture"],
  ["prototype", "pick"],
  ["prototype", "setCamera"],
  ["prototype", "setCameraEased"],
  ["prototype", "getCameraJson"],
  ["prototype", "setCameraJson"],
  ["prototype", "setCameraBookmarkJson"],
  ["prototype", "cameraPointerDown"],
  ["prototype", "cameraPointerMove"],
  ["prototype", "cameraPointerUp"],
  ["prototype", "cameraWheel"],
];

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
  return "application/octet-stream";
}

function html() {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>SceneHost browser proof</title>
    <style>
      html, body {
        margin: 0;
        width: 100%;
        height: 100%;
        background: #050607;
        overflow: hidden;
      }
      #scene {
        width: ${VIEWPORT.width}px;
        height: ${VIEWPORT.height}px;
        display: block;
        background: #050607;
      }
    </style>
  </head>
  <body>
    <canvas id="scene" data-proof="scene-host-browser"></canvas>
  </body>
</html>`;
}

function serve(pkgRoot, assetRoot) {
  const server = http.createServer((request, response) => {
    const url = request.url === "/" ? "/scene-host-browser-proof.html" : request.url;
    if (url === "/scene-host-browser-proof.html") {
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      response.end(html());
      return;
    }

    let base = null;
    let relative = null;
    if (url.startsWith("/pkg/")) {
      base = pkgRoot;
      relative = url.slice("/pkg/".length);
    } else if (url.startsWith("/assets/gltf/")) {
      base = assetRoot;
      relative = url.slice("/assets/gltf/".length);
    }

    if (!base) {
      response.writeHead(404);
      response.end("not found");
      return;
    }

    const root = path.resolve(base);
    const file = path.resolve(root, path.normalize(relative));
    if (file !== root && !file.startsWith(`${root}${path.sep}`)) {
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
        url: `http://127.0.0.1:${server.address().port}/scene-host-browser-proof.html`,
      });
    });
  });
}

function buildWasmPackage() {
  const command = [
    "rustup",
    "run",
    "1.95.0",
    "wasm-pack",
    "build",
    ".",
    "--dev",
    "--target",
    "web",
    "--out-dir",
    "target/scene-host-browser-pkg",
    "--out-name",
    "scena",
    "--features",
    "scene-host",
  ];
  if (process.env.SCENA_SKIP_WASM_BUILD === "1") {
    return { command: command.join(" "), skipped: true };
  }
  childProcess.execFileSync(command[0], command.slice(1), {
    cwd: process.cwd(),
    env: {
      ...process.env,
      CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || "2",
    },
    stdio: "inherit",
  });
  return { command: command.join(" "), skipped: false };
}

function chromiumExecutablePath() {
  if (process.env.SCENA_BROWSER_EXECUTABLE) {
    return process.env.SCENA_BROWSER_EXECUTABLE;
  }
  return fs.existsSync("/usr/bin/chromium") ? "/usr/bin/chromium" : undefined;
}

function chromiumLaunchArgs() {
  return [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--ignore-gpu-blocklist",
    "--enable-gpu",
    "--use-angle=gles",
  ];
}

function assertHardwareRenderer(renderer) {
  if (!/V3D/i.test(renderer)) {
    throw new Error(`WebGL2 renderer is not V3D hardware: ${renderer}`);
  }
  if (/SwiftShader|llvmpipe/i.test(renderer)) {
    throw new Error(`WebGL2 renderer is software-backed: ${renderer}`);
  }
}

function revisionsEqual(a, b) {
  return (
    a &&
    b &&
    a.structure === b.structure &&
    a.transform === b.transform &&
    a.appearance === b.appearance &&
    a.interaction === b.interaction
  );
}

function arraysApproximatelyEqual(a, b, tolerance = 0.0001) {
  if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) {
    return false;
  }
  return a.every((value, index) => Math.abs(value - b[index]) <= tolerance);
}

function transformsApproximatelyEqual(a, b) {
  return (
    a &&
    b &&
    arraysApproximatelyEqual(a.translation, b.translation) &&
    arraysApproximatelyEqual(a.rotation, b.rotation) &&
    arraysApproximatelyEqual(a.scale, b.scale)
  );
}

function cameraStatesApproximatelyEqual(a, b, tolerance = 0.0001) {
  return (
    a &&
    b &&
    arraysApproximatelyEqual(a.target, b.target, tolerance) &&
    Math.abs(a.distance - b.distance) <= tolerance &&
    Math.abs(a.yaw_radians - b.yaw_radians) <= tolerance &&
    Math.abs(a.pitch_radians - b.pitch_radians) <= tolerance
  );
}

function nodeByHandle(report, handle) {
  return report && Array.isArray(report.nodes)
    ? report.nodes.find((node) => node.handle === handle)
    : null;
}

function fnv1a64(bytes) {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (const byte of bytes) {
    hash ^= BigInt(byte);
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, "0");
}

function summarizeRgba8(width, height, rgba8) {
  let nonblack = 0;
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (y * width + x) * 4;
      if (rgba8[offset] > 0 || rgba8[offset + 1] > 0 || rgba8[offset + 2] > 0) {
        nonblack += 1;
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }
  const centerOffset = ((Math.floor(height / 2) * width) + Math.floor(width / 2)) * 4;
  return {
    nonblack,
    bbox:
      nonblack > 0
        ? {
            min_x: minX,
            min_y: minY,
            max_x: maxX,
            max_y: maxY,
            width: maxX - minX + 1,
            height: maxY - minY + 1,
          }
        : null,
    center: Array.from(rgba8.slice(centerOffset, centerOffset + 4)),
    fnv1a64: fnv1a64(rgba8),
  };
}

function paeth(left, up, upLeft) {
  const p = left + up - upLeft;
  const pa = Math.abs(p - left);
  const pb = Math.abs(p - up);
  const pc = Math.abs(p - upLeft);
  if (pa <= pb && pa <= pc) return left;
  if (pb <= pc) return up;
  return upLeft;
}

function decodePngRgba8(bytes) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (!bytes.slice(0, 8).equals(signature)) {
    throw new Error("screenshot is not a PNG");
  }

  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let interlace = 0;
  const idat = [];
  while (offset < bytes.length) {
    const length = bytes.readUInt32BE(offset);
    const type = bytes.slice(offset + 4, offset + 8).toString("ascii");
    const data = bytes.slice(offset + 8, offset + 8 + length);
    offset += 12 + length;
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
      interlace = data[12];
    } else if (type === "IDAT") {
      idat.push(data);
    } else if (type === "IEND") {
      break;
    }
  }

  if (bitDepth !== 8 || interlace !== 0 || (colorType !== 2 && colorType !== 6)) {
    throw new Error(
      `unsupported screenshot PNG format: bitDepth=${bitDepth} colorType=${colorType} interlace=${interlace}`,
    );
  }

  const channels = colorType === 6 ? 4 : 3;
  const bytesPerPixel = channels;
  const scanline = width * channels;
  const inflated = zlib.inflateSync(Buffer.concat(idat));
  const rows = Buffer.alloc(width * height * channels);
  let source = 0;
  for (let y = 0; y < height; y += 1) {
    const filter = inflated[source];
    source += 1;
    const rowOffset = y * scanline;
    const previousRowOffset = (y - 1) * scanline;
    for (let x = 0; x < scanline; x += 1) {
      const raw = inflated[source + x];
      const left = x >= bytesPerPixel ? rows[rowOffset + x - bytesPerPixel] : 0;
      const up = y > 0 ? rows[previousRowOffset + x] : 0;
      const upLeft =
        y > 0 && x >= bytesPerPixel ? rows[previousRowOffset + x - bytesPerPixel] : 0;
      let value;
      if (filter === 0) {
        value = raw;
      } else if (filter === 1) {
        value = raw + left;
      } else if (filter === 2) {
        value = raw + up;
      } else if (filter === 3) {
        value = raw + Math.floor((left + up) / 2);
      } else if (filter === 4) {
        value = raw + paeth(left, up, upLeft);
      } else {
        throw new Error(`unsupported PNG row filter ${filter}`);
      }
      rows[rowOffset + x] = value & 0xff;
    }
    source += scanline;
  }

  const rgba8 = Buffer.alloc(width * height * 4);
  for (let pixel = 0; pixel < width * height; pixel += 1) {
    const input = pixel * channels;
    const output = pixel * 4;
    rgba8[output] = rows[input];
    rgba8[output + 1] = rows[input + 1];
    rgba8[output + 2] = rows[input + 2];
    rgba8[output + 3] = colorType === 6 ? rows[input + 3] : 255;
  }
  return { width, height, rgba8 };
}

async function runPageProof(page) {
  return page.evaluate(
    async ({ assetUrl, phase4AssetUrl, phase5AnimatedAssetUrl, backend, requiredBindings, viewport }) => {
      try {
      const mod = await import("/pkg/scena.js");
      await mod.default("/pkg/scena_bg.wasm");
      const { SceneHost } = mod;
      const fnv1a64 = (bytes) => {
        let hash = 0xcbf29ce484222325n;
        const prime = 0x100000001b3n;
        const mask = 0xffffffffffffffffn;
        for (const byte of bytes) {
          hash ^= BigInt(byte);
          hash = (hash * prime) & mask;
        }
        return hash.toString(16).padStart(16, "0");
      };
      const captureSummary = (capture) => ({
        descriptor: JSON.parse(capture.descriptorJson),
        rgba8_byte_length: capture.rgba8.length,
        rgba8_fnv1a64: fnv1a64(capture.rgba8),
      });
      const nodeByHandle = (report, handle) =>
        report && Array.isArray(report.nodes)
          ? report.nodes.find((node) => node.handle === handle)
          : null;
      const luma = (r, g, b) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
      const sampleProjectedPixel = (capture, cssX, cssY) => {
        const descriptor = JSON.parse(capture.descriptorJson);
        const width = descriptor.width;
        const height = descriptor.height;
        const dpr = window.devicePixelRatio || 1;
        const centerX = Math.round(cssX * dpr);
        const centerY = height - 1 - Math.round(cssY * dpr);
        const radius = Math.max(2, Math.round(4 * dpr));
        let maxLuma = 0;
        let minLuma = 255;
        let nonblack = 0;
        for (let y = Math.max(0, centerY - radius); y <= Math.min(height - 1, centerY + radius); y += 1) {
          for (let x = Math.max(0, centerX - radius); x <= Math.min(width - 1, centerX + radius); x += 1) {
            const offset = (y * width + x) * 4;
            const value = luma(capture.rgba8[offset], capture.rgba8[offset + 1], capture.rgba8[offset + 2]);
            maxLuma = Math.max(maxLuma, value);
            minLuma = Math.min(minLuma, value);
            if (capture.rgba8[offset] > 0 || capture.rgba8[offset + 1] > 0 || capture.rgba8[offset + 2] > 0) {
              nonblack += 1;
            }
          }
        }
        return {
          css_x: cssX,
          css_y: cssY,
          physical_x: centerX,
          physical_y: centerY,
          radius,
          max_luma: maxLuma,
          min_luma: minLuma,
          local_contrast: maxLuma - minLuma,
          nonblack,
        };
      };
      const worldBoundsFromDraw = (draw) => {
        const min = draw.local_bounds.min;
        const max = draw.local_bounds.max;
        const translation = draw.world_transform.translation;
        const scale = draw.world_transform.scale;
        return {
          min: [
            translation[0] + Math.min(min[0] * scale[0], max[0] * scale[0]),
            translation[1] + Math.min(min[1] * scale[1], max[1] * scale[1]),
            translation[2] + Math.min(min[2] * scale[2], max[2] * scale[2]),
          ],
          max: [
            translation[0] + Math.max(min[0] * scale[0], max[0] * scale[0]),
            translation[1] + Math.max(min[1] * scale[1], max[1] * scale[1]),
            translation[2] + Math.max(min[2] * scale[2], max[2] * scale[2]),
          ],
        };
      };
      const phase3GridLineFromBounds = (bounds) => {
        const spacing = 0.08;
        const centerX = (bounds.min[0] + bounds.max[0]) * 0.5;
        const width = Math.max(bounds.max[0] - bounds.min[0], spacing);
        const depth = Math.max(bounds.max[2] - bounds.min[2], spacing);
        const zDivisions = Math.max(1, Math.min(256, Math.round(depth / spacing)));
        const centerZ =
          bounds.min[2] + (depth * Math.floor(zDivisions * 0.5)) / zDivisions;
        const floorY = bounds.min[1];
        return [
          [centerX - width * 0.25, floorY, centerZ],
          [centerX, floorY, centerZ],
          [centerX + width * 0.25, floorY, centerZ],
        ];
      };
      const timedPrepare = (label) => {
        const started = performance.now();
        host.prepare();
        const ended = performance.now();
        return {
          label,
          duration_ms: ended - started,
          started_ms: started,
          ended_ms: ended,
        };
      };
      const timedRender = (label) => {
        const started = performance.now();
        const outcome = JSON.parse(host.render());
        const ended = performance.now();
        return {
          label,
          duration_ms: ended - started,
          started_ms: started,
          ended_ms: ended,
          outcome,
        };
      };
      const median = (values) => {
        const sorted = [...values].sort((left, right) => left - right);
        return sorted[Math.floor(sorted.length / 2)];
      };
      const samplePrepareRender = (label) => {
        const prepare = timedPrepare(`${label}_prepare`);
        const render = timedRender(`${label}_render`);
        return {
          label,
          prepare,
          render,
          total_ms: prepare.duration_ms + render.duration_ms,
        };
      };
      const waitForCanvasPresent = () =>
        new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const handleNumber = (value) => {
        const number = typeof value === "bigint" ? Number(value) : value;
        if (!Number.isSafeInteger(number) || number <= 0) {
          throw new Error(`invalid SceneHost handle ${String(value)}`);
        }
        return number;
      };
      const handleBigInt = (value) => (typeof value === "bigint" ? value : BigInt(value));
      const optionalHandleNumber = (value) =>
        value === null || value === undefined ? null : handleNumber(value);
      const boundsCenter = (bounds) => [
        (bounds.min[0] + bounds.max[0]) / 2,
        (bounds.min[1] + bounds.max[1]) / 2,
        (bounds.min[2] + bounds.max[2]) / 2,
      ];
      const distance3 = (a, b) =>
        Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
      const bindingStatus = requiredBindings.map(([kind, name]) => {
        const owner = kind === "static" ? SceneHost : SceneHost.prototype;
        return { kind, name, present: typeof owner[name] === "function" };
      });

      const rendererProbe = document.createElement("canvas");
      const probeGl = rendererProbe.getContext("webgl2");
      if (!probeGl) {
        throw new Error("WebGL2 probe context did not initialize");
      }
      const debugInfo = probeGl.getExtension("WEBGL_debug_renderer_info");
      if (!debugInfo) {
        throw new Error("WEBGL_debug_renderer_info is unavailable");
      }
      const webgl = {
        vendor: probeGl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL),
        renderer: probeGl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL),
      };

      const canvas = document.getElementById("scene");
      canvas.width = Math.round(viewport.width * viewport.devicePixelRatio);
      canvas.height = Math.round(viewport.height * viewport.devicePixelRatio);
      canvas.style.width = `${viewport.width}px`;
      canvas.style.height = `${viewport.height}px`;

      const host = await SceneHost.newWebgl2(
        canvas,
        viewport.width,
        viewport.height,
        viewport.devicePixelRatio,
      );
      window.__scenaSceneHostProofHost = host;
      const rootHandle = host.rootHandle();
      const leftFrameHandle = host.addEmpty(
        rootHandle,
        [-0.35, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        "frame:left",
      );
      const rightFrameHandle = host.addEmpty(
        rootHandle,
        [0.35, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        "frame:right",
      );

      const leftImportReport = JSON.parse(
        await host.instantiateUrlUnderWithReportJson(leftFrameHandle, assetUrl),
      );
      const rightImportReport = JSON.parse(
        await host.instantiateUrlUnderWithReportJson(rightFrameHandle, assetUrl),
      );
      const leftMeshHandle = host.nodeHandle(
        handleBigInt(leftImportReport.import),
        "ColoredTriangle",
      );
      const rightMeshHandle = host.nodeHandleByName(
        handleBigInt(rightImportReport.import),
        "ColoredTriangle",
      );
      const root = handleNumber(rootHandle);
      const leftFrame = handleNumber(leftFrameHandle);
      const rightFrame = handleNumber(rightFrameHandle);
      const leftMesh = handleNumber(leftMeshHandle);
      const rightMesh = handleNumber(rightMeshHandle);
      const trackedNode = leftMesh;
      const transformBatch = [
        {
          node: leftMesh,
          translation: [-0.05, 0.04, 0.0],
          rotation: [0.0, 0.0, 0.0, 1.0],
          scale: [1.0, 1.0, 1.0],
        },
        {
          node: rightMesh,
          translation: [0.05, -0.04, 0.0],
          rotation: [0.0, 0.0, 0.0, 1.0],
          scale: [1.0, 1.0, 1.0],
        },
      ];
      host.setTransforms(JSON.stringify(transformBatch));
      const typedTransformNodes = new BigUint64Array([
        handleBigInt(leftMeshHandle),
        handleBigInt(rightMeshHandle),
      ]);
      const typedTransformComponents = new Float32Array([
        -0.08, 0.06, 0.0, 0.0, 0.0, 0.0, 1.001, 1.0, 1.0, 1.0,
        0.08, -0.06, 0.0, 0.0, 0.0, 0.0, 0.999, 1.0, 1.0, 1.0,
      ]);
      host.setTransformsTyped(typedTransformNodes, typedTransformComponents);
      const afterTypedTransform = JSON.parse(host.inspectJson());
      let invalidTypedRejected = false;
      try {
        host.setTransformsTyped(
          new BigUint64Array([handleBigInt(leftMeshHandle)]),
          new Float32Array([Number.NaN, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]),
        );
      } catch (error) {
        invalidTypedRejected = error && error.code === "InvalidInput";
      }
      const afterInvalidTypedTransform = JSON.parse(host.inspectJson());
      host.setVisible(rightFrameHandle, false);
      const hiddenInspection = JSON.parse(host.inspectJson());
      host.setVisible(rightFrameHandle, true);
      const subtreeReport = JSON.parse(host.subtreeNodesJson(rootHandle));
      host.setSubtreeTint(
        rootHandle,
        0.25,
        0.5,
        0.75,
        1.0,
        new BigUint64Array([handleBigInt(rightFrameHandle)]),
      );
      const subtreeTintInspection = JSON.parse(host.inspectJson());
      host.clearSubtreeTint(rootHandle, new BigUint64Array([]));
      const partTreeSelectionPatch = JSON.parse(
        host.applyPatch(
          JSON.stringify({
            schema: "scena.visual_patch.v1",
            selection: { node: leftMesh },
          }),
        ),
      );
      const inspectionToolsBefore = JSON.parse(host.inspectJson());
      const visibleBeforeIsolate = inspectionToolsBefore.nodes
        .filter((node) => node.visible)
        .map((node) => handleBigInt(node.handle));
      host.isolate(new BigUint64Array([handleBigInt(leftFrameHandle)]));
      const isolateInspection = JSON.parse(host.inspectJson());
      for (const handle of visibleBeforeIsolate) {
        host.setVisible(handle, true);
      }
      host.ghost(leftFrameHandle, 0.35);
      const ghostInspection = JSON.parse(host.inspectJson());
      host.clearSubtreeTint(leftFrameHandle, new BigUint64Array([]));
      const beforeFitSelectionCamera = JSON.parse(host.getCameraJson());
      host.fitSelection(new BigUint64Array([handleBigInt(leftMeshHandle)]));
      const afterFitSelectionCamera = JSON.parse(host.getCameraJson());
      host.setCameraJson(JSON.stringify(beforeFitSelectionCamera));
      host.setNodeAnnotation("tracked-node", leftMeshHandle, [0.0, 0.0, 0.0]);
      host.setWorldAnnotation("origin", [0.0, 0.0, 0.0]);
      const measurementSelectedPoints = {
        left: boundsCenter(JSON.parse(host.nodeWorldBoundsJson(leftMeshHandle))),
        right: boundsCenter(JSON.parse(host.nodeWorldBoundsJson(rightMeshHandle))),
      };
      const expectedMeasurementDistance = distance3(
        measurementSelectedPoints.left,
        measurementSelectedPoints.right,
      );
      const measurementReport = JSON.parse(
        host.addDistanceMeasurement(
          "browser-gap",
          measurementSelectedPoints.left,
          measurementSelectedPoints.right,
          null,
          "mm",
          0,
        ),
      );
      const measurementInspection = JSON.parse(host.inspectJson());
      host.frameAll();

      const framedCamera = JSON.parse(host.getCameraJson());
      const measurementPrepare = timedPrepare("measurement_distance_overlay");
      const measurementRender = JSON.parse(host.render());
      await waitForCanvasPresent();
      const measurementCapture = captureSummary(host.capture());
      host.removeNode(handleBigInt(measurementReport.line_node));
      host.setCamera(
        framedCamera.target,
        framedCamera.yaw_radians,
        framedCamera.pitch_radians,
        framedCamera.distance,
      );
      const cameraActions = {
        pointer_down: host.cameraPointerDown(320, 240, "primary"),
        pointer_move: host.cameraPointerMove(328, 236, 8, -4),
        pointer_up: host.cameraPointerUp(328, 236),
        wheel: host.cameraWheel(320, 240, 40),
      };
      host.setCameraJson(JSON.stringify(framedCamera));
      const renderedCamera = JSON.parse(host.getCameraJson());
      const flyTargetCamera = {
        target: [framedCamera.target[0] + 0.04, framedCamera.target[1], framedCamera.target[2]],
        distance: framedCamera.distance * 1.08,
        yaw_radians: framedCamera.yaw_radians + 0.18,
        pitch_radians: framedCamera.pitch_radians,
      };
      host.setCameraEased(
        flyTargetCamera.target,
        flyTargetCamera.yaw_radians,
        flyTargetCamera.pitch_radians,
        flyTargetCamera.distance,
        0.5,
        "linear",
      );
      host.advance(0.25);
      const flyHalfPrepare = timedPrepare("camera_fly_to_halfway");
      const flyHalfRender = JSON.parse(host.render());
      await waitForCanvasPresent();
      const flyHalfCamera = JSON.parse(host.getCameraJson());
      const flyHalfCapture = captureSummary(host.capture());
      host.advance(0.25);
      const flyFinalPrepare = timedPrepare("camera_fly_to_final");
      const flyFinalRender = JSON.parse(host.render());
      await waitForCanvasPresent();
      const flyFinalCamera = JSON.parse(host.getCameraJson());
      const flyFinalCapture = captureSummary(host.capture());
      const bookmarkResult = JSON.parse(
        host.setCameraBookmarkJson(
          JSON.stringify({
            name: "browser-proof-framed",
            state: framedCamera,
            target_bounds: null,
            description: "restore the framed proof view after camera fly-to",
          }),
          0.0,
          "linear",
        ),
      );
      timedPrepare("camera_bookmark_restore");
      host.render();
      await waitForCanvasPresent();
      const restoredCamera = JSON.parse(host.getCameraJson());

      const phase1BeforeTintInspection = JSON.parse(host.inspectJson());
      const phase1BeforeTintPrepare = timedPrepare("before_opaque_tint");
      const phase1BeforeTintRenderOutcome = JSON.parse(host.render());
      const phase1BeforeTintCapture = captureSummary(host.capture());
      host.setNodeTint(leftMeshHandle, 1.0, 0.16, 0.08, 1.0);
      const phase1AfterTintInspection = JSON.parse(host.inspectJson());
      const phase1AfterTintPrepare = timedPrepare("after_opaque_tint");
      const renderOutcome = JSON.parse(host.render());
      const capture = captureSummary(host.capture());
      const inspectJson = JSON.parse(host.inspectJson());
      const annotationProjectionsJson = JSON.parse(host.annotationProjectionsJson());

      host.setAntiAliasing("none");
      host.setBloom(null);
      host.setAmbientOcclusion(null);
      host.setNodeTint(rightMeshHandle, 4.0, 4.0, 4.0, 1.0);
      const phase2PerfSampleCount = 9;
      const phase2OffWarmup = samplePrepareRender("phase2_post_off_warmup");
      await waitForCanvasPresent();
      const phase2OffSamples = [];
      for (let index = 0; index < phase2PerfSampleCount; index += 1) {
        phase2OffSamples.push(samplePrepareRender(`phase2_post_off_${index}`));
        await waitForCanvasPresent();
      }
      const phase2OffCapture = captureSummary(host.capture());
      const phase2OffStats = JSON.parse(host.statsJson());

      host.setAntiAliasing("fxaa");
      host.setBloom(JSON.stringify({ threshold_srgb: 208, intensity: 0.28, radius_px: 3 }));
      host.setAmbientOcclusion(
        JSON.stringify({ radius_px: 3, intensity: 0.45, depth_threshold: 0.025 }),
      );
      const phase2OnWarmup = samplePrepareRender("phase2_post_on_warmup");
      await waitForCanvasPresent();
      const phase2OnSamples = [];
      for (let index = 0; index < phase2PerfSampleCount; index += 1) {
        phase2OnSamples.push(samplePrepareRender(`phase2_post_on_${index}`));
        await waitForCanvasPresent();
      }
      const phase2OnCapture = captureSummary(host.capture());
      const phase2OnStats = JSON.parse(host.statsJson());
      const phase2CapabilityReport = JSON.parse(host.capabilitiesJson());

      const trackedAnnotation = annotationProjectionsJson.annotations.find(
        (annotation) => annotation.id === "tracked-node",
      );
      const pick = (() => {
        const candidates = [];
        if (trackedAnnotation && trackedAnnotation.visible) {
          candidates.push({
            x: trackedAnnotation.x,
            y: trackedAnnotation.y,
            source: "annotation_projection",
          });
        }
        for (let y = 12; y < viewport.height; y += 6) {
          for (let x = 12; x < viewport.width; x += 6) {
            candidates.push({ x, y, source: "css_grid" });
          }
        }
        let firstHit = null;
        for (const candidate of candidates) {
          const result = optionalHandleNumber(host.pick(candidate.x, candidate.y));
          if (result !== null && result !== undefined && firstHit === null) {
            firstHit = { ...candidate, result };
          }
          if (result === trackedNode) {
            return { ...candidate, result, expected: trackedNode };
          }
        }
        return { result: null, expected: trackedNode, first_hit: firstHit };
      })();
      const phase3GridHandles = Array.from(
        host.addProductGridFloorUnderNode(handleBigInt(leftImportReport.import)),
        handleNumber,
      );
      const afterGridInspection = JSON.parse(host.inspectJson());
      const phase3GridDraw = afterGridInspection.draw_list.find(
        (entry) => entry.node === phase3GridHandles[1],
      );
      if (!phase3GridDraw) {
        throw new Error("phase3 grid draw entry missing");
      }
      const phase3GridWorldPoints = phase3GridLineFromBounds(worldBoundsFromDraw(phase3GridDraw));
      phase3GridWorldPoints.forEach((point, index) => {
        host.setWorldAnnotation(`phase3-grid-${index}`, point);
      });
      const phase3BaseCamera = renderedCamera;
      const phase3GridViews = [0.0, 0.42, -0.42].map((yawOffset, index) => {
        host.setCamera(
          phase3BaseCamera.target,
          phase3BaseCamera.yaw_radians + yawOffset,
          phase3BaseCamera.pitch_radians,
          phase3BaseCamera.distance,
        );
        const prepare = timedPrepare(`phase3_grid_${index}_prepare`);
        const render = timedRender(`phase3_grid_${index}_render`);
        const capture = host.capture();
        const projections = JSON.parse(host.annotationProjectionsJson());
        const samples = phase3GridWorldPoints.map((_point, pointIndex) => {
          const projection = projections.annotations.find(
            (annotation) => annotation.id === `phase3-grid-${pointIndex}`,
          );
          return {
            id: `phase3-grid-${pointIndex}`,
            projection,
            sample:
              projection && projection.visible
                ? sampleProjectedPixel(capture, projection.x, projection.y)
                : null,
          };
        });
        return {
          yaw_offset: yawOffset,
          prepare,
          render,
          capture: captureSummary(capture),
          projections,
          samples,
        };
      });

      const phase4BeforeCapture = captureSummary(host.capture());
      host.setAntiAliasing("none");
      host.setBloom(null);
      host.setAmbientOcclusion(null);
      const phase4InstanceHandleBigInts = Array.from(
        await host.instantiateUrlInstanced(phase4AssetUrl, 32),
        handleBigInt,
      );
      const phase4InstanceHandles = phase4InstanceHandleBigInts.map(handleNumber);
      const phase4Components = [];
      const phase4Centers = [];
      for (let index = 0; index < phase4InstanceHandles.length; index += 1) {
        const column = index % 8;
        const row = Math.floor(index / 8);
        const x = (column - 3.5) * 0.16;
        const y = (row - 1.5) * 0.14;
        const z = -0.2;
        phase4Centers.push([x, y, z]);
        phase4Components.push(
          x,
          y,
          z,
          0.0,
          0.0,
          0.0,
          1.0,
          0.32,
          0.32,
          0.32,
        );
      }
      host.setTransformsTyped(
        new BigUint64Array(phase4InstanceHandleBigInts),
        new Float32Array(phase4Components),
      );
      host.setVisible(phase4InstanceHandleBigInts[15], false);
      host.setNodeTint(phase4InstanceHandleBigInts[0], 1.0, 0.08, 0.02, 1.0);
      host.setNodeTint(phase4InstanceHandleBigInts[31], 0.05, 0.9, 0.18, 1.0);
      let phase4TranslucentTintRejected = false;
      try {
        host.setNodeTint(phase4InstanceHandleBigInts[1], 0.1, 0.4, 1.0, 0.5);
      } catch (error) {
        phase4TranslucentTintRejected = error && error.code === "InvalidInput";
      }
      host.frameAll();
      host.setCamera([0.0, 0.0, -0.2], 0.0, 0.0, 2.2);
      phase4Centers.slice(0, 4).forEach((center, index) => {
        host.setWorldAnnotation(`phase4-center-${index}`, center);
      });
      const phase4Prepare = timedPrepare("phase4_instanced_prepare");
      const phase4Render = timedRender("phase4_instanced_render");
      const phase4Capture = captureSummary(host.capture());
      const phase4Projections = JSON.parse(host.annotationProjectionsJson()).annotations.filter(
        (entry) => entry.id && entry.id.startsWith("phase4-center-"),
      );
      const phase4Inspect = JSON.parse(host.inspectJson());
      const phase4Stats = JSON.parse(host.statsJson());
      const phase4InstanceSetRoots = (phase4Inspect.instance_sets || []).filter((binding) =>
        phase4InstanceHandles.includes(binding.root_handle),
      );

      host.setAntiAliasing("none");
      host.setBloom(null);
      host.setAmbientOcclusion(null);
      const phase5FrameHandle = host.addEmpty(
        rootHandle,
        [-0.25, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        "phase5:animation-frame",
      );
      const phase5ImportHandle = await host.instantiateUrlUnder(
        phase5FrameHandle,
        phase5AnimatedAssetUrl,
      );
      const phase5ImportHandleBig = handleBigInt(phase5ImportHandle);
      const phase5TriangleHandle = host.nodeHandleByName(
        phase5ImportHandleBig,
        "AnimatedTriangle",
      );
      const phase5Frame = handleNumber(phase5FrameHandle);
      const phase5Import = handleNumber(phase5ImportHandle);
      const phase5Triangle = handleNumber(phase5TriangleHandle);
      host.setCamera([0.0, 0.0, 0.0], 0.0, 0.0, 1.7);
      const phase5Inventory = JSON.parse(host.animationInventoryJson(phase5ImportHandleBig));
      const phase5BeforeInspection = JSON.parse(host.inspectJson());
      const phase5BeforePrepare = timedPrepare("phase5_animation_before_prepare");
      const phase5BeforeRender = timedRender("phase5_animation_before_render");
      const phase5BeforeCapture = captureSummary(host.capture());
      const phase5MixerHandle = host.playAnimation(phase5ImportHandleBig, "MoveTriangle", {
        loop_mode: "repeat",
        speed: 1.0,
      });
      host.advance(0.5);
      const phase5AfterAdvanceInspection = JSON.parse(host.inspectJson());
      const phase5AfterAdvancePrepare = timedPrepare("phase5_animation_after_advance_prepare");
      const phase5AfterAdvanceRender = timedRender("phase5_animation_after_advance_render");
      const phase5AfterAdvanceCapture = captureSummary(host.capture());
      host.pauseAnimation(phase5MixerHandle);
      host.advance(0.25);
      const phase5AfterPauseInspection = JSON.parse(host.inspectJson());
      const phase5EasedStart =
        nodeByHandle(phase5AfterPauseInspection, phase5Triangle).local_transform.translation;
      const phase5EasedTarget = [0.0, 0.32, 0.0];
      host.setTransformEased(
        phase5TriangleHandle,
        phase5EasedTarget,
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        1.0,
        "linear",
      );
      host.advance(0.5);
      const phase5AfterEasedTransformInspection = JSON.parse(host.inspectJson());
      const phase5AfterEasedTransformPrepare = timedPrepare("phase5_eased_transform_prepare");
      const phase5AfterEasedTransformRender = timedRender("phase5_eased_transform_render");
      const phase5AfterEasedTransformCapture = captureSummary(host.capture());
      host.setTransformsEasedTyped(
        new BigUint64Array([handleBigInt(phase5FrameHandle)]),
        new Float32Array([
          0.0, 0.0, 0.0,
          0.0, 0.0, 0.0, 1.0,
          1.0, 1.0, 1.0,
        ]),
        0.0,
        "linear",
      );
      const phase5AfterTypedEasedInspection = JSON.parse(host.inspectJson());
      host.setNodeTintEased(phase5TriangleHandle, 1.0, 0.05, 0.02, 1.0, 1.0, "linear");
      host.advance(0.5);
      const phase5AfterEasedTintInspection = JSON.parse(host.inspectJson());
      const phase5AfterEasedTintPrepare = timedPrepare("phase5_eased_tint_prepare");
      const phase5AfterEasedTintRender = timedRender("phase5_eased_tint_render");
      const phase5AfterEasedTintCapture = captureSummary(host.capture());
      host.clearNodeTintEased(phase5TriangleHandle, 0.0, "linear");
      host.stopAnimation(phase5MixerHandle);

      return {
        backend,
        webgl,
        wasm_bindings: bindingStatus,
        browser: {
          user_agent: navigator.userAgent,
          platform: navigator.platform,
          language: navigator.language,
        },
        viewport: {
          width: viewport.width,
          height: viewport.height,
          device_pixel_ratio: window.devicePixelRatio,
          requested_device_pixel_ratio: viewport.devicePixelRatio,
          canvas_width: canvas.width,
          canvas_height: canvas.height,
        },
        assets: [
          { url: assetUrl, role: "left", report: leftImportReport },
          { url: assetUrl, role: "right", report: rightImportReport },
          { url: phase4AssetUrl, role: "phase4-instanced" },
          { url: phase5AnimatedAssetUrl, role: "phase5-animation" },
        ],
        handles: {
          root,
          left_frame: leftFrame,
          right_frame: rightFrame,
          left_mesh: leftMesh,
          right_mesh: rightMesh,
          tracked_node: trackedNode,
          phase3_grid_floor: phase3GridHandles,
          phase4_instances: phase4InstanceHandles,
          phase5_frame: phase5Frame,
          phase5_import: phase5Import,
          phase5_triangle: phase5Triangle,
        },
        phase3_grid_inspection: afterGridInspection,
        transform_batch: transformBatch,
        typed_transform_batch: {
          nodes: Array.from(typedTransformNodes, (value) => Number(value)),
          components: Array.from(typedTransformComponents),
          invalid_rejected: invalidTypedRejected,
          after_typed_transform: afterTypedTransform,
          after_invalid_typed_transform: afterInvalidTypedTransform,
        },
        visibility_probe: hiddenInspection,
        subtree_report: subtreeReport,
        subtree_tint_probe: subtreeTintInspection,
        inspection_tools_probe: {
          selection_patch: partTreeSelectionPatch,
          before_isolate: inspectionToolsBefore,
          isolate: isolateInspection,
          ghost: ghostInspection,
          fit_selection: {
            before_camera: beforeFitSelectionCamera,
            after_camera: afterFitSelectionCamera,
          },
        },
        measurement_probe: {
          selected_points: measurementSelectedPoints,
          expected_distance: expectedMeasurementDistance,
          report: measurementReport,
          inspection: measurementInspection,
          prepare: measurementPrepare,
          render: measurementRender,
          capture: measurementCapture,
        },
        camera: {
          framed: framedCamera,
          rendered: renderedCamera,
          actions: cameraActions,
          fly_to: {
            target: flyTargetCamera,
            halfway_camera: flyHalfCamera,
            final_camera: flyFinalCamera,
            restored_camera: restoredCamera,
            bookmark_result: bookmarkResult,
            prepare_timings: [flyHalfPrepare, flyFinalPrepare],
            render_outcomes: [flyHalfRender, flyFinalRender],
            halfway_capture: flyHalfCapture,
            final_capture: flyFinalCapture,
          },
        },
        render_outcome: renderOutcome,
        capability_report: JSON.parse(host.capabilitiesJson()),
        diagnostics: JSON.parse(host.diagnosticsJson()),
        stats: JSON.parse(host.statsJson()),
        inspect_json: inspectJson,
        annotation_projections_json: annotationProjectionsJson,
        phase1_appearance_dirty_tracking: {
          before_tint_inspection: phase1BeforeTintInspection,
          after_tint_inspection: phase1AfterTintInspection,
          prepare_timings: [phase1BeforeTintPrepare, phase1AfterTintPrepare],
          before_tint_render_outcome: phase1BeforeTintRenderOutcome,
          after_tint_render_outcome: renderOutcome,
          before_tint_capture: phase1BeforeTintCapture,
          after_tint_capture: capture,
        },
        phase2_post_processing: {
          off_warmup: phase2OffWarmup,
          off_samples: phase2OffSamples,
          on_warmup: phase2OnWarmup,
          on_samples: phase2OnSamples,
          off_median_ms: median(phase2OffSamples.map((sample) => sample.total_ms)),
          on_median_ms: median(phase2OnSamples.map((sample) => sample.total_ms)),
          off_capture: phase2OffCapture,
          on_capture: phase2OnCapture,
          off_stats: phase2OffStats,
          on_stats: phase2OnStats,
          capability_report: phase2CapabilityReport,
        },
        phase3_world_strokes: {
          grid_handles: phase3GridHandles,
          world_points: phase3GridWorldPoints,
          views: phase3GridViews,
        },
        phase4_gpu_instancing: {
          handles: phase4InstanceHandles,
          hidden_middle_handle: phase4InstanceHandles[15],
          translucent_tint_rejected: phase4TranslucentTintRejected,
          prepare: phase4Prepare,
          render: phase4Render,
          before_capture: phase4BeforeCapture,
          after_capture: phase4Capture,
          projections: phase4Projections,
          stats: phase4Stats,
          inspection: phase4Inspect,
          instance_set_roots: phase4InstanceSetRoots,
        },
        phase5_animation_transitions: {
          frame: phase5Frame,
          import: phase5Import,
          triangle: phase5Triangle,
          inventory: phase5Inventory,
          mixer_handle: handleNumber(phase5MixerHandle),
          before_inspection: phase5BeforeInspection,
          after_advance_inspection: phase5AfterAdvanceInspection,
          after_pause_inspection: phase5AfterPauseInspection,
          after_eased_transform_inspection: phase5AfterEasedTransformInspection,
          after_typed_eased_inspection: phase5AfterTypedEasedInspection,
          after_eased_tint_inspection: phase5AfterEasedTintInspection,
          eased_start_translation: phase5EasedStart,
          eased_target_translation: phase5EasedTarget,
          before_prepare: phase5BeforePrepare,
          before_render: phase5BeforeRender,
          after_advance_prepare: phase5AfterAdvancePrepare,
          after_advance_render: phase5AfterAdvanceRender,
          after_eased_transform_prepare: phase5AfterEasedTransformPrepare,
          after_eased_transform_render: phase5AfterEasedTransformRender,
          after_eased_tint_prepare: phase5AfterEasedTintPrepare,
          after_eased_tint_render: phase5AfterEasedTintRender,
          before_capture: phase5BeforeCapture,
          after_advance_capture: phase5AfterAdvanceCapture,
          after_eased_transform_capture: phase5AfterEasedTransformCapture,
          after_eased_tint_capture: phase5AfterEasedTintCapture,
        },
        capture,
        pick,
      };
      } catch (error) {
        const diagnostic = {
          name: error && error.name ? error.name : typeof error,
          message: error && error.message ? error.message : String(error),
          code: error && error.code ? error.code : null,
          stack: error && error.stack ? error.stack : null,
        };
        throw new Error(`runPageProof browser evaluation failed: ${JSON.stringify(diagnostic)}`);
      }
    },
    {
      assetUrl: ASSET_URL,
      phase4AssetUrl: PHASE4_ASSET_URL,
      phase5AnimatedAssetUrl: PHASE5_ANIMATED_ASSET_URL,
      backend: BACKEND,
      requiredBindings: REQUIRED_BINDINGS,
      viewport: VIEWPORT,
    },
  );
}

function assertProof(pageProof, screenshot) {
  const assertions = {};
  const check = (name, passed, detail = null) => {
    assertions[name] = { passed: Boolean(passed), detail };
    if (!passed) {
      throw new Error(`${name} failed: ${JSON.stringify(detail)}`);
    }
  };

  const missingBindings = pageProof.wasm_bindings.filter((binding) => !binding.present);
  check("wasm_scene_host_bindings_exported", missingBindings.length === 0, missingBindings);
  check("hardware_renderer_is_v3d", /V3D/i.test(pageProof.webgl.renderer), pageProof.webgl);
  check(
    "hardware_renderer_is_not_software",
    !/SwiftShader|llvmpipe/i.test(pageProof.webgl.renderer),
    pageProof.webgl,
  );
  assertHardwareRenderer(pageProof.webgl.renderer);

  const capabilities = pageProof.capability_report.capabilities || {};
  check(
    "backend_is_webgl2",
    pageProof.backend === "webgl2" && capabilities.backend === "web_gl2",
    {
      page_backend: pageProof.backend,
      capability_backend: capabilities.backend,
    },
  );
  check(
    "forward_pbr_status_is_recorded_without_fidelity_gate",
    capabilities.forward_pbr === "supported" ||
      (capabilities.hardware_tier === "low" && capabilities.forward_pbr === "degraded"),
    {
      hardware_tier: capabilities.hardware_tier,
      forward_pbr: capabilities.forward_pbr,
      degraded_expected_on_low_tier:
        capabilities.hardware_tier === "low" && capabilities.forward_pbr === "degraded",
    },
  );
  check("viewport_uses_dpr_not_equal_one", pageProof.viewport.device_pixel_ratio !== 1, {
    viewport: pageProof.viewport,
  });

  check("screenshot_pixels_nonblank", screenshot.pixels.nonblack > 0, screenshot.pixels);
  check(
    "capture_rgba8_pixels_nonblank",
    pageProof.capture.descriptor.pixels.nonblack > 0,
    pageProof.capture.descriptor.pixels,
  );
  check(
    "capture_rgba8_length_matches_descriptor",
    pageProof.capture.rgba8_byte_length === pageProof.capture.descriptor.payload.byte_length,
    pageProof.capture,
  );
  check(
    "capture_rgba8_hash_matches_descriptor",
    pageProof.capture.rgba8_fnv1a64 === pageProof.capture.descriptor.pixels.fnv1a64,
    pageProof.capture,
  );
  const flyTo = pageProof.camera.fly_to;
  check(
    "camera_fly_to_halfway_capture_nonblank",
    flyTo.halfway_capture.descriptor.pixels.nonblack > 0,
    flyTo.halfway_capture.descriptor.pixels,
  );
  check(
    "camera_fly_to_final_capture_nonblank",
    flyTo.final_capture.descriptor.pixels.nonblack > 0,
    flyTo.final_capture.descriptor.pixels,
  );
  check(
    "camera_fly_to_final_reaches_target",
    cameraStatesApproximatelyEqual(flyTo.final_camera, flyTo.target),
    { final: flyTo.final_camera, target: flyTo.target },
  );
  check(
    "camera_bookmark_restore_uses_visual_patch",
    flyTo.bookmark_result.applied.camera_eased === 1 &&
      Array.isArray(flyTo.bookmark_result.failed) &&
      flyTo.bookmark_result.failed.length === 0,
    flyTo.bookmark_result,
  );
  check(
    "camera_bookmark_restore_returns_to_framed_view",
    cameraStatesApproximatelyEqual(flyTo.restored_camera, pageProof.camera.framed),
    { restored: flyTo.restored_camera, framed: pageProof.camera.framed },
  );
  const phase1 = pageProof.phase1_appearance_dirty_tracking;
  const phase1BeforeRevisions = phase1.before_tint_inspection.revisions;
  const phase1AfterRevisions = phase1.after_tint_inspection.revisions;
  check(
    "phase1_opaque_tint_preserves_structure_revision",
    phase1AfterRevisions.structure === phase1BeforeRevisions.structure,
    { before: phase1BeforeRevisions, after: phase1AfterRevisions },
  );
  check(
    "phase1_opaque_tint_preserves_transform_revision",
    phase1AfterRevisions.transform === phase1BeforeRevisions.transform,
    { before: phase1BeforeRevisions, after: phase1AfterRevisions },
  );
  check(
    "phase1_opaque_tint_bumps_appearance_revision",
    phase1AfterRevisions.appearance > phase1BeforeRevisions.appearance,
    { before: phase1BeforeRevisions, after: phase1AfterRevisions },
  );
  check(
    "phase1_prepare_timings_recorded",
    Array.isArray(phase1.prepare_timings) &&
      phase1.prepare_timings.length === 2 &&
      phase1.prepare_timings.every((entry) => entry.duration_ms >= 0),
    phase1.prepare_timings,
  );
  check(
    "phase1_opaque_tint_changes_rendered_pixels",
    phase1.before_tint_capture.rgba8_fnv1a64 !== phase1.after_tint_capture.rgba8_fnv1a64,
    {
      before: phase1.before_tint_capture.rgba8_fnv1a64,
      after: phase1.after_tint_capture.rgba8_fnv1a64,
    },
  );
  const phase2 = pageProof.phase2_post_processing;
  const phase2Post = phase2.capability_report.post_processing || {};
  check(
    "phase2_post_off_has_zero_gpu_post_passes",
    phase2.off_stats.fxaa_passes === 0 &&
      phase2.off_stats.bloom_passes === 0 &&
      phase2.off_stats.ambient_occlusion_passes === 0,
    phase2.off_stats,
  );
  check(
    "phase2_post_on_runs_full_gpu_chain",
    phase2.on_stats.ambient_occlusion_passes === 1 &&
      phase2.on_stats.bloom_passes === 1 &&
      phase2.on_stats.fxaa_passes === 1,
    phase2.on_stats,
  );
  check(
    "phase2_capability_report_lists_active_post_passes",
    phase2Post.anti_aliasing === true &&
      phase2Post.bloom === true &&
      phase2Post.screen_space_ambient_occlusion === true &&
      phase2Post.ssao_depth_source === "depth_color_target" &&
      Array.isArray(phase2Post.active_passes) &&
      phase2Post.active_passes.join(",") === "screen_space_ambient_occlusion,bloom,fxaa",
    phase2Post,
  );
  check(
    "phase2_full_post_chain_changes_rendered_pixels",
    phase2.off_capture.rgba8_fnv1a64 !== phase2.on_capture.rgba8_fnv1a64,
    {
      off: phase2.off_capture.rgba8_fnv1a64,
      on: phase2.on_capture.rgba8_fnv1a64,
    },
  );
  const phase3 = pageProof.phase3_world_strokes;
  check(
    "phase3_grid_floor_handles_created",
    Array.isArray(phase3.grid_handles) && phase3.grid_handles.length === 2,
    phase3.grid_handles,
  );
  check(
    "phase3_grid_floor_has_three_orbit_views",
    Array.isArray(phase3.views) && phase3.views.length === 3,
    phase3.views,
  );
  for (const [viewIndex, view] of phase3.views.entries()) {
    check(
      `phase3_grid_view_${viewIndex}_rendered`,
      view.render && view.render.outcome && view.render.outcome.skipped === false,
      view.render,
    );
    check(
      `phase3_grid_view_${viewIndex}_annotations_visible`,
      view.samples.every((entry) => entry.projection && entry.projection.visible === true),
      view.samples,
    );
    check(
      `phase3_grid_view_${viewIndex}_projected_pixels_show_grid_contrast`,
      view.samples.every(
        (entry) =>
          entry.sample &&
          entry.sample.nonblack > 0 &&
          entry.sample.local_contrast >= 2.0,
      ),
      view.samples,
    );
  }
  const phase4 = pageProof.phase4_gpu_instancing;
  check(
    "phase4_instanced_import_returns_32_handles",
    Array.isArray(phase4.handles) &&
      phase4.handles.length === 32 &&
      new Set(phase4.handles).size === 32,
    phase4.handles,
  );
  check(
    "phase4_instance_roots_appear_in_inspection",
    Array.isArray(phase4.instance_set_roots) && phase4.instance_set_roots.length === 32,
    phase4.instance_set_roots,
  );
  const phase4Hidden = phase4.instance_set_roots.find(
    (binding) => binding.root_handle === phase4.hidden_middle_handle,
  );
  check(
    "phase4_hidden_middle_instance_is_not_visible",
    phase4Hidden && phase4Hidden.visible === false,
    { hidden: phase4Hidden, hidden_middle_handle: phase4.hidden_middle_handle },
  );
  check(
    "phase4_translucent_instance_tint_rejected",
    phase4.translucent_tint_rejected === true,
    phase4,
  );
  check(
    "phase4_stats_report_instances_and_submission_counter",
    phase4.stats.instances === 31 &&
      phase4.stats.gpu_draw_submissions > 0 &&
      phase4.stats.gpu_draw_submissions < phase4.stats.triangles,
    phase4.stats,
  );
  check(
    "phase4_instanced_render_changes_pixels",
    phase4.after_capture.descriptor.pixels.nonblack > 0 &&
      phase4.before_capture.rgba8_fnv1a64 !== phase4.after_capture.rgba8_fnv1a64,
    {
      before: phase4.before_capture.rgba8_fnv1a64,
      after: phase4.after_capture.rgba8_fnv1a64,
      pixels: phase4.after_capture.descriptor.pixels,
      capture_camera: phase4.after_capture.descriptor.camera,
      render: phase4.render,
      stats: phase4.stats,
      active_camera: phase4.inspection.nodes.find(
        (node) => node.handle === phase4.inspection.active_camera,
      ),
      instance_set_root_sample: phase4.instance_set_roots.slice(0, 4),
      instance_draw_sample: phase4.inspection.draw_list
        .filter((entry) => entry.instance !== null)
        .slice(0, 4),
      projections: phase4.projections,
      console: pageProof.console_messages,
      draw_sample: phase4.inspection.draw_list.slice(0, 4),
    },
  );
  const phase5 = pageProof.phase5_animation_transitions;
  const phase5Clip =
    phase5.inventory && Array.isArray(phase5.inventory.clips)
      ? phase5.inventory.clips.find((clip) => clip.name === "MoveTriangle")
      : null;
  const phase5BeforeNode = nodeByHandle(phase5.before_inspection, phase5.triangle);
  const phase5AdvancedNode = nodeByHandle(phase5.after_advance_inspection, phase5.triangle);
  const phase5PausedNode = nodeByHandle(phase5.after_pause_inspection, phase5.triangle);
  const phase5EasedNode = nodeByHandle(phase5.after_eased_transform_inspection, phase5.triangle);
  const phase5TypedFrame = nodeByHandle(phase5.after_typed_eased_inspection, phase5.frame);
  const phase5TintedNode = nodeByHandle(phase5.after_eased_tint_inspection, phase5.triangle);
  const phase5ExpectedEasedTranslation = phase5.eased_start_translation.map(
    (value, index) => value + (phase5.eased_target_translation[index] - value) * 0.5,
  );
  check(
    "phase5_animation_inventory_schema_and_clip_exposed",
    phase5.inventory.schema === "scena.animation_inventory.v1" &&
      phase5Clip &&
      phase5Clip.duration_seconds === 1 &&
      phase5Clip.channel_count === 1,
    phase5.inventory,
  );
  check(
    "phase5_animation_advance_moves_node_transform",
    phase5BeforeNode &&
      phase5AdvancedNode &&
      Math.abs(
        phase5AdvancedNode.local_transform.translation[0] -
          phase5BeforeNode.local_transform.translation[0],
      ) > 0.2,
    {
      before: phase5BeforeNode && phase5BeforeNode.local_transform,
      after: phase5AdvancedNode && phase5AdvancedNode.local_transform,
    },
  );
  check(
    "phase5_animation_visible_capture_changes_after_advance",
    phase5.before_capture.rgba8_fnv1a64 !== phase5.after_advance_capture.rgba8_fnv1a64 &&
      phase5.after_advance_capture.descriptor.pixels.nonblack > 0,
    {
      before: phase5.before_capture.rgba8_fnv1a64,
      after: phase5.after_advance_capture.rgba8_fnv1a64,
      pixels: phase5.after_advance_capture.descriptor.pixels,
    },
  );
  check(
    "phase5_pause_freezes_animation_transform",
    transformsApproximatelyEqual(
      phase5AdvancedNode && phase5AdvancedNode.local_transform,
      phase5PausedNode && phase5PausedNode.local_transform,
    ),
    {
      after_advance: phase5AdvancedNode && phase5AdvancedNode.local_transform,
      after_pause: phase5PausedNode && phase5PausedNode.local_transform,
    },
  );
  check(
    "phase5_eased_transform_hits_linear_midpoint",
    phase5EasedNode &&
      arraysApproximatelyEqual(
        phase5EasedNode.local_transform.translation,
        phase5ExpectedEasedTranslation,
        0.0001,
      ),
    {
      expected: phase5ExpectedEasedTranslation,
      actual: phase5EasedNode && phase5EasedNode.local_transform.translation,
    },
  );
  check(
    "phase5_typed_eased_transform_batch_applies_zero_duration_update",
    phase5TypedFrame &&
      arraysApproximatelyEqual(phase5TypedFrame.local_transform.translation, [0, 0, 0], 0.0001),
    phase5TypedFrame && phase5TypedFrame.local_transform,
  );
  check(
    "phase5_eased_tint_sets_dynamic_tint_and_changes_pixels",
    phase5TintedNode &&
      phase5TintedNode.tint &&
      phase5TintedNode.tint.r > 0.99 &&
      phase5.after_eased_transform_capture.rgba8_fnv1a64 !==
        phase5.after_eased_tint_capture.rgba8_fnv1a64,
    {
      tint: phase5TintedNode && phase5TintedNode.tint,
      before: phase5.after_eased_transform_capture.rgba8_fnv1a64,
      after: phase5.after_eased_tint_capture.rgba8_fnv1a64,
    },
  );
  check(
    "phase5_browser_renders_all_animation_transition_steps",
    [
      phase5.before_render,
      phase5.after_advance_render,
      phase5.after_eased_transform_render,
      phase5.after_eased_tint_render,
    ].every((entry) => entry && entry.outcome && entry.outcome.skipped === false),
    {
      before: phase5.before_render,
      after_advance: phase5.after_advance_render,
      after_eased_transform: phase5.after_eased_transform_render,
      after_eased_tint: phase5.after_eased_tint_render,
    },
  );
  check(
    "phase2_post_performance_budget_within_25_percent",
    phase2.off_samples.length >= 5 &&
      phase2.on_samples.length >= 5 &&
      phase2.on_median_ms <= phase2.off_median_ms * 1.25,
    {
      off_median_ms: phase2.off_median_ms,
      on_median_ms: phase2.on_median_ms,
      ratio: phase2.off_median_ms > 0 ? phase2.on_median_ms / phase2.off_median_ms : null,
      off_samples: phase2.off_samples.map((sample) => sample.total_ms),
      on_samples: phase2.on_samples.map((sample) => sample.total_ms),
    },
  );

  const tracked = pageProof.handles.tracked_node;
  const transformed = pageProof.transform_batch.some((entry) => entry.node === tracked);
  const typedTransformed = pageProof.typed_transform_batch.nodes.includes(tracked);
  const inspectedNode = pageProof.inspect_json.nodes.find((node) => node.handle === tracked);
  const draw = pageProof.inspect_json.draw_list.find((entry) => entry.node === tracked);
  const annotation = pageProof.annotation_projections_json.annotations.find(
    (entry) => entry.node_handle === tracked,
  );
  check("tracked_handle_appears_in_set_transforms", transformed, pageProof.transform_batch);
  check(
    "tracked_handle_appears_in_set_transforms_typed",
    typedTransformed,
    pageProof.typed_transform_batch,
  );
  check(
    "invalid_typed_transform_rejected",
    pageProof.typed_transform_batch.invalid_rejected === true,
    pageProof.typed_transform_batch,
  );
  const afterTypedNode = pageProof.typed_transform_batch.after_typed_transform.nodes.find(
    (node) => node.handle === tracked,
  );
  const afterInvalidTypedNode =
    pageProof.typed_transform_batch.after_invalid_typed_transform.nodes.find(
      (node) => node.handle === tracked,
    );
  check(
    "invalid_typed_transform_does_not_mutate",
    transformsApproximatelyEqual(
      afterTypedNode && afterTypedNode.local_transform,
      afterInvalidTypedNode && afterInvalidTypedNode.local_transform,
    ),
    {
      after_typed: afterTypedNode && afterTypedNode.local_transform,
      after_invalid: afterInvalidTypedNode && afterInvalidTypedNode.local_transform,
    },
  );
  check("tracked_handle_appears_in_inspection_nodes", Boolean(inspectedNode), tracked);
  check("tracked_handle_appears_in_draw_list", Boolean(draw), pageProof.inspect_json.draw_list);
  check(
    "tracked_handle_appears_in_annotation_projection",
    Boolean(annotation),
    pageProof.annotation_projections_json,
  );
  check("pick_returns_tracked_handle_from_css_pixels", pageProof.pick.result === tracked, {
    pick: pageProof.pick,
    dpr: pageProof.viewport.device_pixel_ratio,
  });
  const hiddenRightFrame = pageProof.visibility_probe.nodes.find(
    (node) => node.handle === pageProof.handles.right_frame,
  );
  const hiddenRightDraw = pageProof.visibility_probe.draw_list.find(
    (entry) => entry.node === pageProof.handles.right_mesh,
  );
  check("set_visible_hides_subtree_in_inspection", hiddenRightFrame && !hiddenRightFrame.visible, {
    visibility_probe: pageProof.visibility_probe,
  });
  check("set_visible_hides_subtree_draws", !hiddenRightDraw, {
    visibility_probe: pageProof.visibility_probe,
  });
  check("subtree_report_schema_is_versioned", pageProof.subtree_report.schema === "scena.subtree.v1", {
    subtree_report: pageProof.subtree_report,
  });
  check(
    "subtree_report_contains_tracked_handle",
    pageProof.subtree_report.nodes.some((node) => node.handle === tracked),
    pageProof.subtree_report,
  );
  const rootTreeNode = pageProof.subtree_report.nodes.find(
    (node) => node.handle === pageProof.handles.root,
  );
  const leftFrameTreeNode = pageProof.subtree_report.nodes.find(
    (node) => node.handle === pageProof.handles.left_frame,
  );
  check(
    "subtree_report_exposes_part_tree_edges",
    rootTreeNode &&
      rootTreeNode.children.includes(pageProof.handles.left_frame) &&
      leftFrameTreeNode &&
      leftFrameTreeNode.parent === pageProof.handles.root &&
      leftFrameTreeNode.children.includes(tracked),
    {
      root: rootTreeNode,
      left_frame: leftFrameTreeNode,
    },
  );
  const tintedLeft = pageProof.subtree_tint_probe.nodes.find((node) => node.handle === tracked);
  const excludedRight = pageProof.subtree_tint_probe.nodes.find(
    (node) => node.handle === pageProof.handles.right_mesh,
  );
  check("set_subtree_tint_applies_to_included_subtree", Boolean(tintedLeft && tintedLeft.tint), {
    subtree_tint_probe: pageProof.subtree_tint_probe,
  });
  check("set_subtree_tint_skips_excluded_subtree", Boolean(excludedRight && !excludedRight.tint), {
    subtree_tint_probe: pageProof.subtree_tint_probe,
  });
  check(
    "part_tree_selection_patch_applies",
    pageProof.inspection_tools_probe.selection_patch.applied.selection === 1 &&
      pageProof.inspection_tools_probe.selection_patch.failed.length === 0,
    pageProof.inspection_tools_probe.selection_patch,
  );
  const isolatedLeftFrame = pageProof.inspection_tools_probe.isolate.nodes.find(
    (node) => node.handle === pageProof.handles.left_frame,
  );
  const isolatedRightFrame = pageProof.inspection_tools_probe.isolate.nodes.find(
    (node) => node.handle === pageProof.handles.right_frame,
  );
  const isolatedRightDraw = pageProof.inspection_tools_probe.isolate.draw_list.find(
    (entry) => entry.node === pageProof.handles.right_mesh,
  );
  check(
    "isolate_keeps_selected_part_tree_node_visible",
    isolatedLeftFrame && isolatedLeftFrame.visible,
    pageProof.inspection_tools_probe.isolate,
  );
  check(
    "isolate_hides_unrelated_part_tree_node",
    isolatedRightFrame && !isolatedRightFrame.visible && !isolatedRightDraw,
    pageProof.inspection_tools_probe.isolate,
  );
  const ghostedLeft = pageProof.inspection_tools_probe.ghost.nodes.find(
    (node) => node.handle === tracked,
  );
  check(
    "ghost_applies_alpha_tint_to_selected_subtree",
    ghostedLeft && ghostedLeft.tint && Math.abs(ghostedLeft.tint.a - 0.35) <= 0.0001,
    ghostedLeft,
  );
  check(
    "fit_selection_frames_selected_part_tree_node",
    pageProof.inspection_tools_probe.fit_selection.after_camera.target[0] <
      pageProof.inspection_tools_probe.fit_selection.before_camera.target[0] - 0.1 &&
      Number.isFinite(pageProof.inspection_tools_probe.fit_selection.after_camera.distance) &&
      pageProof.inspection_tools_probe.fit_selection.after_camera.distance > 0,
    pageProof.inspection_tools_probe.fit_selection,
  );
  const measurementDraw = pageProof.measurement_probe.inspection.draw_list.find(
    (entry) => entry.node === pageProof.measurement_probe.report.line_node,
  );
  check(
    "distance_measurement_report_uses_selected_points",
    pageProof.measurement_probe.report.schema === "scena.scene_host_measurement_overlay.v1" &&
      pageProof.measurement_probe.report.kind === "distance" &&
      Math.abs(
        pageProof.measurement_probe.report.value - pageProof.measurement_probe.expected_distance,
      ) <= 0.0001 &&
      pageProof.measurement_probe.report.formatted_value.endsWith("mm") &&
      pageProof.measurement_probe.report.label_text === undefined,
    pageProof.measurement_probe,
  );
  check(
    "distance_measurement_line_is_in_browser_draw_list",
    measurementDraw && measurementDraw.material && measurementDraw.material.kind === "line",
    {
      report: pageProof.measurement_probe.report,
      draw: measurementDraw,
      draw_list: pageProof.measurement_probe.inspection.draw_list,
    },
  );
  check(
    "distance_measurement_overlay_renders_pixels",
    pageProof.measurement_probe.capture.descriptor.pixels.nonblack > 40,
    pageProof.measurement_probe.capture.descriptor.pixels,
  );

  check(
    "capture_revisions_match_inspection",
    revisionsEqual(pageProof.capture.descriptor.revisions, pageProof.inspect_json.revisions),
    {
      capture: pageProof.capture.descriptor.revisions,
      inspection: pageProof.inspect_json.revisions,
    },
  );
  check("capture_camera_is_active", pageProof.capture.descriptor.camera.active === true, {
    capture_camera: pageProof.capture.descriptor.camera,
  });
  const activeCamera = pageProof.inspect_json.nodes.find(
    (node) => node.handle === pageProof.inspect_json.active_camera,
  );
  check("inspection_active_camera_exists", Boolean(activeCamera), {
    active_camera: pageProof.inspect_json.active_camera,
  });
  check(
    "capture_camera_matches_inspection_active_camera",
    transformsApproximatelyEqual(
      pageProof.capture.descriptor.camera.world_transform,
      activeCamera && activeCamera.world_transform,
    ),
    {
      capture: pageProof.capture.descriptor.camera.world_transform,
      inspection: activeCamera && activeCamera.world_transform,
    },
  );

  return assertions;
}

async function main() {
  const selectedBackends = (process.env.SCENA_BROWSER_BACKENDS || BACKEND)
    .split(",")
    .map((backend) => backend.trim())
    .filter(Boolean);
  if (selectedBackends.length !== 1 || selectedBackends[0] !== BACKEND) {
    throw new Error(
      `scene-host browser proof is restricted to SCENA_BROWSER_BACKENDS=${BACKEND}, got ${selectedBackends.join(",")}`,
    );
  }

  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
  const build = buildWasmPackage();
  const { chromium } = loadPlaywright();
  const { server, url } = await serve(PKG_DIR, path.join(process.cwd(), "tests", "assets", "gltf"));
  const executablePath = chromiumExecutablePath();
  const browser = await chromium.launch({
    headless: true,
    executablePath,
    args: chromiumLaunchArgs(),
  });
  const browserVersion = browser.version();

  const captureCanvasScreenshot = async (page, source) => {
    await page.locator("#scene").screenshot({ path: SCREENSHOT_PATH });
    const screenshotBytes = fs.readFileSync(SCREENSHOT_PATH);
    const decoded = decodePngRgba8(screenshotBytes);
    return {
      path: path.relative(process.cwd(), SCREENSHOT_PATH),
      source,
      sha256: crypto.createHash("sha256").update(screenshotBytes).digest("hex"),
      byte_length: screenshotBytes.length,
      width: decoded.width,
      height: decoded.height,
      pixels: summarizeRgba8(decoded.width, decoded.height, decoded.rgba8),
    };
  };

  let pageProof;
  let screenshot;
  const consoleMessages = [];
  try {
    const page = await browser.newPage({
      viewport: { width: VIEWPORT.width, height: VIEWPORT.height },
      deviceScaleFactor: VIEWPORT.devicePixelRatio,
    });
    page.on("console", (message) => {
      consoleMessages.push(`${message.type()}: ${message.text()}`);
    });
    page.on("pageerror", (error) => {
      consoleMessages.push(`pageerror: ${error.message}`);
    });
    try {
      await page.goto(url);
      pageProof = await runPageProof(page);
      pageProof.console_messages = consoleMessages.slice();
      const finalRender = await page.evaluate(
        async () => {
          try {
            window.__scenaSceneHostProofHost.prepare();
            window.__scenaSceneHostProofHost.render();
            await new Promise((resolve) =>
              requestAnimationFrame(() => requestAnimationFrame(resolve)),
            );
            return { ok: true };
          } catch (error) {
            return {
              ok: false,
              name: error && error.name ? error.name : typeof error,
              message: error && error.message ? error.message : String(error),
              code: error && error.code ? error.code : null,
              stack: error && error.stack ? error.stack : null,
            };
          }
        },
      );
      if (!finalRender.ok) {
        throw new Error(`final browser render failed: ${JSON.stringify(finalRender)}`);
      }
      screenshot = await captureCanvasScreenshot(page, "webgl_canvas");
    } finally {
      await page.close();
    }
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }

  const assertions = assertProof(pageProof, screenshot);
  const artifact = {
    schema: SCHEMA,
    status: "passed",
    generated_at: new Date().toISOString(),
    build,
    harness: {
      entrypoint: "tests/browser/scene_host_browser_proof.js",
      command: "SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof",
      server_url: url,
    },
    browser: {
      engine: "chromium",
      executable_path: executablePath || "playwright-bundled-chromium",
      launch_args: chromiumLaunchArgs(),
      version: browserVersion,
      os: {
        platform: os.platform(),
        release: os.release(),
        arch: os.arch(),
      },
      ...pageProof.browser,
      webgl: pageProof.webgl,
    },
    backend: pageProof.backend,
    capability_report: pageProof.capability_report,
    diagnostics: pageProof.diagnostics,
    stats: pageProof.stats,
    viewport: pageProof.viewport,
    assets: pageProof.assets,
    wasm_bindings: pageProof.wasm_bindings,
    handles: pageProof.handles,
    transform_batch: pageProof.transform_batch,
    typed_transform_batch: pageProof.typed_transform_batch,
    visibility_probe: pageProof.visibility_probe,
    subtree_report: pageProof.subtree_report,
    subtree_tint_probe: pageProof.subtree_tint_probe,
    inspection_tools_probe: pageProof.inspection_tools_probe,
    measurement_probe: pageProof.measurement_probe,
    phase1_appearance_dirty_tracking: pageProof.phase1_appearance_dirty_tracking,
    phase2_post_processing: pageProof.phase2_post_processing,
    phase3_world_strokes: pageProof.phase3_world_strokes,
    phase3_grid_inspection: pageProof.phase3_grid_inspection,
    phase4_gpu_instancing: pageProof.phase4_gpu_instancing,
    phase5_animation_transitions: pageProof.phase5_animation_transitions,
    camera: pageProof.camera,
    render_outcome: pageProof.render_outcome,
    inspect_json: pageProof.inspect_json,
    annotation_projections_json: pageProof.annotation_projections_json,
    capture: pageProof.capture,
    pick: pageProof.pick,
    screenshot,
    assertions,
    console: consoleMessages,
    notes: {
      host_render_cadence: "push-driven prepare/render; no requestAnimationFrame loop",
      proof_scope: "SceneHost browser contracts and rendered output on Pi V3D hardware",
      forward_pbr_status: pageProof.capability_report.capabilities.forward_pbr,
      forward_pbr_degraded_expected_on_low_tier:
        pageProof.capability_report.capabilities.hardware_tier === "low" &&
        pageProof.capability_report.capabilities.forward_pbr === "degraded",
      forward_pbr_supported_on_this_run:
        pageProof.capability_report.capabilities.forward_pbr === "supported",
      renderer_fidelity_epics_out_of_scope:
        "dense PBR/source-material fidelity still requires a non-Pi GPU lane",
    },
  };
  fs.writeFileSync(ARTIFACT_PATH, `${JSON.stringify(artifact, null, 2)}\n`);
  console.log(JSON.stringify({
    schema: SCHEMA,
    status: "passed",
    artifact: path.relative(process.cwd(), ARTIFACT_PATH),
    screenshot: screenshot.path,
    renderer: pageProof.webgl.renderer,
    hardware_tier: pageProof.capability_report.capabilities.hardware_tier,
    forward_pbr: pageProof.capability_report.capabilities.forward_pbr,
  }, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

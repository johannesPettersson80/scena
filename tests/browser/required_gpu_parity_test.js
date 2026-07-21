const assert = require("assert");
const {
  evaluateRequiredGpuParity,
  evaluateRequiredHardwareAdapter,
} = require("./required_gpu_parity.js");
const {
  browserEngineForBackend,
  chromiumArgsForPlatform,
  sanitizeChromiumGpuInfo,
} = require("./hardware_browser.js");

function result(adapter, drawCalls = 1, nonblack = 64, parity = null) {
  const gpuFrame = parity && parity.gpu_frame;
  return {
    backend: "WebGpu",
    status: "passed",
    gpu_device: true,
    draw_calls: drawCalls,
    gpu_submissions: drawCalls,
    adapter,
    renderer_readback: {
      source: "renderer-owned-gpu-copy",
      width: gpuFrame && gpuFrame.width,
      height: gpuFrame && gpuFrame.height,
      rgba8_base64: gpuFrame && gpuFrame.rgba8_base64,
      pixel_statistics: { nonblack },
    },
    parity,
  };
}

function parityFixture(gpuTransform = (rgba) => rgba) {
  const width = 32;
  const height = 32;
  const cpu = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (y * width + x) * 4;
      const foreground = y >= 4 && y < 28 && x >= 4 && x <= 4 + (y - 4) / 2;
      cpu[offset] = foreground ? 190 : 0;
      cpu[offset + 1] = foreground ? 105 : 0;
      cpu[offset + 2] = foreground ? 45 : 0;
      cpu[offset + 3] = 255;
    }
  }
  const gpu = gpuTransform(Buffer.from(cpu));
  return {
    schema: "scena.m6.cpu_webgpu_parity.v1",
    status: "passed",
    backend: "WebGpu",
    fixture: { id: "m6-identical-unlit-triangle-v1" },
    normalization: {
      row_origin: "top-left",
      transfer: "srgb8",
      alpha: "straight-opaque",
      dimensions: "exact",
      width,
      height,
      comparison_channels: "rgb",
    },
    cpu_frame: {
      source: "renderer-owned-cpu-frame",
      width,
      height,
      rgba8_base64: cpu.toString("base64"),
    },
    gpu_frame: {
      source: "renderer-owned-gpu-copy",
      width,
      height,
      rgba8_base64: gpu.toString("base64"),
    },
  };
}

assert.deepStrictEqual(
  evaluateRequiredGpuParity({ required: true, requestedBackend: "webgpu", error: "NoAdapter" })
    .failure_codes,
  ["NO_ADAPTER"],
);
assert(
  evaluateRequiredGpuParity({
    required: true,
    requestedBackend: "webgpu",
    result: result({ name: "NVIDIA RTX", device_type: "DiscreteGpu" }, 0, 0),
  }).failure_codes.includes("ZERO_RENDERER_OUTPUT"),
);
assert(
  evaluateRequiredGpuParity({
    required: true,
    requestedBackend: "webgpu",
    result: result({ name: "unknown adapter", device_type: "Other" }),
  }).failure_codes.includes("ADAPTER_HARDWARE_UNPROVEN"),
);
assert(
  evaluateRequiredGpuParity({
    required: true,
    requestedBackend: "webgpu",
    result: result({ name: "Google SwiftShader", device_type: "Cpu" }),
  }).failure_codes.includes("SOFTWARE_ADAPTER"),
);
const requiredHardwarePixels = evaluateRequiredGpuParity({
  required: true,
  requestedBackend: "webgpu",
  result: result(
    { name: "NVIDIA RTX", device_type: "DiscreteGpu" },
    1,
    64,
    parityFixture(),
  ),
});
assert.strictEqual(
  requiredHardwarePixels.status,
  "passed",
  JSON.stringify(requiredHardwarePixels, null, 2),
);

const missingPixels = evaluateRequiredGpuParity({
  required: true,
  requestedBackend: "webgpu",
  result: result({ name: "NVIDIA RTX", device_type: "DiscreteGpu" }),
});
assert(
  missingPixels.failure_codes.includes("PIXEL_PARITY_MISSING"),
  "nonblack renderer output must not pass required WebGPU parity without reference pixels",
);

const correctPixels = evaluateRequiredGpuParity({
  required: true,
  requestedBackend: "webgpu",
  result: result(
    { name: "NVIDIA RTX", device_type: "DiscreteGpu" },
    1,
    64,
    parityFixture(),
  ),
});
assert.strictEqual(correctPixels.pixel_parity.status, "passed");
assert.deepStrictEqual(
  correctPixels.pixel_parity.mutations.map((mutation) => mutation.name),
  [
    "wrong-colors",
    "geometry-shift",
    "missing-object",
    "vertical-flip",
    "linear-as-srgb",
    "stale-reference",
  ],
);
assert(correctPixels.pixel_parity.mutations.every((mutation) => mutation.rejected === true));
assert.strictEqual(correctPixels.pixel_parity.normalization.transfer, "srgb8");
assert.strictEqual(correctPixels.pixel_parity.mask.kind, "two-pixel-gradient-edge-exclusion");
assert.strictEqual(correctPixels.pixel_parity.mask.source, "cpu-reference-gradient");
assert.strictEqual(correctPixels.pixel_parity.mask.foreground_domain, "edge-excluded");
assert(Array.isArray(correctPixels.pixel_parity.worst_region.bbox));
assert.strictEqual(typeof correctPixels.pixel_parity.diff_heatmap_rgba8_base64, "string");

const softwareConformance = evaluateRequiredGpuParity({
  required: false,
  requestedBackend: "webgpu",
  result: result(
    { name: "Google SwiftShader", device_type: "Cpu" },
    1,
    64,
    parityFixture(),
  ),
});
assert.strictEqual(softwareConformance.status, "diagnostic");
assert.strictEqual(softwareConformance.pixel_parity.status, "passed");

const wrongSoftwarePixels = evaluateRequiredGpuParity({
  required: false,
  requestedBackend: "webgpu",
  result: result(
    { name: "Google SwiftShader", device_type: "Cpu" },
    1,
    64,
    parityFixture((gpu) => {
      gpu.fill(0);
      for (let offset = 3; offset < gpu.length; offset += 4) gpu[offset] = 255;
      return gpu;
    }),
  ),
});
assert.strictEqual(
  wrongSoftwarePixels.status,
  "failed",
  "software conformance must evaluate pixels without claiming hardware parity",
);

const edgeCoverageVariant = evaluateRequiredGpuParity({
  required: true,
  requestedBackend: "webgpu",
  result: result(
    { name: "NVIDIA RTX", device_type: "DiscreteGpu" },
    1,
    64,
    parityFixture((gpu) => {
      const width = 32;
      const height = 32;
      for (let y = 0; y < height; y += 1) {
        let rightmost = -1;
        for (let x = 0; x < width; x += 1) {
          if (gpu[(y * width + x) * 4] > 0) rightmost = x;
        }
        if (rightmost >= 0) {
          const offset = (y * width + rightmost) * 4;
          gpu[offset] = 0;
          gpu[offset + 1] = 0;
          gpu[offset + 2] = 0;
        }
      }
      return gpu;
    }),
  ),
});
assert.strictEqual(
  edgeCoverageVariant.status,
  "passed",
  "declared edge masking must tolerate one-pixel raster fill-convention differences",
);

const detachedReadbackResult = result(
  { name: "NVIDIA RTX", device_type: "DiscreteGpu" },
  1,
  64,
  parityFixture(),
);
detachedReadbackResult.renderer_readback.rgba8_base64 = Buffer.alloc(32 * 32 * 4).toString("base64");
assert(
  evaluateRequiredGpuParity({
    required: true,
    requestedBackend: "webgpu",
    result: detachedReadbackResult,
  }).failure_codes.includes("PIXEL_PARITY_MALFORMED"),
  "the compared GPU candidate must be the renderer headline readback",
);

const wrongColors = evaluateRequiredGpuParity({
  required: true,
  requestedBackend: "webgpu",
  result: result(
    { name: "NVIDIA RTX", device_type: "DiscreteGpu" },
    1,
    64,
    parityFixture((gpu) => {
      for (let offset = 0; offset < gpu.length; offset += 4) {
        if (gpu[offset] > 0) {
          gpu[offset] = 20;
          gpu[offset + 1] = 210;
          gpu[offset + 2] = 230;
        }
      }
      return gpu;
    }),
  ),
});
assert.strictEqual(wrongColors.status, "failed");
assert(wrongColors.failure_codes.includes("PIXEL_PARITY_MISMATCH"));

assert.deepStrictEqual(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgl2",
    actualBackend: "WebGl2",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: { name: "Google SwiftShader", device_type: "Cpu" },
  }).failure_codes,
  ["SOFTWARE_ADAPTER"],
);
assert.deepStrictEqual(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgpu",
    actualBackend: "WebGl2",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: { name: "Apple M2", device_type: "IntegratedGpu" },
  }).failure_codes,
  ["BACKEND_MISMATCH"],
);
assert.strictEqual(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgpu",
    actualBackend: "WebGpu",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: { name: "Apple M2", device_type: "IntegratedGpu" },
  }).status,
  "passed",
);

const redactedWebGpuAdapter = {
  name: "",
  device_type: "Other",
  vendor: 0,
  device: 0,
  driver: "",
  driver_info: "",
};
const chromiumNvidiaGpu = {
  source: "chromium-cdp-system-info",
  devices: [{
    vendor_id: 0x10de,
    device_id: 0x1f99,
    vendor_string: "NVIDIA",
    device_string: "NVIDIA RTX 1000 Ada Generation Laptop GPU",
    driver_vendor: "NVIDIA",
    driver_version: "32.0.15.9647",
  }],
  aux_attributes: {
    gl_renderer: "ANGLE (NVIDIA, NVIDIA RTX 1000 Ada Generation Laptop GPU Direct3D11)",
    gl_vendor: "Google Inc. (NVIDIA)",
  },
  feature_status: { webgpu: "enabled", webgl2: "enabled" },
};
assert.deepStrictEqual(
  sanitizeChromiumGpuInfo({
    gpu: {
      devices: [{
        vendorId: 0x10de,
        deviceId: 0x1f99,
        vendorString: "NVIDIA",
        deviceString: "NVIDIA RTX 1000 Ada Generation Laptop GPU",
        driverVendor: "NVIDIA",
        driverVersion: "32.0.15.9647",
      }],
      auxAttributes: {
        glRenderer: "ANGLE (NVIDIA, NVIDIA RTX 1000 Ada Generation Laptop GPU Direct3D11)",
        glVendor: "Google Inc. (NVIDIA)",
      },
      featureStatus: { webgpu: "enabled" },
    },
  }),
  {
    ...chromiumNvidiaGpu,
    feature_status: { webgpu: "enabled" },
  },
  "the persisted CDP report must contain only normalized GPU attestation fields",
);
assert.strictEqual(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgpu",
    actualBackend: "WebGpu",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: redactedWebGpuAdapter,
    browserGpu: chromiumNvidiaGpu,
  }).status,
  "passed",
  "a strict same-browser CDP GPU report must attest privacy-redacted WebGPU adapters",
);
const chromiumHybridGpu = {
  source: "chromium-cdp-system-info",
  devices: [
    {
      vendor_id: 0x8086,
      device_id: 0x7d55,
      vendor_string: "",
      device_string: "Intel(R) Arc(TM) Pro Graphics",
      driver_vendor: "Intel",
      driver_version: "32.0.101.8517",
    },
    {
      vendor_id: 0x10de,
      device_id: 0x28b9,
      vendor_string: "",
      device_string: "NVIDIA RTX 1000 Ada Generation Laptop GPU",
      driver_vendor: "",
      driver_version: "32.0.15.9647",
    },
    {
      vendor_id: 0x1414,
      device_id: 0x008c,
      vendor_string: "",
      device_string: "Microsoft Basic Render Driver",
      driver_vendor: "",
      driver_version: "10.0.26100.8521",
    },
  ],
  aux_attributes: {
    gl_vendor: "Google Inc. (Intel)",
    gl_renderer: "ANGLE (Intel, Intel(R) Arc(TM) Pro Graphics Direct3D11)",
  },
  feature_status: { webgpu: "enabled", webgl: "enabled" },
};
assert.strictEqual(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgpu",
    actualBackend: "WebGpu",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: redactedWebGpuAdapter,
    browserGpu: chromiumHybridGpu,
  }).status,
  "passed",
  "an inactive Microsoft fallback must not override the active physical CDP renderer",
);
assert(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgpu",
    actualBackend: "WebGpu",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: redactedWebGpuAdapter,
    browserGpu: {
      ...chromiumNvidiaGpu,
      devices: [{
        vendor_id: 0x1ae0,
        device_id: 0xc0de,
        vendor_string: "Google",
        device_string: "Google SwiftShader",
      }],
      aux_attributes: { gl_renderer: "ANGLE (Google, Vulkan 1.3 SwiftShader Device)" },
    },
  }).failure_codes.includes("SOFTWARE_ADAPTER"),
  "CDP evidence must fail closed when Chromium is software-rendered",
);
assert(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgpu",
    actualBackend: "WebGpu",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: redactedWebGpuAdapter,
    browserGpu: {
      source: "chromium-cdp-system-info",
      devices: [],
      aux_attributes: {},
      feature_status: {},
    },
  }).failure_codes.includes("ADAPTER_HARDWARE_UNPROVEN"),
  "empty CDP evidence must not waive adapter identity",
);
assert.strictEqual(
  evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: "webgl2",
    actualBackend: "web_gl2",
    gpuDevice: true,
    surfaceAttached: true,
    adapter: { name: "ANGLE (Broadcom, V3D)", device_type: "IntegratedGpu" },
  }).status,
  "passed",
);

const previousWebGpuBrowser = process.env.SCENA_WEBGPU_BROWSER;
process.env.SCENA_WEBGPU_BROWSER = "firefox";
assert.strictEqual(browserEngineForBackend("webgpu"), "firefox");
delete process.env.SCENA_WEBGPU_BROWSER;
assert.strictEqual(browserEngineForBackend("webgpu"), "chromium");
if (previousWebGpuBrowser === undefined) {
  delete process.env.SCENA_WEBGPU_BROWSER;
} else {
  process.env.SCENA_WEBGPU_BROWSER = previousWebGpuBrowser;
}

const windowsChromiumArgs = chromiumArgsForPlatform("win32");
assert(windowsChromiumArgs.includes("--enable-features=WebGPU"));
assert(!windowsChromiumArgs.some((argument) => argument.includes("Vulkan")));
const linuxChromiumArgs = chromiumArgsForPlatform("linux");
assert(linuxChromiumArgs.includes("--enable-features=Vulkan,WebGPU"));

console.log("required GPU parity evaluator: pass");

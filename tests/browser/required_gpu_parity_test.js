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

function result(adapter, drawCalls = 1, nonblack = 64) {
  return {
    backend: "WebGpu",
    status: "passed",
    gpu_device: true,
    draw_calls: drawCalls,
    gpu_submissions: drawCalls,
    adapter,
    renderer_readback: {
      source: "renderer-owned-gpu-copy",
      pixel_statistics: { nonblack },
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
assert.strictEqual(
  evaluateRequiredGpuParity({
    required: true,
    requestedBackend: "webgpu",
    result: result({ name: "NVIDIA RTX", device_type: "DiscreteGpu" }),
  }).status,
  "passed",
);

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

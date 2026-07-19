const SOFTWARE_ADAPTER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "lavapipe",
  "software rasterizer",
  "microsoft basic render",
];

function softwareAdapter(adapter) {
  if (!adapter || typeof adapter !== "object") {
    return false;
  }
  const identity = [adapter.name, adapter.device_type, adapter.driver, adapter.driver_info]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  return (
    String(adapter.device_type || "").toLowerCase() === "cpu" ||
    SOFTWARE_ADAPTER_MARKERS.some((marker) => identity.includes(marker))
  );
}

function hardwareAdapter(adapter) {
  const deviceType = String(adapter && adapter.device_type ? adapter.device_type : "").toLowerCase();
  return ["discretegpu", "integratedgpu", "virtualgpu"].includes(deviceType) && !softwareAdapter(adapter);
}

function browserGpuRendererIdentity(browserGpu) {
  if (!browserGpu || typeof browserGpu !== "object") return "";
  const aux = browserGpu.aux_attributes && typeof browserGpu.aux_attributes === "object"
    ? browserGpu.aux_attributes
    : {};
  return [aux.gl_vendor, aux.gl_renderer].filter(Boolean).join(" ").toLowerCase();
}

function softwareBrowserGpu(browserGpu) {
  const rendererIdentity = browserGpuRendererIdentity(browserGpu);
  if (rendererIdentity) {
    return SOFTWARE_ADAPTER_MARKERS.some((marker) => rendererIdentity.includes(marker));
  }
  const devices = Array.isArray(browserGpu && browserGpu.devices) ? browserGpu.devices : [];
  return devices.length > 0 && devices.every((device) => {
    const identity = [device && device.vendor_string, device && device.device_string]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return SOFTWARE_ADAPTER_MARKERS.some((marker) => identity.includes(marker));
  });
}

function hardwareBrowserGpu(browserGpu) {
  if (
    !browserGpu
    || browserGpu.source !== "chromium-cdp-system-info"
    || softwareBrowserGpu(browserGpu)
  ) {
    return false;
  }
  const renderer = browserGpuRendererIdentity(browserGpu);
  if (!renderer) return false;
  const devices = Array.isArray(browserGpu.devices) ? browserGpu.devices : [];
  return devices.some((device) => {
    if (!device || typeof device !== "object") return false;
    const deviceIdentity = String(device.device_string || "").trim().toLowerCase();
    const vendorId = Number(device.vendor_id || 0);
    const deviceId = Number(device.device_id || 0);
    return deviceIdentity.length > 0
      && vendorId > 0
      && deviceId > 0
      && renderer.includes(deviceIdentity);
  });
}

function normalizedBackend(value) {
  return String(value || "").toLowerCase().replace(/[^a-z0-9]/g, "");
}

function evaluateRequiredGpuParity({ required, requestedBackend, result, error }) {
  if (!required) {
    return { status: "diagnostic", failure_codes: [] };
  }
  const message = String(error && error.message ? error.message : error || "");
  if (message.includes("NoAdapter")) {
    return { status: "failed", failure_codes: ["NO_ADAPTER"] };
  }
  const failures = [];
  if (!result || typeof result !== "object") {
    failures.push("MISSING_RESULT");
    return { status: "failed", failure_codes: failures };
  }
  if (normalizedBackend(result.backend) !== normalizedBackend(requestedBackend)) {
    failures.push("BACKEND_MISMATCH");
  }
  if (result.status !== "passed") failures.push("PROBE_NOT_PASSED");
  if (result.gpu_device !== true) failures.push("GPU_DEVICE_MISSING");
  const nonblack =
    result.renderer_readback &&
    result.renderer_readback.pixel_statistics &&
    result.renderer_readback.pixel_statistics.nonblack;
  if (
    !(result.draw_calls > 0) ||
    !(result.gpu_submissions > 0) ||
    !result.renderer_readback ||
    result.renderer_readback.source !== "renderer-owned-gpu-copy" ||
    !(nonblack > 0)
  ) {
    failures.push("ZERO_RENDERER_OUTPUT");
  }
  if (!result.adapter || typeof result.adapter !== "object") {
    failures.push("ADAPTER_IDENTITY_MISSING");
  } else if (softwareAdapter(result.adapter)) {
    failures.push("SOFTWARE_ADAPTER");
  } else if (!hardwareAdapter(result.adapter)) {
    failures.push("ADAPTER_HARDWARE_UNPROVEN");
  }
  return {
    status: failures.length === 0 ? "passed" : "failed",
    failure_codes: failures,
  };
}

function evaluateRequiredHardwareAdapter({
  required,
  requestedBackend,
  actualBackend,
  gpuDevice,
  surfaceAttached,
  adapter,
  browserGpu,
}) {
  if (!required) {
    return { status: "diagnostic", failure_codes: [] };
  }
  const failures = [];
  if (normalizedBackend(actualBackend) !== normalizedBackend(requestedBackend)) {
    failures.push("BACKEND_MISMATCH");
  }
  if (gpuDevice !== true) failures.push("GPU_DEVICE_MISSING");
  if (surfaceAttached !== true) failures.push("SURFACE_NOT_ATTACHED");
  if (softwareAdapter(adapter) || softwareBrowserGpu(browserGpu)) {
    failures.push("SOFTWARE_ADAPTER");
  } else if (!adapter || typeof adapter !== "object") {
    failures.push("ADAPTER_IDENTITY_MISSING");
  } else if (!hardwareAdapter(adapter) && !hardwareBrowserGpu(browserGpu)) {
    failures.push("ADAPTER_HARDWARE_UNPROVEN");
  }
  return {
    status: failures.length === 0 ? "passed" : "failed",
    failure_codes: failures,
  };
}

module.exports = {
  evaluateRequiredGpuParity,
  evaluateRequiredHardwareAdapter,
  hardwareBrowserGpu,
  hardwareAdapter,
  softwareBrowserGpu,
  softwareAdapter,
};

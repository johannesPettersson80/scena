function browserEngineForBackend(backend) {
  const variable = backend === "webgpu" ? "SCENA_WEBGPU_BROWSER" : "SCENA_WEBGL2_BROWSER";
  const engine = String(process.env[variable] || "chromium").trim().toLowerCase();
  if (!['chromium', 'firefox'].includes(engine)) {
    throw new Error(`${variable} must be chromium or firefox, got ${engine}`);
  }
  return engine;
}

function chromiumArgsForPlatform(platform = process.platform) {
  const args = [
    "--ignore-gpu-blocklist",
    "--enable-unsafe-webgpu",
  ];
  if (platform === "linux") {
    args.unshift("--no-sandbox");
    args.push("--enable-features=Vulkan,WebGPU");
  } else {
    args.push("--enable-features=WebGPU");
  }
  return args;
}

async function launchHardwareBrowser(backend) {
  const engine = browserEngineForBackend(backend);
  const playwright = require("playwright");
  if (engine === "firefox") {
    const browser = await playwright.firefox.launch({
      headless: true,
      firefoxUserPrefs: {
        "dom.webgpu.enabled": true,
        "gfx.webgpu.force-enabled": true,
      },
    });
    return { browser, engine };
  }
  const browser = await playwright.chromium.launch({
    executablePath: process.env.SCENA_BROWSER_EXECUTABLE || process.env.CHROMIUM || undefined,
    headless: true,
    args: chromiumArgsForPlatform(),
  });
  return { browser, engine };
}

function sanitizeChromiumGpuInfo(info) {
  const gpu = info && info.gpu && typeof info.gpu === "object" ? info.gpu : {};
  const aux = gpu.auxAttributes && typeof gpu.auxAttributes === "object"
    ? gpu.auxAttributes
    : {};
  const featureStatus = gpu.featureStatus && typeof gpu.featureStatus === "object"
    ? gpu.featureStatus
    : {};
  return {
    source: "chromium-cdp-system-info",
    devices: (Array.isArray(gpu.devices) ? gpu.devices : []).map((device) => ({
      vendor_id: Number(device.vendorId || 0),
      device_id: Number(device.deviceId || 0),
      vendor_string: String(device.vendorString || ""),
      device_string: String(device.deviceString || ""),
      driver_vendor: String(device.driverVendor || ""),
      driver_version: String(device.driverVersion || ""),
    })),
    aux_attributes: {
      gl_vendor: String(aux.glVendor || ""),
      gl_renderer: String(aux.glRenderer || ""),
    },
    feature_status: Object.fromEntries(
      Object.entries(featureStatus).map(([key, value]) => [key, String(value)]),
    ),
  };
}

async function collectBrowserGpuEvidence(browser, engine) {
  if (engine !== "chromium") return null;
  const session = await browser.newBrowserCDPSession();
  try {
    return sanitizeChromiumGpuInfo(await session.send("SystemInfo.getInfo"));
  } finally {
    await session.detach();
  }
}

module.exports = {
  browserEngineForBackend,
  collectBrowserGpuEvidence,
  chromiumArgsForPlatform,
  launchHardwareBrowser,
  sanitizeChromiumGpuInfo,
};

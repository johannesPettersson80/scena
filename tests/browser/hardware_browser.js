function browserEngineForBackend(backend) {
  const variable = backend === "webgpu" ? "SCENA_WEBGPU_BROWSER" : "SCENA_WEBGL2_BROWSER";
  const engine = String(process.env[variable] || "chromium").trim().toLowerCase();
  if (!['chromium', 'firefox'].includes(engine)) {
    throw new Error(`${variable} must be chromium or firefox, got ${engine}`);
  }
  return engine;
}

function chromiumArgsForPlatform(
  platform = process.platform,
  backend = "webgpu",
  executablePath = process.env.SCENA_BROWSER_EXECUTABLE || process.env.CHROMIUM || null,
) {
  const args = ["--ignore-gpu-blocklist"];
  if (platform === "linux") args.unshift("--no-sandbox");
  if (backend === "webgpu") {
    args.push("--enable-unsafe-webgpu");
    args.push(platform === "linux" ? "--enable-features=Vulkan,WebGPU" : "--enable-features=WebGPU");
  } else if (backend === "webgl2") {
    if (!executablePath) {
      args.push("--use-angle=swiftshader", "--enable-unsafe-swiftshader");
    }
  } else {
    throw new Error(`unsupported Chromium backend '${backend}'`);
  }
  return args;
}

async function launchHardwareBrowser(backend) {
  const engine = browserEngineForBackend(backend);
  const headless = process.env.SCENA_BROWSER_HEADLESS !== "0";
  const playwright = require("playwright");
  if (engine === "firefox") {
    const browser = await playwright.firefox.launch({
      headless,
      firefoxUserPrefs: {
        "dom.webgpu.enabled": true,
        "gfx.webgpu.force-enabled": true,
      },
    });
    return { browser, engine };
  }
  const executablePath = process.env.SCENA_BROWSER_EXECUTABLE || process.env.CHROMIUM || undefined;
  const browser = await playwright.chromium.launch({
    executablePath,
    headless,
    args: chromiumArgsForPlatform(process.platform, backend, executablePath),
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

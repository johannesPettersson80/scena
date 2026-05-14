import init, {
  m6RenderWebgl2Probe,
  m6RenderWebgpuProbe,
  m6RenderSurfaceLifecycleProbe,
  m6RenderBenchmarkProbe,
  m6RenderStateLifecycleProbe,
  m6RenderWorkflowProbe,
} from "/pkg/scena.js";

let initialized = false;

async function ensureInit() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

function createCanvas(backend, workflow = "triangle") {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  canvas.dataset.backend = backend;
  canvas.dataset.workflow = workflow;
  document.body.appendChild(canvas);
  return canvas;
}

function summarizePixels(width, height, pixels) {
  let nonblack = 0;
  let max = [0, 0, 0, 0];
  for (let index = 0; index < pixels.length; index += 4) {
    if (pixels[index] > 0 || pixels[index + 1] > 0 || pixels[index + 2] > 0) {
      nonblack += 1;
    }
    max = [
      Math.max(max[0], pixels[index]),
      Math.max(max[1], pixels[index + 1]),
      Math.max(max[2], pixels[index + 2]),
      Math.max(max[3], pixels[index + 3]),
    ];
  }
  const sampleAt = (x, y) => {
    const clampedX = Math.max(0, Math.min(width - 1, Math.floor(x)));
    const clampedY = Math.max(0, Math.min(height - 1, Math.floor(y)));
    const offset = (clampedY * width + clampedX) * 4;
    return Array.from(pixels.slice(offset, offset + 4));
  };
  return {
    center: sampleAt(width / 2, height / 2),
    flat: sampleAt(width * 0.38, height / 2),
    inverted: sampleAt(width * 0.62, height / 2),
    nonblack,
    max,
  };
}

function readWebGl2Pixels(canvas) {
  const gl = canvas.getContext("webgl2", { antialias: false });
  if (!gl) {
    return null;
  }
  const pixels = new Uint8Array(canvas.width * canvas.height * 4);
  gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  return summarizePixels(canvas.width, canvas.height, pixels);
}

function readCanvasPixels(canvas) {
  const copy = document.createElement("canvas");
  copy.width = canvas.width;
  copy.height = canvas.height;
  const context = copy.getContext("2d", { willReadFrequently: true });
  if (!context) {
    return null;
  }
  context.drawImage(canvas, 0, 0);
  return summarizePixels(
    copy.width,
    copy.height,
    context.getImageData(0, 0, copy.width, copy.height).data,
  );
}

function readRenderedPixels(backend, canvas) {
  if (backend === "webgl2") {
    return readWebGl2Pixels(canvas) || readCanvasPixels(canvas);
  }
  return readCanvasPixels(canvas);
}

async function readRenderedPixelsWithRetry(backend, canvas, workflow) {
  const maxAttempts = backend === "webgpu" ? 8 : 2;
  let lastPixels = null;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
    if (backend === "webgpu") {
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
    lastPixels = readRenderedPixels(backend, canvas);
    const benchmarkOk = workflow === "benchmark-idle";
    if (benchmarkOk || (lastPixels && lastPixels.nonblack > 0)) {
      return { pixels: lastPixels, attempts: attempt };
    }
  }
  return { pixels: lastPixels, attempts: maxAttempts };
}

async function runProbe(backend, workflow, render) {
  await ensureInit();
  const canvas = createCanvas(backend, workflow);
  const raw = await render(canvas);
  const result = JSON.parse(raw);
  const readback = await readRenderedPixelsWithRetry(backend, canvas, workflow);
  const rendererReadback =
    result.renderer_readback && result.renderer_readback.pixel_statistics;
  const useRendererReadback =
    backend === "webgpu" && rendererReadback && rendererReadback.nonblack > 0;
  const pixelStatistics = useRendererReadback ? rendererReadback : readback.pixels;
  result.workflow = workflow;
  result.pixels = pixelStatistics;
  result.pixel_source = useRendererReadback ? "renderer-owned-gpu-copy" : "canvas-readback";
  result.pixel_readback_attempts = readback.attempts;
  result.canvas_data_url = canvas.toDataURL("image/png");
  result.screenshot_metadata = {
    backend,
    workflow,
    adapter: result.gpu_device,
    width: canvas.width,
    height: canvas.height,
    device_pixel_ratio: window.devicePixelRatio || 1,
    canvas_mime: "image/png",
    pixel_source: result.pixel_source,
    pixel_readback_attempts: readback.attempts,
    pixel_statistics: pixelStatistics,
    canvas_pixel_statistics: readback.pixels,
    renderer_readback: result.renderer_readback || null,
  };
  const benchmarkOk =
    workflow === "benchmark-idle" &&
    result.benchmark_metrics &&
    result.benchmark_metrics.idle_render_skipped === true &&
    result.benchmark_metrics.high_instance_primitives > 0;
  result.status =
    result.draw_calls > 0 &&
    result.gpu_submissions > 0 &&
    (benchmarkOk || (result.pixels && result.pixels.nonblack > 0))
      ? "passed"
      : "failed";
  return result;
}

window.scenaM6RustWasmRendererProbe = async function scenaM6RustWasmRendererProbe(backend) {
  return runProbe(backend, "triangle", (canvas) =>
    backend === "webgpu" ? m6RenderWebgpuProbe(canvas) : m6RenderWebgl2Probe(canvas),
  );
};

window.scenaM6RustWasmWorkflowProbe = async function scenaM6RustWasmWorkflowProbe(
  backend,
  workflow,
) {
  return runProbe(backend, workflow, (canvas) =>
    m6RenderWorkflowProbe(canvas, backend, workflow),
  );
};

window.scenaM6RustWasmLifecycleProbe = async function scenaM6RustWasmLifecycleProbe(backend) {
  return runProbe(backend, "surface-context-lifecycle", (canvas) =>
    m6RenderSurfaceLifecycleProbe(canvas, backend),
  );
};

window.scenaM6RustWasmBenchmarkProbe = async function scenaM6RustWasmBenchmarkProbe(backend) {
  return runProbe(backend, "benchmark-idle", (canvas) =>
    m6RenderBenchmarkProbe(canvas, backend),
  );
};

window.scenaM6RustWasmStateLifecycleProbe =
  async function scenaM6RustWasmStateLifecycleProbe(backend) {
    return runProbe(backend, "state-lifetime-idle", (canvas) =>
      m6RenderStateLifecycleProbe(canvas, backend),
    );
  };

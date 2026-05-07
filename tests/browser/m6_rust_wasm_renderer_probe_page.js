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

function readWebGl2Pixels(canvas) {
  const gl = canvas.getContext("webgl2", { antialias: false });
  if (!gl) {
    return null;
  }
  const pixels = new Uint8Array(canvas.width * canvas.height * 4);
  gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
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
  const centerOffset = ((canvas.height / 2) * canvas.width + canvas.width / 2) * 4;
  return {
    center: Array.from(pixels.slice(centerOffset, centerOffset + 4)),
    nonblack,
    max,
  };
}

async function runProbe(backend, workflow, render) {
  await ensureInit();
  const canvas = createCanvas(backend, workflow);
  const raw = await render(canvas);
  await new Promise((resolve) => requestAnimationFrame(() => resolve()));
  const result = JSON.parse(raw);
  result.workflow = workflow;
  result.pixels = backend === "webgl2" ? readWebGl2Pixels(canvas) : null;
  result.canvas_data_url = canvas.toDataURL("image/png");
  const benchmarkOk =
    workflow === "benchmark-idle" &&
    result.benchmark_metrics &&
    result.benchmark_metrics.idle_render_skipped === true &&
    result.benchmark_metrics.high_instance_primitives > 0;
  result.status =
    result.draw_calls > 0 &&
    result.gpu_submissions > 0 &&
    (backend === "webgpu" || benchmarkOk || (result.pixels && result.pixels.nonblack > 0))
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

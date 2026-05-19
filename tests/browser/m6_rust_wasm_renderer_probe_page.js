import init, {
  defineScenaViewer,
  m6RenderWebgl2Probe,
  m6RenderWebgpuProbe,
  m6RenderSurfaceLifecycleProbe,
  m6RenderBenchmarkProbe,
  m6CameraControlKitProbe,
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

async function nextFrame() {
  await new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function loadInspectorSnapshot() {
  const response = await fetch("/fixtures/viewer/inspector_snapshot.json");
  if (!response.ok) {
    throw new Error(`inspector fixture load failed: ${response.status}`);
  }
  const snapshot = await response.json();
  if (snapshot.schema !== "scena.scena_viewer_inspector_snapshot.v1") {
    throw new Error(`unexpected inspector fixture schema: ${snapshot.schema}`);
  }
  return snapshot;
}

function once(target, eventName) {
  return new Promise((resolve) => {
    target.addEventListener(eventName, (event) => resolve(event.detail || {}), { once: true });
  });
}

async function dispatchDrop(viewer) {
  const dataTransfer = new DataTransfer();
  dataTransfer.items.add(new File(["{}"], "accepted-machine.glb", { type: "model/gltf-binary" }));
  dataTransfer.items.add(new File(["not a model"], "notes.txt", { type: "text/plain" }));
  const accepted = once(viewer, "scena-viewer-file-drop");
  const rejected = once(viewer, "scena-viewer-drop-error");
  viewer.dispatchEvent(new DragEvent("drop", {
    bubbles: true,
    cancelable: true,
    dataTransfer,
  }));
  return {
    accepted: await accepted,
    rejected: await rejected,
  };
}

window.scenaViewerElementProbe = async function scenaViewerElementProbe() {
  await ensureInit();
  const defined = defineScenaViewer();
  await customElements.whenDefined("scena-viewer");

  const viewer = document.createElement("scena-viewer");
  viewer.dataset.proof = "custom-element";
  viewer.setAttribute("src", "/fixtures/gltf/non_ndc_camera_scene.gltf");
  viewer.setAttribute("camera-controls", "");
  viewer.setAttribute("auto-rotate", "");
  viewer.style.cssText = "display:block;width:360px;height:240px;background:#111827;color:#f8fafc";

  const annotation = document.createElement("span");
  annotation.slot = "annotation";
  annotation.id = "bearing-label";
  annotation.dataset.position = "0 1 0";
  annotation.dataset.normal = "0 0 1";
  annotation.dataset.surface = "bearing";
  annotation.textContent = "Bearing";
  annotation.style.cssText = "padding:4px 7px;background:#f8fafc;color:#0f172a;font:12px system-ui,sans-serif;border-radius:4px";
  viewer.append(annotation);

  const ready = once(viewer, "scena-viewer-ready");
  document.body.append(viewer);
  const readyDetail = await ready;
  await nextFrame();

  const root = viewer.shadowRoot;
  const canvas = root.querySelector("canvas");
  const progress = root.querySelector("[part=progress]");
  const progressBar = root.querySelector("[part=progress-bar]");
  const progressRendered = once(viewer, "scena-viewer-progress-rendered");
  viewer.dispatchEvent(new CustomEvent("scena-viewer-progress", {
    bubbles: true,
    detail: { phase: "fetching", value: 0.42, ariaText: "Fetching model" },
  }));
  const progressDetail = await progressRendered;

  const variantsReady = once(viewer, "scena-viewer-variants-ready");
  viewer.setMaterialVariants([
    { name: "raw", label: "Raw metal" },
    { name: "painted", label: "Painted" },
  ], "raw");
  const variantReadyDetail = await variantsReady;
  const variantChange = once(viewer, "scena-viewer-variant-change");
  const variantPicker = root.querySelector("[part=variant-picker]");
  variantPicker.value = "painted";
  variantPicker.dispatchEvent(new Event("change", { bubbles: true }));
  const variantChangeDetail = await variantChange;

  const annotationRequest = once(viewer, "scena-viewer-annotations-request");
  viewer.requestAnnotationProjections();
  const annotationRequestDetail = await annotationRequest;
  const annotationsRendered = once(viewer, "scena-viewer-annotations-rendered");
  viewer.setAnnotationProjections([{ id: "bearing-label", x: 144, y: 72, visible: true }]);
  const annotationsRenderedDetail = await annotationsRendered;

  const inspectorRendered = once(viewer, "scena-viewer-inspector-rendered");
  const inspectorSnapshot = await loadInspectorSnapshot();
  viewer.setInspectorSnapshot(inspectorSnapshot);
  const inspectorDetail = await inspectorRendered;

  const keyboard = once(viewer, "scena-viewer-key-control");
  viewer.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowLeft" }));
  const keyboardDetail = await keyboard;
  const dropDetail = await dispatchDrop(viewer);
  await nextFrame();

  const checks = {
    defined,
    host_role: viewer.getAttribute("role"),
    host_label: viewer.getAttribute("aria-label"),
    host_tabindex: viewer.getAttribute("tabindex"),
    host_roledescription: viewer.getAttribute("aria-roledescription"),
    canvas_label: canvas.getAttribute("aria-label"),
    canvas_touch_action: getComputedStyle(canvas).touchAction,
    ready_src: readyDetail.src,
    progress_phase: progressDetail.phase,
    progress_value_now: progress.getAttribute("aria-valuenow"),
    progress_bar_transform: progressBar.style.transform,
    variant_names: variantReadyDetail.names,
    variant_change: variantChangeDetail.name,
    annotation_count: annotationRequestDetail.anchors.length,
    annotation_visible: annotationsRenderedDetail.visible,
    annotation_transform: getComputedStyle(annotation).transform,
    inspector_overlay: inspectorDetail.overlay,
    inspector_warnings: inspectorDetail.warnings,
    inspector_fixture_schema: inspectorSnapshot.schema,
    inspector_fixture_source: inspectorSnapshot.source,
    keyboard_action: keyboardDetail.action,
    drop_accepted_names: dropDetail.accepted.names,
    drop_rejected_names: dropDetail.rejected.names,
    drop_error_message: dropDetail.rejected.message,
  };

  const passed =
    checks.host_role === "img" &&
    checks.host_label === "3D model viewer" &&
    checks.host_tabindex === "0" &&
    checks.host_roledescription === "interactive 3D model" &&
    checks.canvas_label === "scena 3D viewer canvas" &&
    checks.canvas_touch_action === "none" &&
    checks.ready_src.endsWith("non_ndc_camera_scene.gltf") &&
    checks.progress_phase === "fetching" &&
    checks.progress_value_now === "42" &&
    checks.progress_bar_transform === "scaleX(0.42)" &&
    Array.isArray(checks.variant_names) &&
    checks.variant_names.includes("raw") &&
    checks.variant_names.includes("painted") &&
    checks.variant_change === "painted" &&
    checks.annotation_count === 1 &&
    checks.annotation_visible === 1 &&
    checks.annotation_transform !== "none" &&
    checks.inspector_overlay === "Diagnostics" &&
    checks.inspector_warnings === 1 &&
    checks.keyboard_action === "orbit-left" &&
    checks.drop_accepted_names.includes("accepted-machine.glb") &&
    checks.drop_rejected_names.includes("notes.txt") &&
    checks.drop_error_message.includes("notes.txt");

  return {
    schema: "scena.scena_viewer_element_browser_proof.v1",
    status: passed ? "passed" : "failed",
    proof_class: "browser-demo",
    visual_proof: "browser-demo",
    screenshot_selector: "scena-viewer[data-proof=\"custom-element\"]",
    checks,
  };
};

window.scenaCameraControlKitProbe = async function scenaCameraControlKitProbe() {
  await ensureInit();
  const result = JSON.parse(m6CameraControlKitProbe());
  if (result.schema !== "scena.m6.camera_control_kit_browser_proof.v1") {
    throw new Error(`unexpected camera-control-kit schema: ${result.schema}`);
  }
  const panel = document.createElement("section");
  panel.dataset.proof = "camera-control-kit";
  panel.style.cssText = "display:grid;gap:6px;width:360px;margin-top:12px;padding:12px;background:#0f172a;color:#e2e8f0;font:12px system-ui,sans-serif";
  panel.innerHTML = `
    <strong>Camera controls</strong>
    <span>Orbit: ${result.orbit.actions.join(" -> ")}</span>
    <span>Follow: ${result.follow.camera_translation.map((value) => value.toFixed(2)).join(", ")}</span>
    <span>Fly: ${result.fly.camera_translation.map((value) => value.toFixed(2)).join(", ")}</span>
  `;
  document.body.append(panel);
  return result;
};

function createCanvas(backend, workflow = "triangle") {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  canvas.dataset.backend = backend;
  canvas.dataset.workflow = workflow;
  document.body.appendChild(canvas);
  if (backend === "webgl2") {
    canvas.getContext("webgl2", { antialias: false, preserveDrawingBuffer: true });
  }
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
    left: sampleAt(width * 0.25, height / 2),
    right: sampleAt(width * 0.75, height / 2),
    flat: sampleAt(width * 0.38, height / 2),
    inverted: sampleAt(width * 0.62, height / 2),
    nonblack,
    max,
  };
}

function readWebGl2Pixels(canvas) {
  const gl = canvas.getContext("webgl2", { antialias: false, preserveDrawingBuffer: true });
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

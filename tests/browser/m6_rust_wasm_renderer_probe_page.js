import init, {
  defineScenaViewer,
  m6RenderWebgl2Probe,
  m6RenderWebgpuProbe,
  m6RenderSurfaceLifecycleProbe,
  m6RenderBenchmarkProbe,
  m6CameraControlKitProbe,
  m6RenderDroppedFileProbe,
  m6RenderDisplayP3Probe,
  m6RenderMaterialVariantProbe,
  m6RenderStateLifecycleProbe,
  m6RenderWorkflowProbe,
  m6AssetDoctorBrowserProbe,
} from "/pkg/scena.js";

let initialized = false;
let modelViewerInitialized = false;

const SCENA_VIEWER_PARITY_ASSETS = [
  {
    label: "Framed glTF camera",
    source: "/fixtures/gltf/non_ndc_camera_scene.gltf",
    workflow: "model-viewer",
  },
  {
    label: "Animated morph asset",
    source: "/fixtures/gltf/khronos/MorphCube/AnimatedMorphCube.gltf",
    workflow: "animation",
  },
  {
    label: "Textured material asset",
    source: "/fixtures/gltf/khronos/WaterBottle/WaterBottle.gltf",
    workflow: "source-gltf-materials",
  },
];

async function ensureInit() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

async function ensureModelViewer() {
  if (!modelViewerInitialized) {
    await import("/model-viewer/model-viewer.min.js");
    modelViewerInitialized = true;
  }
  await customElements.whenDefined("model-viewer");
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
  const response = await fetch("/fixtures/gltf/load_unit.glb");
  if (!response.ok) {
    throw new Error(`drop fixture load failed: ${response.status}`);
  }
  const droppedBytes = await response.arrayBuffer();
  const dataTransfer = new DataTransfer();
  dataTransfer.items.add(new File([droppedBytes], "accepted-machine.glb", { type: "model/gltf-binary" }));
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

async function renderDroppedFileIntoViewer(viewer, backend, dropDetail) {
  const droppedFile = dropDetail.accepted.files[0];
  const bytes = new Uint8Array(await droppedFile.arrayBuffer());
  viewer.canvas.width = 96;
  viewer.canvas.height = 64;
  const raw = await m6RenderDroppedFileProbe(viewer.canvas, backend, bytes, droppedFile.name);
  const rendered = JSON.parse(raw);
  const readback = await readRenderedPixelsWithRetry(backend, viewer.canvas, "scena-viewer-drop-render");
  const rendererPixels =
    rendered.renderer_readback && rendered.renderer_readback.pixel_statistics;
  const pixels = rendererPixels && rendererPixels.nonblack > 0 ? rendererPixels : readback.pixels;
  const passed =
    rendered.workflow === "scena-viewer-drop-render" &&
    rendered.draw_calls > 0 &&
    rendered.gpu_submissions > 0 &&
    pixels &&
    pixels.nonblack > 0;
  return {
    ...rendered,
    status: passed ? "passed" : "failed",
    pixels,
    pixel_source: rendererPixels && rendererPixels.nonblack > 0 ? "renderer-owned-gpu-copy" : "canvas-readback",
    pixel_readback_attempts: readback.attempts,
  };
}

async function renderSelectedVariantIntoViewer(viewer, backend, variantName) {
  viewer.canvas.width = 96;
  viewer.canvas.height = 64;
  const raw = await m6RenderMaterialVariantProbe(viewer.canvas, backend, variantName);
  const rendered = JSON.parse(raw);
  const readback = await readRenderedPixelsWithRetry(
    backend,
    viewer.canvas,
    "scena-viewer-material-variant-render",
  );
  const rendererPixels =
    rendered.renderer_readback && rendered.renderer_readback.pixel_statistics;
  const pixels = rendererPixels && rendererPixels.nonblack > 0 ? rendererPixels : readback.pixels;
  const center = (pixels && pixels.center) || [];
  const greenDominant =
    center.length >= 3 &&
    center[1] > center[0] + 20 &&
    center[1] > center[2] + 20;
  const passed =
    rendered.workflow === "scena-viewer-material-variant-render" &&
    rendered.metadata.proof_class === "scena-viewer-material-variant-render" &&
    rendered.metadata.selected_variant === variantName &&
    rendered.metadata.active_variant === variantName &&
    rendered.draw_calls > 0 &&
    pixels &&
    pixels.nonblack > 0 &&
    greenDominant;
  return {
    ...rendered,
    status: passed ? "passed" : "failed",
    pixels,
    green_dominant: greenDominant,
    pixel_source: rendererPixels && rendererPixels.nonblack > 0 ? "renderer-owned-gpu-copy" : "canvas-readback",
    pixel_readback_attempts: readback.attempts,
  };
}

function waitForModelViewerLoaded(viewer, source) {
  if (viewer.loaded) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error(`model-viewer timed out loading ${source}`)),
      15000,
    );
    viewer.addEventListener("load", () => {
      clearTimeout(timeout);
      resolve();
    }, { once: true });
    viewer.addEventListener("error", (event) => {
      clearTimeout(timeout);
      reject(new Error(`model-viewer failed loading ${source}: ${event.detail || event.type}`));
    }, { once: true });
  });
}

async function renderWorkflowIntoCanvas(canvas, backend, workflow) {
  canvas.width = 160;
  canvas.height = 120;
  const raw = await m6RenderWorkflowProbe(canvas, backend, workflow);
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
  result.status =
    result.draw_calls > 0 &&
    result.gpu_submissions > 0 &&
    result.pixels &&
    result.pixels.nonblack > 0
      ? "passed"
      : "failed";
  return result;
}

function modelViewerCanvasReady(viewer) {
  const canvas = viewer.shadowRoot && viewer.shadowRoot.querySelector("canvas");
  return Boolean(canvas && canvas.width > 0 && canvas.height > 0);
}

window.scenaViewerModelViewerParityProbe =
  async function scenaViewerModelViewerParityProbe(backend = "webgl2") {
    await ensureInit();
    await ensureModelViewer();
    defineScenaViewer();
    await customElements.whenDefined("scena-viewer");

    document.body.style.margin = "0";
    const section = document.createElement("section");
    section.dataset.proof = "scena-viewer-model-viewer-parity";
    section.style.cssText = "display:grid;gap:12px;width:900px;padding:16px;background:#0b1020;color:#e5e7eb;font:12px system-ui,sans-serif";
    const title = document.createElement("strong");
    title.textContent = "<scena-viewer> / <model-viewer> asset parity";
    section.append(title);
    document.body.append(section);

    const assets = [];
    for (const asset of SCENA_VIEWER_PARITY_ASSETS) {
      const row = document.createElement("div");
      row.dataset.asset = asset.workflow;
      row.style.cssText = "display:grid;grid-template-columns:1fr 1fr;gap:10px;align-items:stretch";

      const referencePane = document.createElement("div");
      referencePane.style.cssText = "display:grid;gap:4px;padding:8px;background:#111827";
      const referenceLabel = document.createElement("span");
      referenceLabel.textContent = `model-viewer: ${asset.label}`;
      const modelViewer = document.createElement("model-viewer");
      modelViewer.setAttribute("src", asset.source);
      modelViewer.setAttribute("camera-controls", "");
      modelViewer.setAttribute("reveal", "auto");
      modelViewer.style.cssText = "display:block;width:100%;height:170px;background:#111827";
      referencePane.append(referenceLabel, modelViewer);

      const scenaPane = document.createElement("div");
      scenaPane.style.cssText = "display:grid;gap:4px;padding:8px;background:#111827";
      const scenaLabel = document.createElement("span");
      scenaLabel.textContent = `scena-viewer: ${asset.label}`;
      const scenaViewer = document.createElement("scena-viewer");
      scenaViewer.setAttribute("src", asset.source);
      scenaViewer.setAttribute("camera-controls", "");
      scenaViewer.setAttribute("tone-mapping", "neutral");
      scenaViewer.style.cssText = "display:block;width:100%;height:170px;background:#111827;color:#f8fafc";
      scenaPane.append(scenaLabel, scenaViewer);

      const ready = once(scenaViewer, "scena-viewer-ready");
      row.append(referencePane, scenaPane);
      section.append(row);

      await ready;
      await waitForModelViewerLoaded(modelViewer, asset.source);
      await nextFrame();
      await nextFrame();

      const canvas = scenaViewer.shadowRoot.querySelector("canvas");
      const render = await renderWorkflowIntoCanvas(canvas, backend, asset.workflow);
      const referenceRect = modelViewer.getBoundingClientRect();
      const scenaRect = scenaViewer.getBoundingClientRect();
      assets.push({
        label: asset.label,
        source: asset.source,
        workflow: asset.workflow,
        side_by_side: true,
        model_viewer_tag: modelViewer.tagName,
        scena_viewer_tag: scenaViewer.tagName,
        model_viewer_loaded: modelViewer.loaded === true,
        model_viewer_canvas_ready: modelViewerCanvasReady(modelViewer),
        model_viewer_width: Math.round(referenceRect.width),
        model_viewer_height: Math.round(referenceRect.height),
        scena_width: Math.round(scenaRect.width),
        scena_height: Math.round(scenaRect.height),
        scena_backend: backend,
        scena_render_status: render.status,
        scena_pixels_nonblack: render.pixels && render.pixels.nonblack,
        scena_pixel_source: render.pixel_source,
        scena_workflow: render.workflow,
        scene_api: render.scene_api,
        assets_api: render.assets_api,
      });
    }

    const passed =
      assets.length === SCENA_VIEWER_PARITY_ASSETS.length &&
      assets.every((asset) =>
        asset.side_by_side === true &&
        asset.model_viewer_tag === "MODEL-VIEWER" &&
        asset.scena_viewer_tag === "SCENA-VIEWER" &&
        asset.model_viewer_loaded === true &&
        asset.model_viewer_canvas_ready === true &&
        asset.scena_render_status === "passed" &&
        asset.scena_pixels_nonblack > 0,
      );

    return {
      schema: "scena.scena_viewer_model_viewer_parity_proof.v1",
      status: passed ? "passed" : "failed",
      proof_class: "three_asset_side_by_side",
      visual_proof: "side-by-side-screenshot",
      model_viewer_package: "@google/model-viewer",
      screenshot_selector: "section[data-proof=\"scena-viewer-model-viewer-parity\"]",
      assets,
    };
  };

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
  annotation.dataset.priority = "10";
  annotation.dataset.width = "72";
  annotation.dataset.height = "24";
  annotation.textContent = "Bearing";
  annotation.style.cssText = "padding:4px 7px;background:#f8fafc;color:#0f172a;font:12px system-ui,sans-serif;border-radius:4px";
  viewer.append(annotation);
  const clampAnnotation = document.createElement("span");
  clampAnnotation.slot = "annotation";
  clampAnnotation.id = "clamped-label";
  clampAnnotation.dataset.position = "1 0 0";
  clampAnnotation.dataset.priority = "5";
  clampAnnotation.dataset.width = "72";
  clampAnnotation.dataset.height = "24";
  clampAnnotation.textContent = "Clamp";
  clampAnnotation.style.cssText = annotation.style.cssText;
  viewer.append(clampAnnotation);
  const overlapAnnotation = document.createElement("span");
  overlapAnnotation.slot = "annotation";
  overlapAnnotation.id = "overlap-label";
  overlapAnnotation.dataset.position = "1 0 0";
  overlapAnnotation.dataset.priority = "1";
  overlapAnnotation.dataset.width = "72";
  overlapAnnotation.dataset.height = "24";
  overlapAnnotation.textContent = "Overlap";
  overlapAnnotation.style.cssText = annotation.style.cssText;
  viewer.append(overlapAnnotation);

  const ready = once(viewer, "scena-viewer-ready");
  document.body.append(viewer);
  const readyDetail = await ready;
  await nextFrame();

  const root = viewer.shadowRoot;
  const canvas = root.querySelector("canvas");
  const progress = root.querySelector("[part=progress]");
  const progressBar = root.querySelector("[part=progress-bar]");
  const progressSequence = [];
  async function renderProgress(detail) {
    const progressRendered = once(viewer, "scena-viewer-progress-rendered");
    viewer.dispatchEvent(new CustomEvent("scena-viewer-progress", {
      bubbles: true,
      detail,
    }));
    const rendered = await progressRendered;
    progressSequence.push({
      phase: rendered.phase,
      text: rendered.text,
      valueNow: progress.getAttribute("aria-valuenow"),
      barTransform: progressBar.style.transform,
      hidden: progress.hidden,
    });
    return rendered;
  }
  await renderProgress({ phase: "loading", ariaText: "Loading model" });
  const progressDetail = await renderProgress({ phase: "fetching", value: 0.42, ariaText: "Fetching model" });

  const variantsReady = once(viewer, "scena-viewer-variants-ready");
  viewer.setMaterialVariants([
    { name: "midnight", label: "Midnight" },
    { name: "noon", label: "Noon" },
  ], "midnight");
  const variantReadyDetail = await variantsReady;
  const variantChange = once(viewer, "scena-viewer-variant-change");
  const variantPicker = root.querySelector("[part=variant-picker]");
  variantPicker.value = "noon";
  variantPicker.dispatchEvent(new Event("change", { bubbles: true }));
  const variantChangeDetail = await variantChange;
  const variantRender = await renderSelectedVariantIntoViewer(viewer, "webgl2", variantChangeDetail.name);

  const annotationRequest = once(viewer, "scena-viewer-annotations-request");
  viewer.requestAnnotationProjections();
  const annotationRequestDetail = await annotationRequest;
  const annotationsRendered = once(viewer, "scena-viewer-annotations-rendered");
  viewer.setAnnotationProjections([
    { id: "bearing-label", x: 144, y: 72, visible: true },
    { id: "clamped-label", x: -24, y: 260, visible: true },
    { id: "overlap-label", x: 146, y: 74, visible: true },
  ]);
  const annotationsRenderedDetail = await annotationsRendered;
  const firstAnnotationTransform = getComputedStyle(annotation).transform;
  const clampedEntry = annotationsRenderedDetail.layout_report.entries.find((entry) => entry.id === "clamped-label");
  const overlapEntry = annotationsRenderedDetail.layout_report.entries.find((entry) => entry.id === "overlap-label");
  const annotationsUpdated = once(viewer, "scena-viewer-annotations-rendered");
  viewer.setAnnotationProjections([
    { id: "bearing-label", x: 188, y: 96, visible: true },
    { id: "clamped-label", x: -24, y: 260, visible: true },
    { id: "overlap-label", x: 190, y: 98, visible: true },
  ]);
  const annotationsUpdatedDetail = await annotationsUpdated;
  const secondAnnotationTransform = getComputedStyle(annotation).transform;

  const inspectorRendered = once(viewer, "scena-viewer-inspector-rendered");
  const inspectorSnapshot = await loadInspectorSnapshot();
  viewer.setInspectorSnapshot(inspectorSnapshot);
  const inspectorDetail = await inspectorRendered;

  const keyboard = once(viewer, "scena-viewer-key-control");
  viewer.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowLeft" }));
  const keyboardDetail = await keyboard;
  const dropDetail = await dispatchDrop(viewer);
  const dropRender = await renderDroppedFileIntoViewer(viewer, "webgl2", dropDetail);
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
    progress_sequence: progressSequence,
    variant_names: variantReadyDetail.names,
    variant_change: variantChangeDetail.name,
    variant_render_status: variantRender.status,
    variant_render_workflow: variantRender.workflow,
    variant_render_selected: variantRender.metadata.selected_variant,
    variant_render_active: variantRender.metadata.active_variant,
    variant_render_green_dominant: variantRender.green_dominant,
    variant_render_pixels_nonblack: variantRender.pixels.nonblack,
    annotation_count: annotationRequestDetail.anchors.length,
    annotation_visible: annotationsRenderedDetail.visible,
    annotation_update_visible: annotationsUpdatedDetail.visible,
    annotation_layout_entries: annotationsRenderedDetail.layout_report.entries.length,
    annotation_clamped_visible: Boolean(clampedEntry && clampedEntry.visible && clampedEntry.x === 0 && clampedEntry.y < clampedEntry.original_y),
    annotation_overlap_hidden: Boolean(overlapEntry && overlapEntry.visible === false && overlapEntry.hidden_reason === "overlap"),
    annotation_tracking_sequence: [firstAnnotationTransform, secondAnnotationTransform],
    annotation_transform: secondAnnotationTransform,
    inspector_overlay: inspectorDetail.overlay,
    inspector_warnings: inspectorDetail.warnings,
    inspector_fixture_schema: inspectorSnapshot.schema,
    inspector_fixture_source: inspectorSnapshot.source,
    keyboard_action: keyboardDetail.action,
    drop_accepted_names: dropDetail.accepted.names,
    drop_rejected_names: dropDetail.rejected.names,
    drop_error_message: dropDetail.rejected.message,
    drop_render_status: dropRender.status,
    drop_render_workflow: dropRender.workflow,
    drop_render_file_name: dropRender.metadata.file_name,
    drop_render_roots: dropRender.metadata.roots,
    drop_render_pixels_nonblack: dropRender.pixels.nonblack,
    drop_render_auto_frame_status: dropRender.metadata.auto_frame.status,
    drop_render_auto_frame_proof_class: dropRender.metadata.auto_frame.proof_class,
    drop_render_auto_frame_inside_viewport: dropRender.metadata.auto_frame.inside_viewport,
    drop_render_auto_frame_centered: dropRender.metadata.auto_frame.centered,
    drop_render_auto_frame_fill_fraction: dropRender.metadata.auto_frame.fill_fraction,
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
    Array.isArray(checks.progress_sequence) &&
    checks.progress_sequence.length === 2 &&
    checks.progress_sequence[0].phase === "loading" &&
    checks.progress_sequence[0].valueNow === null &&
    checks.progress_sequence[0].barTransform === "scaleX(0.35)" &&
    checks.progress_sequence[1].phase === "fetching" &&
    checks.progress_sequence[1].valueNow === "42" &&
    checks.progress_sequence[1].barTransform === "scaleX(0.42)" &&
    Array.isArray(checks.variant_names) &&
    checks.variant_names.includes("midnight") &&
    checks.variant_names.includes("noon") &&
    checks.variant_change === "noon" &&
    checks.variant_render_status === "passed" &&
    checks.variant_render_workflow === "scena-viewer-material-variant-render" &&
    checks.variant_render_selected === "noon" &&
    checks.variant_render_active === "noon" &&
    checks.variant_render_green_dominant === true &&
    checks.variant_render_pixels_nonblack > 0 &&
    checks.annotation_count === 3 &&
    checks.annotation_visible === 2 &&
    checks.annotation_update_visible === 2 &&
    checks.annotation_layout_entries === 3 &&
    checks.annotation_clamped_visible === true &&
    checks.annotation_overlap_hidden === true &&
    Array.isArray(checks.annotation_tracking_sequence) &&
    checks.annotation_tracking_sequence.length === 2 &&
    checks.annotation_tracking_sequence[0] !== checks.annotation_tracking_sequence[1] &&
    checks.annotation_transform !== "none" &&
    checks.inspector_overlay === "Diagnostics" &&
    checks.inspector_warnings === 1 &&
    checks.keyboard_action === "orbit-left" &&
    checks.drop_accepted_names.includes("accepted-machine.glb") &&
    checks.drop_rejected_names.includes("notes.txt") &&
    checks.drop_error_message.includes("notes.txt") &&
    checks.drop_render_status === "passed" &&
    checks.drop_render_workflow === "scena-viewer-drop-render" &&
    checks.drop_render_file_name === "accepted-machine.glb" &&
    checks.drop_render_roots > 0 &&
    checks.drop_render_pixels_nonblack > 0 &&
    checks.drop_render_auto_frame_status === "passed" &&
    checks.drop_render_auto_frame_proof_class === "viewer-level-auto-framing" &&
    checks.drop_render_auto_frame_inside_viewport === true &&
    checks.drop_render_auto_frame_centered === true &&
    checks.drop_render_auto_frame_fill_fraction > 0.2 &&
    checks.drop_render_auto_frame_fill_fraction <= 0.75;

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

function dispatchPointer(target, type, options) {
  target.dispatchEvent(new PointerEvent(type, {
    bubbles: true,
    cancelable: true,
    isPrimary: options.isPrimary ?? true,
    pointerId: options.pointerId,
    pointerType: options.pointerType || "touch",
    clientX: options.x,
    clientY: options.y,
  }));
}

window.scenaViewerMobileA11yProbe = async function scenaViewerMobileA11yProbe() {
  await ensureInit();
  defineScenaViewer();
  await customElements.whenDefined("scena-viewer");
  document.body.style.margin = "0";

  const viewer = document.createElement("scena-viewer");
  viewer.dataset.proof = "mobile-a11y";
  viewer.setAttribute("src", "/fixtures/gltf/non_ndc_camera_scene.gltf");
  viewer.setAttribute("camera-controls", "");
  viewer.style.cssText = "display:block;width:100vw;max-width:100%;height:220px;background:#111827;color:#f8fafc";
  const ready = once(viewer, "scena-viewer-ready");
  document.body.append(viewer);
  await ready;
  await nextFrame();

  const root = viewer.shadowRoot;
  const canvas = root.querySelector("canvas");

  const pinch = once(viewer, "scena-viewer-gesture-control");
  dispatchPointer(viewer, "pointerdown", { pointerId: 1, pointerType: "touch", x: 120, y: 120 });
  dispatchPointer(viewer, "pointerdown", { pointerId: 2, pointerType: "touch", isPrimary: false, x: 220, y: 120 });
  dispatchPointer(viewer, "pointermove", { pointerId: 2, pointerType: "touch", isPrimary: false, x: 250, y: 120 });
  const pinchDetail = await pinch;
  dispatchPointer(viewer, "pointerup", { pointerId: 1, pointerType: "touch", x: 120, y: 120 });
  dispatchPointer(viewer, "pointerup", { pointerId: 2, pointerType: "touch", isPrimary: false, x: 250, y: 120 });

  const orbit = once(viewer, "scena-viewer-gesture-control");
  dispatchPointer(viewer, "pointerdown", { pointerId: 3, pointerType: "touch", x: 120, y: 160 });
  dispatchPointer(viewer, "pointermove", { pointerId: 3, pointerType: "touch", x: 146, y: 174 });
  const orbitDetail = await orbit;
  dispatchPointer(viewer, "pointerup", { pointerId: 3, pointerType: "touch", x: 146, y: 174 });

  const wheel = once(viewer, "scena-viewer-gesture-control");
  viewer.dispatchEvent(new WheelEvent("wheel", {
    bubbles: true,
    cancelable: true,
    deltaY: -120,
  }));
  const wheelDetail = await wheel;

  const keyboard = once(viewer, "scena-viewer-key-control");
  viewer.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "Home" }));
  const keyboardDetail = await keyboard;

  const rect = viewer.getBoundingClientRect();
  const checks = {
    viewport_width: window.innerWidth,
    viewer_width: Math.round(rect.width),
    viewer_overflows_x: rect.left < 0 || rect.right > window.innerWidth,
    host_role: viewer.getAttribute("role"),
    host_tabindex: viewer.getAttribute("tabindex"),
    canvas_touch_action: getComputedStyle(canvas).touchAction,
    pinch_action: pinchDetail.action,
    pinch_pointers: pinchDetail.pointers,
    pinch_delta_positive: pinchDetail.deltaDistance > 0,
    orbit_action: orbitDetail.action,
    orbit_pointer_type: orbitDetail.pointerType,
    orbit_delta_x: orbitDetail.deltaX,
    orbit_delta_y: orbitDetail.deltaY,
    wheel_action: wheelDetail.action,
    wheel_delta_y: wheelDetail.deltaY,
    keyboard_action: keyboardDetail.action,
  };

  const passed =
    checks.viewport_width <= 390 &&
    checks.viewer_width <= checks.viewport_width &&
    checks.viewer_overflows_x === false &&
    checks.host_role === "img" &&
    checks.host_tabindex === "0" &&
    checks.canvas_touch_action === "none" &&
    checks.pinch_action === "pinch-zoom" &&
    checks.pinch_pointers === 2 &&
    checks.pinch_delta_positive === true &&
    checks.orbit_action === "orbit" &&
    checks.orbit_pointer_type === "touch" &&
    checks.orbit_delta_x === 26 &&
    checks.orbit_delta_y === 14 &&
    checks.wheel_action === "wheel-zoom" &&
    checks.wheel_delta_y === -120 &&
    checks.keyboard_action === "reset-view";

  return {
    schema: "scena.scena_viewer_mobile_a11y_browser_proof.v1",
    status: passed ? "passed" : "failed",
    proof_class: "browser-demo",
    visual_proof: "browser-demo",
    screenshot_selector: "scena-viewer[data-proof=\"mobile-a11y\"]",
    checks,
  };
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

function probeCanvasOutputColorSpace(backend, canvas) {
  const state = canvas.__scenaOutputColorSpace || {};
  if (backend === "webgl2") {
    const gl = canvas.getContext("webgl2", { antialias: false, preserveDrawingBuffer: true });
    const supported = !!gl && "drawingBufferColorSpace" in gl;
    const effective = supported ? gl.drawingBufferColorSpace : state.effective || null;
    return {
      api: "webgl2",
      property: "drawingBufferColorSpace",
      requested: state.requested || null,
      supported,
      configured: effective === state.requested,
      effective,
      display_p3: effective === "display-p3",
      injected_by: state.injected_by || null,
      error: state.error || null,
    };
  }
  return {
    api: "webgpu",
    property: "GPUCanvasConfiguration.colorSpace",
    requested: state.requested || null,
    supported: state.supported === true,
    configured: state.configured === true,
    effective: state.effective || null,
    display_p3: state.display_p3 === true,
    injected_by: state.injected_by || null,
    error: state.error || null,
  };
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
  result.canvas_output_color_space = probeCanvasOutputColorSpace(backend, canvas);
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

window.scenaAssetDoctorBrowserProbe = async function scenaAssetDoctorBrowserProbe() {
  await ensureInit();
  const raw = await m6AssetDoctorBrowserProbe(
    "/fixtures/gltf/unsupported_required_extension.gltf",
  );
  const result = JSON.parse(raw);
  const finding =
    result.doctor &&
    Array.isArray(result.doctor.findings) &&
    result.doctor.findings.find((entry) => entry.code === "unsupported_required_extension");
  const section = document.createElement("section");
  section.dataset.proof = "asset-doctor-browser";
  section.style.cssText =
    "box-sizing:border-box;width:560px;padding:16px;background:#101820;color:#f3f4f6;font:13px system-ui,sans-serif";
  const title = document.createElement("strong");
  title.textContent = "Asset doctor";
  const code = document.createElement("p");
  code.dataset.field = "code";
  code.textContent = finding ? finding.code : "missing-finding";
  const message = document.createElement("p");
  message.dataset.field = "message";
  message.textContent = finding ? finding.message : "no message";
  const fix = document.createElement("p");
  fix.dataset.field = "fix";
  fix.textContent = finding ? finding.suggested_fix : "no fix";
  section.append(title, code, message, fix);
  document.body.append(section);
  return {
    ...result,
    displayed_code: code.textContent === "unsupported_required_extension",
    displayed_fix: fix.textContent,
    screenshot_selector: "section[data-proof=\"asset-doctor-browser\"]",
  };
};

window.scenaM6DisplayP3OutputProbe = async function scenaM6DisplayP3OutputProbe(backend) {
  return runProbe(backend, "display-p3-output", (canvas) =>
    m6RenderDisplayP3Probe(canvas, backend),
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

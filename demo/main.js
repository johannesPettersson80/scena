import init, {
  attach_to_canvas,
  background_scheme_css_color,
  connector_marker_positions,
  detach_from_canvas,
  connector_replay_active,
  forward_pointer_event,
  load_connector_snap_from_bytes,
  load_gltf_with_floor_from_bytes,
  load_gltf_with_view_from_bytes,
  load_material_presets_scene,
  replay_connector_snap,
  resize,
  set_fixed_exposure_ev,
  tick,
  transfer_renderer_to,
} from "./pkg/scena.js?v=1.10.3-public-7e371c33d0b4";

const WASM_URL = "./pkg/scena_bg.wasm?v=1.10.3-public-7e371c33d0b4";
const MAX_CANVAS_DIMENSION = 2048;
const MIN_CANVAS_RENDER_SCALE = 1.5;
const MAX_DEVICE_PIXEL_RATIO = 2;
const MIN_RENDERED_FRAME_COUNT = 1;

const MATERIALS = [
  ["matte", "Matte", "MaterialDesc::matte(Color::BLUE)"],
  ["plastic", "Plastic", "MaterialDesc::plastic(Color::BLUE)"],
  ["metal", "Metal", "MaterialDesc::metal(Color::LIGHT_GRAY)"],
  ["rough_metal", "Rough metal", "MaterialDesc::rough_metal(Color::GRAY)"],
  ["chrome", "Chrome", "MaterialDesc::chrome()"],
  ["brushed_steel", "Brushed steel", "MaterialDesc::brushed_steel()"],
  ["clearcoat_plastic", "Clearcoat plastic", "MaterialDesc::clearcoat_plastic(Color::BLUE)"],
  ["satin", "Satin", "assets.material_presets().satin().await?"],
  ["leather", "Leather", "assets.material_presets().leather().await?"],
  ["clear_glass", "Clear glass", "MaterialDesc::clear_glass(Color::COOL_WHITE)"],
  ["frosted_glass", "Frosted glass", "MaterialDesc::frosted_glass(Color::WHITE)"],
  ["rubber", "Rubber", "assets.material_presets().rubber().await?"],
];

const controllers = new Map();
const byteCache = new Map();
let wasmReady = null;
let materialSelection = "chrome";

function ensureWasm() {
  if (!wasmReady) {
    wasmReady = init({ module_or_path: new URL(WASM_URL, import.meta.url) });
  }
  return wasmReady;
}

function setStatus(stage, text) {
  const status = stage.querySelector(".stage-status");
  if (status) status.textContent = text;
}

function canvasSize(canvas) {
  const rect = canvas.getBoundingClientRect();
  const cssWidth = Math.max(1, Math.round(rect.width || canvas.clientWidth || 640));
  const cssHeight = Math.max(1, Math.round(rect.height || canvas.clientHeight || 420));
  const pixelRatio = Math.max(
    MIN_CANVAS_RENDER_SCALE,
    Math.min(window.devicePixelRatio || 1, MAX_DEVICE_PIXEL_RATIO),
  );
  let width = Math.max(1, Math.round(cssWidth * pixelRatio));
  let height = Math.max(1, Math.round(cssHeight * pixelRatio));
  const scale = Math.min(1, MAX_CANVAS_DIMENSION / Math.max(width, height));
  width = Math.max(1, Math.round(width * scale));
  height = Math.max(1, Math.round(height * scale));
  if (canvas.width !== width) canvas.width = width;
  if (canvas.height !== height) canvas.height = height;
  canvas.dataset.renderScale = (width / cssWidth).toFixed(2);
  return { width, height, cssWidth, cssHeight, renderScale: width / cssWidth };
}

function nextAnimationFrame() {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function applyCanvasBackground(scheme) {
  const canvas = document.querySelector(".material-route canvas");
  const stage = document.querySelector(".material-route .stage");
  if (!canvas || !stage) return;
  const cssColor = background_scheme_css_color(scheme);
  canvas.style.background = cssColor;
  stage.style.background = cssColor;
}

async function runMaterialPresetRoute() {
  document.body.innerHTML = `<main class="material-route">
    <section class="section materials">
      <div class="section-head">
        <h1>Material presets</h1>
        <p class="section-copy">browser-rendered WebGL2 material showcase</p>
      </div>
      <div class="stage">
        <canvas aria-label="browser-rendered WebGL2 material showcase"></canvas>
        <div class="stage-status">rendering browser material showcase</div>
      </div>
    </section>
  </main>`;
  await ensureWasm();
  const stage = document.querySelector(".material-route .stage");
  const canvas = stage.querySelector("canvas");
  canvasSize(canvas);
  applyCanvasBackground("dark_studio");
  const app = await load_material_presets_scene(canvas.width, canvas.height);
  await attach_to_canvas(app, canvas);
  resize(app, canvas.width, canvas.height);
  set_fixed_exposure_ev(app, 0.0);
  await nextAnimationFrame();
  tick(app, 0.016);
  await nextAnimationFrame();
  tick(app, 0.016);
  setStatus(stage, "browser-rendered WebGL2 material showcase");
  window.__scenaShowcaseProbe = {
    controllers() {
      return [
        {
          scene: "material-presets",
          loaded: true,
          active: true,
          status: stage.querySelector(".stage-status")?.textContent || "",
        },
      ];
    },
    materialSelection() {
      return "material-presets";
    },
  };
}

async function fetchBytes(path) {
  const url = new URL(path, window.location.href).toString();
  let promise = byteCache.get(url);
  if (!promise) {
    promise = fetch(url).then(async (response) => {
      if (!response.ok) throw new Error(`${path} HTTP ${response.status}`);
      return response.arrayBuffer();
    });
    byteCache.set(url, promise);
  }
  const buffer = await promise;
  return new Uint8Array(buffer);
}

class LiveStage {
  constructor(stage) {
    this.stage = stage;
    this.canvas = stage.querySelector("canvas");
    this.scene = stage.dataset.scene;
    this.app = null;
    this.loaded = false;
    this.active = false;
    this.attached = false;
    this.renderScheduled = false;
    this.replayActive = false;
    this.lastFrameAt = performance.now();
    this.framesSinceAttach = 0;
    this.activationGeneration = 0;
    this.visibleGeneration = 0;
    this.replayTimer = null;
    this.pointerDown = false;
    this.loadingPromise = null;
    this.wirePointer();
    this.wireResize();
  }

  async activate() {
    detachOtherStages(this.scene);
    this.active = true;
    this.activationGeneration += 1;
    this.visibleGeneration = 0;
    this.framesSinceAttach = 0;
    if (!this.loaded) {
      await this.load();
    }
    if (!this.attached) {
      await this.attachCurrentCanvas();
    }
    if (this.scene === "connector") this.startConnectorLoop();
    this.requestRender();
  }

  deactivate() {
    this.active = false;
    window.clearTimeout(this.replayTimer);
    this.replayTimer = null;
    this.detach();
  }

  async load() {
    if (this.loadingPromise) {
      await this.loadingPromise;
      return;
    }
    this.loadingPromise = this.loadNow();
    try {
      await this.loadingPromise;
    } finally {
      this.loadingPromise = null;
    }
  }

  async loadNow() {
    const wasmReadyPromise = ensureWasm();
    setStatus(this.stage, "loading");
    try {
      if (this.scene === "hero") await this.loadHero(wasmReadyPromise);
      if (this.scene === "material") await this.loadMaterial(materialSelection, wasmReadyPromise);
      if (this.scene === "model") await this.loadModel(this.stage.dataset.sample, wasmReadyPromise);
      if (this.scene === "connector") await this.loadConnector(wasmReadyPromise);
      this.loaded = true;
      setStatus(this.stage, "rendering");
    } catch (error) {
      console.error(`showcase ${this.scene} failed`, error);
      setStatus(this.stage, `render failed: ${String(error).slice(0, 120)}`);
    }
  }

  async prepareScene() {
    if (this.loaded) return;
    if (this.loadingPromise) {
      await this.loadingPromise;
      return;
    }
    if (this.scene === "hero") return;
    this.loadingPromise = this.prepareSceneNow();
    try {
      await this.loadingPromise;
    } finally {
      this.loadingPromise = null;
    }
  }

  async prepareSceneNow() {
    const wasmReadyPromise = ensureWasm();
    setStatus(this.stage, "preparing");
    try {
      if (this.scene === "material") {
        this.app = await this.buildMaterial(materialSelection, wasmReadyPromise);
      }
      if (this.scene === "model") {
        this.app = await this.buildModel(this.stage.dataset.sample, wasmReadyPromise);
      }
      if (this.scene === "connector") {
        this.app = await this.buildConnector(wasmReadyPromise);
        this.replayActive = false;
        this.updateConnectorMarkers();
      }
      this.loaded = Boolean(this.app);
      if (this.loaded) setStatus(this.stage, "ready");
    } catch (error) {
      console.error(`prepareScene ${this.scene} failed`, error);
      setStatus(this.stage, `prepare failed: ${String(error).slice(0, 120)}`);
    }
  }

  async attach(app) {
    this.app = app;
    await this.attachCurrentCanvas();
  }

  async attachCurrentCanvas() {
    if (!this.app) return;
    transferWarmRendererTo(this);
    setStatus(this.stage, "rendering");
    const { width, height } = canvasSize(this.canvas);
    await attach_to_canvas(this.app, this.canvas);
    this.attached = true;
    this.framesSinceAttach = 0;
    this.lastFrameAt = performance.now();
    resize(this.app, width, height);
    await nextAnimationFrame();
  }

  detach() {
    if (!this.app || !this.attached) return;
    try {
      detach_from_canvas(this.app);
    } catch (error) {
      console.error(`detach ${this.scene} failed`, error);
    }
    this.attached = false;
  }

  async loadHero(wasmReadyPromise = ensureWasm()) {
    const app = await this.buildHero(wasmReadyPromise);
    await this.attach(app);
  }

  async buildHero(wasmReadyPromise = ensureWasm()) {
    const { width, height } = canvasSize(this.canvas);
    const bytesPromise = fetchBytes("/samples/connector-snap/connector_snap_assembly.glb");
    await wasmReadyPromise;
    const bytes = await bytesPromise;
    return load_gltf_with_view_from_bytes(bytes, width, height, true, -0.42, 0.28);
  }

  async loadMaterial(preset, wasmReadyPromise = ensureWasm()) {
    const app = await this.buildMaterial(preset, wasmReadyPromise);
    await this.attach(app);
  }

  async buildMaterial(preset, wasmReadyPromise = ensureWasm()) {
    materialSelection = preset;
    this.stage.dataset.material = preset;
    const { width, height } = canvasSize(this.canvas);
    await wasmReadyPromise;
    return load_material_presets_scene(width, height);
  }

  async loadModel(path, wasmReadyPromise = ensureWasm()) {
    const app = await this.buildModel(path, wasmReadyPromise);
    await this.attach(app);
  }

  async buildModel(path, wasmReadyPromise = ensureWasm()) {
    const { width, height } = canvasSize(this.canvas);
    const bytesPromise = fetchBytes(path);
    await wasmReadyPromise;
    const bytes = await bytesPromise;
    return load_gltf_with_floor_from_bytes(bytes, width, height);
  }

  async loadDropped(bytes, label) {
    await ensureWasm();
    setStatus(this.stage, `loading ${label}`);
    const { width, height } = canvasSize(this.canvas);
    const app = await load_gltf_with_floor_from_bytes(bytes, width, height);
    await this.attach(app);
    this.loaded = true;
    setStatus(this.stage, "rendering");
    this.requestRender();
  }

  async loadConnector(wasmReadyPromise = ensureWasm()) {
    const app = await this.buildConnector(wasmReadyPromise);
    await this.attach(app);
    this.replayActive = false;
    this.updateConnectorMarkers();
  }

  async buildConnector(wasmReadyPromise = ensureWasm()) {
    const { width, height } = canvasSize(this.canvas);
    const bytesPromise = Promise.all([
      fetchBytes("/samples/connector-snap/drive_unit.glb"),
      fetchBytes("/samples/connector-snap/load_unit.glb"),
    ]);
    await wasmReadyPromise;
    const [driveBytes, loadBytes] = await bytesPromise;
    return load_connector_snap_from_bytes(driveBytes, loadBytes, width, height);
  }

  requestRender() {
    if (!this.active || !this.app || this.renderScheduled) return;
    this.renderScheduled = true;
    requestAnimationFrame(() => {
      this.renderScheduled = false;
      if (!this.active || !this.app) return;
      try {
        const now = performance.now();
        const dtSeconds = Math.min(0.08, Math.max(0.001, (now - this.lastFrameAt) / 1000));
        this.lastFrameAt = now;
        tick(this.app, dtSeconds);
        this.framesSinceAttach += 1;
        this.updateConnectorMarkers();
        if (this.framesSinceAttach < MIN_RENDERED_FRAME_COUNT) {
          setStatus(this.stage, "rendering");
          this.requestRender();
          return;
        }
        this.visibleGeneration = this.activationGeneration;
        if (this.scene === "connector") {
          this.replayActive = connector_replay_active(this.app);
          if (this.replayActive) {
            setStatus(this.stage, "mating connectors");
            this.requestRender();
            return;
          }
          setStatus(this.stage, "assembled");
          this.scheduleReplay();
          return;
        }
        setStatus(this.stage, "rendered");
      } catch (error) {
        console.error(`showcase render ${this.scene} failed`, error);
        setStatus(this.stage, `render failed: ${String(error).slice(0, 120)}`);
      }
    });
  }

  startConnectorLoop() {
    if (!this.app || this.replayActive) return;
    this.replayActive = true;
    try {
      replay_connector_snap(this.app);
      this.lastFrameAt = performance.now();
      this.requestRender();
    } catch (error) {
      console.error("connector replay failed", error);
      setStatus(this.stage, `replay failed: ${String(error).slice(0, 120)}`);
    }
  }

  scheduleReplay() {
    if (!this.active || this.replayTimer) return;
    this.replayTimer = window.setTimeout(() => {
      this.replayTimer = null;
      if (this.active) this.startConnectorLoop();
    }, 2200);
  }

  updateConnectorMarkers() {
    if (this.scene !== "connector" || !this.app) return;
    const markers = this.stage.querySelectorAll(".connector-marker");
    try {
      const positions = connector_marker_positions(
        this.app,
        Math.max(1, Math.round(this.canvas.clientWidth)),
        Math.max(1, Math.round(this.canvas.clientHeight)),
      );
      for (const marker of markers) {
        const position = positions?.[marker.dataset.connector];
        if (!position?.visible) {
          marker.dataset.visible = "false";
          continue;
        }
        marker.style.left = `${position.x}px`;
        marker.style.top = `${position.y}px`;
        marker.dataset.visible = "true";
      }
    } catch {
      for (const marker of markers) marker.dataset.visible = "false";
    }
  }

  wirePointer() {
    this.canvas.addEventListener("pointerdown", (event) => {
      if (!this.app) return;
      this.pointerDown = true;
      this.canvas.setPointerCapture?.(event.pointerId);
      forward_pointer_event(this.app, "down", event.offsetX, event.offsetY, 0, 0);
    });
    this.canvas.addEventListener("pointermove", (event) => {
      if (!this.app || !this.pointerDown) return;
      forward_pointer_event(
        this.app,
        "move",
        event.offsetX,
        event.offsetY,
        event.movementX,
        event.movementY,
      );
      this.requestRender();
    });
    const endPointer = (event) => {
      if (!this.app || !this.pointerDown) return;
      this.pointerDown = false;
      forward_pointer_event(this.app, "up", event.offsetX || 0, event.offsetY || 0, 0, 0);
    };
    this.canvas.addEventListener("pointerup", endPointer);
    this.canvas.addEventListener("pointercancel", endPointer);
    this.canvas.addEventListener(
      "wheel",
      (event) => {
        if (!this.app) return;
        event.preventDefault();
        forward_pointer_event(this.app, "wheel", event.offsetX, event.offsetY, 0, event.deltaY);
        this.requestRender();
      },
      { passive: false },
    );
  }

  wireResize() {
    let scheduled = false;
    const onResize = () => {
      if (scheduled) return;
      scheduled = true;
      requestAnimationFrame(() => {
        scheduled = false;
        const { width, height } = canvasSize(this.canvas);
        if (!this.app) return;
        if (!this.attached) return;
        try {
          resize(this.app, width, height);
          this.requestRender();
        } catch (error) {
          console.error(`resize ${this.scene} failed`, error);
        }
      });
    };
    window.addEventListener("resize", onResize);
    if ("ResizeObserver" in window) new ResizeObserver(onResize).observe(this.canvas);
  }
}

function createMaterialThumbs() {
  const grid = document.getElementById("material-choices");
  if (!grid) return;
  const buttons = MATERIALS.map(([id, label]) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "material-choice";
    button.dataset.material = id;
    button.textContent = label;
    button.addEventListener("click", () => selectMaterial(id));
    return button;
  });
  grid.replaceChildren(...buttons);
  updateMaterialSelection();
}

function selectMaterial(id) {
  materialSelection = id;
  updateMaterialSelection();
}

function updateMaterialSelection() {
  for (const button of document.querySelectorAll("[data-material]")) {
    const active = button.dataset.material === materialSelection;
    button.classList.toggle("active", active);
    button.setAttribute("aria-pressed", active ? "true" : "false");
  }
  const selected = MATERIALS.find(([id]) => id === materialSelection);
  const code = document.getElementById("material-code");
  if (code && selected) code.textContent = `let material = ${selected[2]};`;
  const selectedLabel = document.getElementById("material-selected");
  if (selectedLabel && selected) selectedLabel.textContent = `Selected: ${selected[1]}`;
}

function detachOtherStages(activeScene) {
  for (const [scene, controller] of controllers) {
    if (scene === activeScene) continue;
    controller.active = false;
    window.clearTimeout(controller.replayTimer);
    controller.replayTimer = null;
    controller.detach();
  }
}

function transferWarmRendererTo(target) {
  if (!target.app) return false;
  for (const controller of controllers.values()) {
    if (controller === target || !controller.app) continue;
    try {
      if (transfer_renderer_to(controller.app, target.app)) {
        controller.attached = false;
        return true;
      }
    } catch (error) {
      console.error(`renderer transfer ${controller.scene} -> ${target.scene} failed`, error);
    }
  }
  return false;
}

function wireSamples() {
  const modelController = () => controllers.get("model");
  for (const button of document.querySelectorAll(".sample-chip[data-sample]")) {
    button.addEventListener("click", async () => {
      const controller = modelController();
      if (!controller) return;
      controller.stage.dataset.sample = button.dataset.sample;
      if (!controller.active) {
        controller.stage.scrollIntoView({ behavior: "smooth", block: "center" });
        return;
      }
      setStatus(controller.stage, `loading ${button.dataset.label}`);
      try {
        await controller.loadModel(button.dataset.sample);
        controller.loaded = true;
        setStatus(controller.stage, "rendering");
        controller.requestRender();
      } catch (error) {
        console.error("sample load failed", error);
        setStatus(controller.stage, `load failed: ${String(error).slice(0, 120)}`);
      }
    });
  }
}

function wireDrop() {
  const stage = document.querySelector('[data-scene="model"]');
  const layer = stage?.querySelector(".drop-layer");
  const controller = () => controllers.get("model");
  if (!stage || !layer) return;
  for (const type of ["dragenter", "dragover"]) {
    window.addEventListener(type, (event) => {
      event.preventDefault();
      layer.classList.add("over");
    });
  }
  for (const type of ["dragleave", "drop"]) {
    window.addEventListener(type, () => layer.classList.remove("over"));
  }
  window.addEventListener("drop", async (event) => {
    event.preventDefault();
    const file = Array.from(event.dataTransfer?.files || []).find((entry) =>
      /\.(glb|gltf)$/i.test(entry.name),
    );
    if (!file) return;
    const active = controller();
    if (!active) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    await active.loadDropped(bytes, file.name);
  });
}

function wireCopyButtons() {
  async function copyText(text, button) {
    await navigator.clipboard.writeText(text);
    const previous = button.textContent;
    button.textContent = "Copied";
    window.setTimeout(() => {
      button.textContent = previous;
    }, 1000);
  }
  for (const button of document.querySelectorAll("[data-copy]")) {
    button.addEventListener("click", () => {
      const target = document.querySelector(button.dataset.copy);
      const text = target?.innerText || target?.textContent || "";
      copyText(text, button).catch((error) => console.error("copy failed", error));
    });
  }
  for (const button of document.querySelectorAll("[data-copy-text]")) {
    button.addEventListener("click", () => {
      copyText(button.dataset.copyText, button).catch((error) => console.error("copy failed", error));
    });
  }
}

function wireReplayButtons() {
  for (const button of document.querySelectorAll(".replay")) {
    button.addEventListener("click", () => controllers.get("connector")?.startConnectorLoop());
  }
}

function observeStages() {
  const stages = Array.from(document.querySelectorAll(".stage[data-scene]"));
  for (const stage of stages) {
    const controller = new LiveStage(stage);
    controllers.set(stage.dataset.scene, controller);
  }
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const controller = controllers.get(entry.target.dataset.scene);
        if (!controller) continue;
        if (entry.isIntersecting) {
          controller.activate();
        } else {
          controller.deactivate();
        }
      }
    },
    { rootMargin: "220px 0px", threshold: 0.18 },
  );
  for (const stage of stages) observer.observe(stage);
}

function waitForHeroRendered() {
  const hero = controllers.get("hero");
  if (!hero) return Promise.resolve();
  return new Promise((resolve) => {
    const check = () => {
      if (hero.visibleGeneration > 0) {
        resolve();
        return;
      }
      requestAnimationFrame(check);
    };
    check();
  });
}

function idleCallback(callback, options) {
  if ("requestIdleCallback" in window) {
    return window.requestIdleCallback(callback, options);
  }
  return window.setTimeout(callback, 200);
}

async function schedulePrefetch() {
  await waitForHeroRendered();
  idleCallback(
    () => {
      (async () => {
        for (const scene of ["material", "model", "connector"]) {
          await controllers.get(scene)?.prepareScene();
        }
      })().catch((error) => console.error("showcase prefetch failed", error));
    },
    { timeout: 4000 },
  );
}

function exposeProbe() {
  window.__scenaShowcaseProbe = {
    controllers() {
      return Array.from(controllers.values()).map((controller) => ({
        scene: controller.scene,
        loaded: controller.loaded,
        active: controller.active,
        attached: controller.attached,
        prepared: controller.loaded && !controller.attached,
        status: controller.stage.querySelector(".stage-status")?.textContent || "",
        renderScale: Number(controller.canvas.dataset.renderScale || "1"),
        activationGeneration: controller.activationGeneration,
        visibleGeneration: controller.visibleGeneration,
        renderedForActivation: controller.visibleGeneration === controller.activationGeneration,
      }));
    },
    materialSelection() {
      return materialSelection;
    },
  };
}

if (new URLSearchParams(window.location.search).get("sample") === "material-presets") {
  runMaterialPresetRoute().catch((error) => {
    console.error("material preset route failed", error);
    document.body.textContent = `render failed: ${String(error)}`;
  });
} else {
  createMaterialThumbs();
  wireSamples();
  wireDrop();
  wireCopyButtons();
  wireReplayButtons();
  observeStages();
  exposeProbe();
  schedulePrefetch().catch((error) => console.error("schedulePrefetch failed", error));
}

import init, {
  attach_to_canvas,
  load_material_spheres_scene,
  resize,
  set_background_scheme,
  set_fixed_exposure_ev,
  tick,
} from "./pkg/scena.js?v=20260523-internal-spheres-material-glass-1";

const canvas = document.getElementById("canvas");
const status = document.getElementById("status");
const MAX_RENDER_DIMENSION = 1920;

let app = null;
let attached = false;
let lastFrameAt = performance.now();
let renderQueued = false;

function setStatus(message) {
  status.textContent = message;
}

function applyBufferSize() {
  const rect = canvas.getBoundingClientRect();
  const cssWidth = Math.max(1, Math.round(rect.width || window.innerWidth));
  const cssHeight = Math.max(1, Math.round(rect.height || window.innerHeight));
  const scale = Math.min(1, MAX_RENDER_DIMENSION / Math.max(cssWidth, cssHeight));
  const width = Math.max(1, Math.floor(cssWidth * scale));
  const height = Math.max(1, Math.floor(cssHeight * scale));
  const changed = canvas.width !== width || canvas.height !== height;
  canvas.width = width;
  canvas.height = height;
  return changed;
}

function requestRender() {
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    if (!app || !attached) return;
    const now = performance.now();
    const dt = Math.min(0.05, Math.max(0.0, (now - lastFrameAt) / 1000));
    lastFrameAt = now;
    try {
      tick(app, dt);
      setStatus("rendered");
    } catch (err) {
      console.error("render failed:", err);
      setStatus(`render failed: ${err}`);
      return;
    }
  });
}

async function start() {
  applyBufferSize();
  setStatus("initialising WebAssembly");
  await init({
    module_or_path: new URL(
      "./pkg/scena_bg.wasm?v=20260523-internal-spheres-material-glass-1",
      import.meta.url,
    ),
  });
  setStatus("building sphere scene");
  app = await load_material_spheres_scene(canvas.width, canvas.height);
  setStatus("creating WebGL2 renderer");
  await attach_to_canvas(app, canvas);
  set_background_scheme(app, "dark_studio");
  set_fixed_exposure_ev(app, 0.0);
  attached = true;
  lastFrameAt = performance.now();
  requestRender();
}

window.addEventListener("resize", () => {
  const changed = applyBufferSize();
  if (!app || !attached || !changed) return;
  try {
    resize(app, canvas.width, canvas.height);
    requestRender();
  } catch (err) {
    console.error("resize failed:", err);
    setStatus(`resize failed: ${err}`);
  }
});

start().catch((err) => {
  console.error("material sphere page failed:", err);
  setStatus(`failed: ${err}`);
});

import init, {
  load_gltf_from_bytes,
  load_connector_snap_from_bytes,
  attach_to_canvas,
  connector_replay_active,
  forward_pointer_event,
  replay_connector_snap,
  resize,
  tick,
} from "./pkg/scena.js";

const SAMPLE_GROUPS = [
  {
    label: "Showcase",
    samples: [
      {
        id: "connector-snap",
        label: "Connector snap",
        detail: "drive_unit + load_unit",
        path: "/samples/connector-snap/connector_snap_assembly.glb",
        drivePath: "/samples/connector-snap/drive_unit.glb",
        loadPath: "/samples/connector-snap/load_unit.glb",
        tone: "teal",
        code: "connector",
      },
      {
        id: "drive-unit",
        label: "Drive unit",
        detail: "authored connector: shaft",
        path: "/samples/connector-snap/drive_unit.glb",
        tone: "blue",
        code: "asset",
      },
      {
        id: "load-unit",
        label: "Load unit",
        detail: "authored connector: hub",
        path: "/samples/connector-snap/load_unit.glb",
        tone: "amber",
        code: "asset",
      },
    ],
  },
  {
    label: "Khronos compatibility",
    samples: [
      {
        id: "water-bottle",
        label: "Khronos PBR",
        detail: "textured PBR sample",
        path: "/samples/khronos/WaterBottle.glb",
        tone: "rust",
        code: "asset",
      },
      {
        id: "toy-car",
        label: "Khronos vehicle",
        detail: "official GLB sample",
        path: "/samples/khronos/ToyCar.glb",
        tone: "blue",
        code: "asset",
      },
    ],
  },
];

const SAMPLES = SAMPLE_GROUPS.flatMap((group) => group.samples);

const ORBIT_RADIANS_PER_PIXEL = 0.01;
const ZOOM_SCALE = 0.1;
const MIN_DISTANCE = 0.001;
const MAX_PITCH_RADIANS = 1.553343;
const RESOLUTION_SCALE = Math.min(Math.max(window.devicePixelRatio || 1, 1), 1.5);
const QUERY_PARAMS = new URLSearchParams(window.location.search);
const TIMING_ENABLED = ["perf", "timing"].some((key) => {
  const value = QUERY_PARAMS.get(key);
  return value !== null && value !== "0" && value !== "false";
});

const canvas = document.getElementById("canvas");
const dropzone = document.getElementById("dropzone");
const sampleList = document.getElementById("sample-list");
const statusTitle = document.getElementById("status-title");
const statusDetail = document.getElementById("status-detail");
const codeTitle = document.getElementById("code-title");
const codeSubtitle = document.getElementById("code-subtitle");
const codeSnippet = document.getElementById("code-snippet");
const copyButton = document.getElementById("copy-code");
const replayButton = document.getElementById("replay-button");
const metricFrame = document.getElementById("metric-frame");
const metricBytes = document.getElementById("metric-bytes");
const metricPhase = document.getElementById("metric-phase");
const metricOrbit = document.getElementById("metric-orbit");

let app = null;
let attached = false;
let renderScheduled = false;
let frameCount = 0;
let activeAsset = SAMPLES[0];
let phaseStartedAt = performance.now();
let lastFrameAt = performance.now();
let pointerDown = false;
let orbit = { yaw: -0.48, pitch: 0.31, distance: 2.0 };
let replayActive = false;

function buildSampleButtons() {
  const children = [];
  SAMPLE_GROUPS.forEach((group, index) => {
    if (index > 0) {
      const heading = document.createElement("p");
      heading.className = "sample-group-label";
      heading.textContent = group.label;
      children.push(heading);
    }
    for (const sample of group.samples) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "sample-button";
      button.dataset.sample = sample.id;
      button.dataset.tone = sample.tone;
      button.innerHTML = `<span><strong></strong><small></small></span>`;
      button.querySelector("strong").textContent = sample.label;
      button.querySelector("small").textContent = sample.detail;
      button.addEventListener("click", () => loadSample(sample));
      children.push(button);
    }
  });
  sampleList.replaceChildren(...children);
}

function updateActiveButton() {
  for (const button of sampleList.querySelectorAll(".sample-button")) {
    button.classList.toggle("active", button.dataset.sample === activeAsset.id);
  }
  replayButton.hidden = activeAsset.code !== "connector";
}

function formatBytes(bytes) {
  if (!bytes) return "0";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

function setStatus(title, detail) {
  statusTitle.textContent = title;
  statusDetail.textContent = detail;
}

function setPhase(label) {
  const elapsedSeconds = ((performance.now() - phaseStartedAt) / 1000).toFixed(1);
  metricPhase.textContent = label;
  setStatus(activeAsset.label, `${label} · ${elapsedSeconds}s`);
}

function setReplayStatus() {
  replayActive = true;
  metricPhase.textContent = "replaying";
  setStatus(activeAsset.label, "replaying connector snap");
}

function beginPhase(label) {
  phaseStartedAt = performance.now();
  setPhase(label);
  logDemo(`${activeAsset.label}: ${label}`);
}

function logDemo(message) {
  if (TIMING_ENABLED) console.info(`[scena-demo] ${message}`);
}

function setError(text) {
  attached = false;
  metricPhase.textContent = "error";
  setStatus(activeAsset.label, String(text).slice(0, 180));
}

function updateMetrics(bytes = null) {
  metricFrame.textContent = String(frameCount);
  if (bytes !== null) metricBytes.textContent = formatBytes(bytes);
  metricOrbit.textContent = `${orbit.yaw.toFixed(2)} / ${orbit.pitch.toFixed(2)} / ${orbit.distance.toFixed(2)}`;
}

function rustString(value) {
  return String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function updateCodePanel() {
  codeTitle.textContent = activeAsset.code === "connector" ? "Connector snap" : "Rust";
  codeSubtitle.textContent =
    activeAsset.code === "connector" ? 'scene.mate(&drive, "shaft", &load, "hub")' : activeAsset.path;
  if (activeAsset.code === "connector") {
    codeSnippet.textContent = `let assets = Assets::new();
let drive_part = assets.load_scene("drive_unit.glb").await?;
let load_part  = assets.load_scene("load_unit.glb").await?;

let mut scene = Scene::new();
let drive = scene.instantiate(&drive_part)?;
let load  = scene.instantiate(&load_part)?;

scene.mate(&drive, "shaft", &load, "hub")?;`;
    return;
  }

  codeSnippet.textContent = `let assets = Assets::new();
let scene_asset = assets
    .load_scene("${rustString(activeAsset.path)}")
    .await?;

let mut scene = Scene::new();
let import = scene.instantiate(&scene_asset)?;
let camera = scene.add_default_camera()?;
scene.frame_import(camera, &import)?;

let controls = OrbitControls::new(Vec3::ZERO, ${orbit.distance.toFixed(2)})
    .with_damping(0.12);`;
}

function bufferDimensions() {
  const rect = canvas.getBoundingClientRect();
  return {
    width: Math.max(1, Math.floor((rect.width || window.innerWidth) * RESOLUTION_SCALE)),
    height: Math.max(1, Math.floor((rect.height || window.innerHeight) * RESOLUTION_SCALE)),
  };
}

function applyBufferSize() {
  const { width, height } = bufferDimensions();
  canvas.width = width;
  canvas.height = height;
}

async function start() {
  buildSampleButtons();
  applyBufferSize();
  updateCodePanel();
  updateMetrics();
  beginPhase("initialising WASM");
  await init();
  wireDragDrop();
  wirePointer();
  wireResize();
  wireActions();

  const params = new URLSearchParams(window.location.search);
  const sampleParam = params.get("sample");
  const sample = SAMPLES.find((entry) => entry.id === sampleParam) || SAMPLES[0];
  await loadSample(sample);
}

async function loadSample(sample) {
  activeAsset = sample;
  updateActiveButton();
  updateCodePanel();
  beginPhase(sample.code === "connector" ? "fetching connector parts" : "fetching sample");
  try {
    if (sample.code === "connector") {
      const [driveResponse, loadResponse] = await Promise.all([
        fetch(sample.drivePath),
        fetch(sample.loadPath),
      ]);
      if (!driveResponse.ok) throw new Error(`drive HTTP ${driveResponse.status}`);
      if (!loadResponse.ok) throw new Error(`load HTTP ${loadResponse.status}`);
      const driveBytes = new Uint8Array(await driveResponse.arrayBuffer());
      const loadBytes = new Uint8Array(await loadResponse.arrayBuffer());
      logDemo(`${sample.label}: fetched ${driveBytes.byteLength + loadBytes.byteLength} connector bytes`);
      await loadConnectorAndAttach(driveBytes, loadBytes, sample, driveBytes.byteLength + loadBytes.byteLength);
      return;
    }
    const response = await fetch(sample.path);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    logDemo(`${sample.label}: fetched ${bytes.byteLength} bytes`);
    await loadAndAttach(bytes, sample, bytes.byteLength);
  } catch (err) {
    console.error("sample load failed:", err);
    setError(`load: ${err}`);
  }
}

async function loadConnectorAndAttach(driveBytes, loadBytes, asset, byteLength) {
  attached = false;
  activeAsset = asset;
  orbit = { yaw: -0.48, pitch: 0.31, distance: 2.0 };
  replayActive = false;
  updateActiveButton();
  updateCodePanel();
  updateMetrics(byteLength);
  beginPhase("building mate scene");
  app = await load_connector_snap_from_bytes(driveBytes, loadBytes, canvas.width, canvas.height);
  beginPhase("creating WebGL2 renderer");
  await attach_to_canvas(app, canvas);
  attached = true;
  frameCount = 0;
  lastFrameAt = performance.now();
  beginPhase("preparing first frame");
  requestRender();
}

async function loadAndAttach(bytes, asset, byteLength) {
  attached = false;
  activeAsset = asset;
  orbit = { yaw: -0.46, pitch: 0.34, distance: 2.0 };
  replayActive = false;
  updateActiveButton();
  updateCodePanel();
  updateMetrics(byteLength);
  beginPhase("parsing glTF");
  app = await load_gltf_from_bytes(bytes, canvas.width, canvas.height);
  beginPhase("creating WebGL2 renderer");
  await attach_to_canvas(app, canvas);
  attached = true;
  frameCount = 0;
  lastFrameAt = performance.now();
  beginPhase("preparing first frame");
  requestRender();
}

function requestRender() {
  if (!attached || renderScheduled) return;
  renderScheduled = true;
  requestAnimationFrame(() => {
    renderScheduled = false;
    if (!attached) return;
    try {
      const now = performance.now();
      const isReplayTick = replayActive && activeAsset.code === "connector";
      const dtLimit = isReplayTick ? 0.12 : 0.05;
      const dtSeconds = Math.min(dtLimit, Math.max(0.001, (now - lastFrameAt) / 1000));
      lastFrameAt = now;
      tick(app, dtSeconds);
      frameCount += 1;
      const stillReplaying = isReplayTick && connector_replay_active(app);
      replayActive = stillReplaying;
      if (stillReplaying) {
        metricPhase.textContent = "replaying";
        setStatus(activeAsset.label, `replaying connector snap · frame ${frameCount}`);
        requestRender();
      } else {
        metricPhase.textContent = "rendered";
        setStatus(activeAsset.label, `frame ${frameCount}`);
      }
      updateMetrics();
    } catch (err) {
      console.error("tick failed:", err);
      setError(`render: ${err}`);
    }
  });
}

function wireActions() {
  replayButton.addEventListener("click", () => {
    if (!attached || activeAsset.code !== "connector") return;
    try {
      replay_connector_snap(app);
      frameCount = 0;
      lastFrameAt = performance.now();
      setReplayStatus();
      requestRender();
    } catch (err) {
      console.error("replay failed:", err);
      setError(`replay: ${err}`);
    }
  });

  copyButton.addEventListener("click", async () => {
    const text = codeSnippet.textContent || "";
    try {
      await navigator.clipboard.writeText(text);
      copyButton.textContent = "Copied";
      window.setTimeout(() => {
        copyButton.textContent = "Copy";
      }, 1000);
    } catch (err) {
      console.error("copy failed:", err);
    }
  });
}

function wireResize() {
  let scheduled = false;
  window.addEventListener("resize", () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      if (!app) return;
      applyBufferSize();
      try {
        resize(app, canvas.width, canvas.height);
        requestRender();
      } catch (err) {
        console.error("resize failed:", err);
        setError(`resize: ${err}`);
      }
    });
  });
}

function isExternalUri(uri) {
  return (
    typeof uri === "string" &&
    uri.length > 0 &&
    !uri.startsWith("data:") &&
    !uri.startsWith("http://") &&
    !uri.startsWith("https://") &&
    !uri.startsWith("blob:")
  );
}

function mimeForPath(filePath) {
  const lower = filePath.toLowerCase();
  if (lower.endsWith(".bin")) return "application/octet-stream";
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  if (lower.endsWith(".webp")) return "image/webp";
  if (lower.endsWith(".ktx2")) return "image/ktx2";
  return "application/octet-stream";
}

function normalizePath(filePath) {
  return decodeURIComponent(filePath).replace(/^\.?\//, "");
}

function fileLookup(files) {
  const lookup = new Map();
  for (const file of files) {
    const relative = normalizePath(file.webkitRelativePath || file.name);
    lookup.set(relative, file);
    lookup.set(pathBasename(relative), file);
  }
  return lookup;
}

function pathBasename(filePath) {
  const normalized = normalizePath(filePath);
  const slash = normalized.lastIndexOf("/");
  return slash >= 0 ? normalized.slice(slash + 1) : normalized;
}

function arrayBufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const chunk = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunk) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunk));
  }
  return btoa(binary);
}

async function embedUri(uri, lookup, missing) {
  if (!isExternalUri(uri)) return uri;
  const key = normalizePath(uri);
  const file = lookup.get(key) || lookup.get(pathBasename(key));
  if (!file) {
    missing.add(uri);
    return uri;
  }
  const base64 = arrayBufferToBase64(await file.arrayBuffer());
  return `data:${mimeForPath(file.name)};base64,${base64}`;
}

async function bundleLocalGltf(gltfFile, files) {
  const lookup = fileLookup(files);
  const json = JSON.parse(await gltfFile.text());
  const missing = new Set();

  for (const buffer of json.buffers || []) {
    if (buffer.uri) buffer.uri = await embedUri(buffer.uri, lookup, missing);
  }
  for (const image of json.images || []) {
    if (image.uri) image.uri = await embedUri(image.uri, lookup, missing);
  }

  if (missing.size > 0) {
    throw new Error(`missing referenced file(s): ${Array.from(missing).join(", ")}`);
  }
  return new TextEncoder().encode(JSON.stringify(json));
}

async function readDroppedAsset(files) {
  const list = Array.from(files);
  const glb = list.find((file) => file.name.toLowerCase().endsWith(".glb"));
  if (glb) {
    return {
      bytes: new Uint8Array(await glb.arrayBuffer()),
      asset: {
        id: "drop",
        label: glb.name,
        detail: "dropped GLB",
        path: glb.name,
        tone: "teal",
        code: "asset",
      },
      byteLength: glb.size,
    };
  }
  const gltf = list.find((file) => file.name.toLowerCase().endsWith(".gltf"));
  if (!gltf) throw new Error("drop a GLB or a glTF bundle");
  const bytes = await bundleLocalGltf(gltf, list);
  return {
    bytes,
    asset: {
      id: "drop",
      label: gltf.name,
      detail: "dropped glTF bundle",
      path: gltf.name,
      tone: "teal",
      code: "asset",
    },
    byteLength: bytes.byteLength,
  };
}

function wireDragDrop() {
  for (const evt of ["dragover", "dragenter"]) {
    window.addEventListener(evt, (e) => {
      e.preventDefault();
      dropzone.classList.add("over");
    });
  }
  for (const evt of ["dragleave", "drop"]) {
    window.addEventListener(evt, () => dropzone.classList.remove("over"));
  }
  window.addEventListener("drop", async (e) => {
    e.preventDefault();
    try {
      beginPhase("reading dropped asset");
      const dropped = await readDroppedAsset(e.dataTransfer?.files || []);
      await loadAndAttach(dropped.bytes, dropped.asset, dropped.byteLength);
      updateActiveButton();
    } catch (err) {
      console.error(err);
      setError(`drop: ${err}`);
    }
  });
}

function safeForwardPointer(kind, x, y, dx, dy) {
  if (!attached) return;
  try {
    forward_pointer_event(app, kind, x, y, dx, dy);
  } catch (err) {
    console.error("forward_pointer_event failed:", err);
    setError(`pointer ${kind}: ${err}`);
  }
}

function updateOrbitFromPointer(kind, deltaX, deltaY) {
  if (kind === "down") {
    pointerDown = true;
  } else if (kind === "up") {
    pointerDown = false;
  } else if (kind === "move" && pointerDown) {
    orbit.yaw += deltaX * ORBIT_RADIANS_PER_PIXEL;
    orbit.pitch = Math.max(
      -MAX_PITCH_RADIANS,
      Math.min(MAX_PITCH_RADIANS, orbit.pitch + deltaY * ORBIT_RADIANS_PER_PIXEL),
    );
  } else if (kind === "wheel") {
    const zoom = Math.max(0.05, 1.0 + deltaY * ZOOM_SCALE);
    orbit.distance = Math.max(MIN_DISTANCE, orbit.distance * zoom);
  }
  updateCodePanel();
  updateMetrics();
}

function wirePointer() {
  const scaled = (e) => {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: (e.clientY - rect.top) * sy,
      sx,
      sy,
    };
  };
  canvas.addEventListener("pointerdown", (e) => {
    if (!attached) return;
    canvas.setPointerCapture(e.pointerId);
    const p = scaled(e);
    safeForwardPointer("down", p.x, p.y, 0, 0);
    updateOrbitFromPointer("down", 0, 0);
    requestRender();
  });
  canvas.addEventListener("pointerup", (e) => {
    if (!attached) return;
    canvas.releasePointerCapture(e.pointerId);
    const p = scaled(e);
    safeForwardPointer("up", p.x, p.y, 0, 0);
    updateOrbitFromPointer("up", 0, 0);
    requestRender();
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!attached) return;
    const p = scaled(e);
    const dx = e.movementX * p.sx;
    const dy = e.movementY * p.sy;
    safeForwardPointer("move", p.x, p.y, dx, dy);
    updateOrbitFromPointer("move", dx, dy);
    requestRender();
  });
  canvas.addEventListener(
    "wheel",
    (e) => {
      if (!attached) return;
      e.preventDefault();
      const p = scaled(e);
      // OrbitControls expects ~unit-scale wheel deltas; browser deltaY is
      // ~100px per notch which would multiply distance ~11× per click.
      const lineDelta =
        e.deltaMode === 1 ? e.deltaY : e.deltaMode === 2 ? e.deltaY * 10 : e.deltaY / 100;
      const normalized = Math.max(-2.0, Math.min(2.0, lineDelta));
      safeForwardPointer("wheel", p.x, p.y, 0, normalized);
      updateOrbitFromPointer("wheel", 0, normalized);
      requestRender();
    },
    { passive: false },
  );
}

start().catch((err) => {
  console.error("start failed:", err);
  setError(`init failed: ${err}`);
});

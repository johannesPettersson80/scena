#!/usr/bin/env node

import crypto from "crypto";
import fs from "fs";
import http from "http";
import os from "os";
import path from "path";
import { createRequire } from "module";
import { fileURLToPath } from "url";
import { chromium } from "playwright";

const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repo = path.resolve(__dirname, "..");
const outDir = path.join(repo, "tests", "visual", "references", "round_e");
const fixturePath = path.join(repo, "tests", "visual", "references", "round_e_material_fixture.toml");
const thresholdsPath = path.join(repo, "tests", "visual", "references", "round_e_material_thresholds.toml");
const modelViewerPath = require.resolve("@google/model-viewer/dist/model-viewer.min.js");
const modelViewerPackage = require("@google/model-viewer/package.json");

const mode = process.argv.includes("--check") ? "check" : "write";
const referenceCommand = "node scripts/generate_round_e_model_viewer_references.mjs --write";
const referenceCameraOrbit = "-18deg 72deg 5.8m";
const referenceCameraFixture =
  "reference-orbit:-18deg,72deg,5.8m; browser-crops:4x3; matches scena azimuth_elevation(-18deg,18deg)";
const referenceBackground = "#93969c";
const referenceViewport = { width: 512, height: 512 };
const proofViewport = { width: 960, height: 960 };

const demoHdrPath = "demo/samples/environment/white_studio_03_1k.hdr";
const demoHdrSha256 = "ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b";
const SCENA_BLUE = [0.00303527, 0.23074006, 1.0, 1.0];
const SCENA_GRAY = [0.2158605, 0.2158605, 0.2158605, 1.0];
const SCENA_LIGHT_GRAY = [0.69387174, 0.7230551, 0.76815116, 1.0];
const SCENA_CYAN = [0.031896032, 0.41788507, 0.7912979, 1.0];
const SCENA_COOL_WHITE = [0.8549926, 0.92158186, 1.0, 1.0];
const SCENA_WHITE = [1.0, 1.0, 1.0, 1.0];
const SCENA_LEATHER_BASE = [0.27049779, 0.08437621, 0.02955683, 1.0];
const glassBackgroundBars = [
  { offset: [0.0, 0.0, -0.006], scale: [0.56, 0.38, 0.006], color: [1, 1, 1, 1] },
  { offset: [0.0, -0.13, 0.006], scale: [0.50, 0.040, 0.010], color: [0, 0, 0, 1] },
  { offset: [0.0, -0.06, 0.006], scale: [0.50, 0.040, 0.010], color: [0, 0, 0, 1] },
  { offset: [0.0, 0.01, 0.006], scale: [0.50, 0.040, 0.010], color: [0, 0, 0, 1] },
  { offset: [0.0, 0.08, 0.006], scale: [0.50, 0.040, 0.010], color: [0, 0, 0, 1] },
  { offset: [0.0, 0.15, 0.006], scale: [0.50, 0.040, 0.010], color: [0, 0, 0, 1] },
  { offset: [-0.19, 0.0, 0.006], scale: [0.040, 0.34, 0.010], color: [0, 0, 0, 1] },
  { offset: [0.0, 0.0, 0.006], scale: [0.040, 0.34, 0.010], color: [0, 0, 0, 1] },
  { offset: [0.19, 0.0, 0.006], scale: [0.040, 0.34, 0.010], color: [0, 0, 0, 1] },
];
const sourceBackedTextureAssets = {
  satin: {
    scale: 3.0,
    color: "demo/samples/materials/ambientcg/Fabric001/demo-512/Fabric001_512_Color.jpg",
    normal: "demo/samples/materials/ambientcg/Fabric001/demo-512/Fabric001_512_NormalGL.jpg",
    orm: "demo/samples/materials/ambientcg/Fabric001/demo-512/Fabric001_512_OcclusionRoughnessMetallic.png",
  },
  leather: {
    scale: 2.5,
    color: "demo/samples/materials/ambientcg/Leather001/demo-512/Leather001_512_Color.jpg",
    normal: "demo/samples/materials/ambientcg/Leather001/demo-512/Leather001_512_NormalGL.jpg",
    orm: "demo/samples/materials/ambientcg/Leather001/demo-512/Leather001_512_OcclusionRoughnessMetallic.png",
  },
  rubber: {
    scale: 3.5,
    color: "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_Color.jpg",
    normal: "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_NormalGL.jpg",
    orm: "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_OcclusionRoughnessMetallic.png",
  },
};

const presets = [
  materialPreset("matte", "Matte", "MaterialDesc", "curved-panel", "studio", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_BLUE, metallicFactor: 0.0, roughnessFactor: 0.92 },
  }),
  materialPreset("plastic", "Plastic", "MaterialDesc", "curved-panel", "studio", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_BLUE, metallicFactor: 0.0, roughnessFactor: 0.42 },
  }),
  materialPreset("metal", "Metal", "MaterialDesc", "curved-part", "ibl-only", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_LIGHT_GRAY, metallicFactor: 1.0, roughnessFactor: 0.42 },
  }),
  materialPreset("rough_metal", "Rough metal", "MaterialDesc", "curved-part", "ibl-only", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_GRAY, metallicFactor: 1.0, roughnessFactor: 0.82 },
  }),
  materialPreset("chrome", "Chrome", "MaterialDesc", "curved-part", "ibl-only", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_LIGHT_GRAY, metallicFactor: 1.0, roughnessFactor: 0.02 },
  }),
  materialPreset("brushed_steel", "Brushed steel", "MaterialDesc", "brushed-plate", "ibl-only", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_LIGHT_GRAY, metallicFactor: 1.0, roughnessFactor: 0.36 },
    extensions: {
      KHR_materials_anisotropy: {
        anisotropyStrength: 0.72,
        anisotropyRotation: 1.57079632679,
      },
    },
  }),
  materialPreset("clearcoat_plastic", "Clearcoat plastic", "MaterialDesc", "curved-panel", "ibl-only", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_BLUE, metallicFactor: 0.0, roughnessFactor: 0.32 },
    extensions: {
      KHR_materials_clearcoat: {
        clearcoatFactor: 0.9,
        clearcoatRoughnessFactor: 0.08,
      },
    },
  }),
  materialPreset("satin", "Satin", "Assets::material_presets()", "folded-sheet", "studio", {
    pbrMetallicRoughness: { baseColorFactor: [1.0, 1.0, 1.0, 1.0], metallicFactor: 0.0, roughnessFactor: 0.68 },
    roundETextureSlots: textureSlots(sourceBackedTextureAssets.satin),
    extensions: {
      KHR_materials_sheen: {
        sheenColorFactor: [1.0, 1.0, 1.0],
        sheenRoughnessFactor: 0.48,
      },
    },
  }),
  materialPreset("leather", "Leather", "Assets::material_presets()", "strap-panel", "studio", {
    pbrMetallicRoughness: { baseColorFactor: SCENA_LEATHER_BASE, metallicFactor: 0.0, roughnessFactor: 0.78 },
    roundETextureSlots: textureSlots(sourceBackedTextureAssets.leather),
    extensions: {
      KHR_materials_sheen: {
        sheenColorFactor: SCENA_LEATHER_BASE.slice(0, 3),
        sheenRoughnessFactor: 0.72,
      },
    },
  }),
  materialPreset("clear_glass", "Clear glass", "MaterialDesc", "glass-block-grid", "ibl-only", {
    alphaMode: "BLEND",
    pbrMetallicRoughness: { baseColorFactor: [...SCENA_COOL_WHITE.slice(0, 3), 0.20], metallicFactor: 0.0, roughnessFactor: 0.02 },
    extensions: {
      KHR_materials_transmission: { transmissionFactor: 1.0 },
      KHR_materials_ior: { ior: 1.45 },
      KHR_materials_volume: {
        thicknessFactor: 0.08,
        attenuationDistance: 2.0,
        attenuationColor: SCENA_COOL_WHITE.slice(0, 3),
      },
    },
  }),
  materialPreset("frosted_glass", "Frosted glass", "MaterialDesc", "glass-screen-grid", "ibl-only", {
    alphaMode: "BLEND",
    pbrMetallicRoughness: { baseColorFactor: [...SCENA_WHITE.slice(0, 3), 0.88], metallicFactor: 0.0, roughnessFactor: 1.0 },
    extensions: {
      KHR_materials_transmission: { transmissionFactor: 0.45 },
      KHR_materials_ior: { ior: 1.45 },
      KHR_materials_volume: {
        thicknessFactor: 0.12,
        attenuationDistance: 1.25,
        attenuationColor: SCENA_WHITE.slice(0, 3),
      },
    },
  }),
  materialPreset("rubber", "Rubber", "Assets::material_presets()", "gasket-foot", "studio", {
    pbrMetallicRoughness: { baseColorFactor: [1.0, 1.0, 1.0, 1.0], metallicFactor: 0.0, roughnessFactor: 0.86 },
    roundETextureSlots: textureSlots(sourceBackedTextureAssets.rubber),
  }),
];

const proofWindows = new Map([
  ["matte", [0.218, 0.302, 0.20, 0.20]],
  ["plastic", [0.411, 0.302, 0.20, 0.20]],
  ["metal", [0.599, 0.302, 0.18, 0.20]],
  ["rough_metal", [0.776, 0.302, 0.18, 0.20]],
  ["chrome", [0.218, 0.485, 0.20, 0.22]],
  ["brushed_steel", [0.412, 0.485, 0.28, 0.16]],
  ["clearcoat_plastic", [0.600, 0.485, 0.20, 0.22]],
  ["satin", [0.776, 0.485, 0.22, 0.18]],
  ["leather", [0.218, 0.657, 0.23, 0.16]],
  ["clear_glass", [0.412, 0.657, 0.20, 0.18]],
  ["frosted_glass", [0.600, 0.657, 0.20, 0.18]],
  ["rubber", [0.776, 0.657, 0.20, 0.16]],
]);

const thresholds = {
  chrome: {
    specular_dynamic_range: 2.0,
    dark_reflection_luminance_p05_max: 85.0,
    bright_reflection_luminance_p99_min: 230.0,
    reflection_edge_contrast: 0.30,
    delta_e2000_max: 4.0,
  },
  brushed_steel: {
    anisotropy_aspect_ratio_direct: 3.0,
    anisotropy_aspect_ratio_ibl: 2.0,
    delta_e2000_max: 4.0,
  },
  clearcoat_plastic: {
    clearcoat_lobe_delta: 0.05,
    delta_e2000_max: 4.0,
  },
  clear_glass: {
    background_delta_e2000_max: 8.0,
    refraction_offset_min: 4.0,
    delta_e2000_max: 4.0,
  },
  frosted_glass: {
    high_frequency_contrast_reduction_min: 0.50,
    delta_e2000_max: 4.0,
  },
  leather: {
    texture_variance_min: 0.02,
    delta_e2000_max: 4.0,
  },
  rubber: {
    roughness_variance_min: 0.02,
    delta_e2000_max: 4.0,
  },
  satin: {
    sheen_width_min: 0.20,
    delta_e2000_max: 4.0,
  },
  global: {
    neighbor_delta_e2000_min: 6.0,
    reference_delta_e2000_max: 4.0,
  },
};

main().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});

async function main() {
  if (mode === "write") {
    fs.mkdirSync(outDir, { recursive: true });
    for (const file of fs.readdirSync(outDir)) {
      if (file.endsWith(".png")) fs.unlinkSync(path.join(outDir, file));
    }
    fs.writeFileSync(thresholdsPath, thresholdsToml());
    const references = await renderReferences();
    fs.writeFileSync(fixturePath, fixtureToml(references));
    console.log(`wrote ${presets.length} Round E model-viewer references`);
    return;
  }

  const missing = [fixturePath, thresholdsPath].filter((file) => !fs.existsSync(file));
  for (const preset of presets) {
    const reference = path.join(outDir, `${preset.id}.png`);
    if (!fs.existsSync(reference)) missing.push(reference);
  }
  if (missing.length > 0) {
    throw new Error(`missing Round E reference outputs:\n${missing.join("\n")}`);
  }
  console.log("Round E reference outputs exist");
}

async function renderReferences() {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "scena-round-e-model-viewer-"));
  const publicDir = path.join(temp, "public");
  fs.mkdirSync(publicDir, { recursive: true });
  const server = await startServer(publicDir);
  const browser = await chromium.launch({
    headless: true,
    chromiumSandbox: false,
    executablePath: process.env.CHROMIUM || undefined,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  try {
    const page = await browser.newPage({ viewport: proofViewport, deviceScaleFactor: 1 });
    const references = new Map();
    const gltfPath = path.join(publicDir, "round-e-showcase.gltf");
    fs.writeFileSync(gltfPath, JSON.stringify(createShowcaseGltf(), null, 2));
    const htmlPath = path.join(publicDir, "round-e-showcase.html");
    fs.writeFileSync(htmlPath, renderHtml("round-e-showcase.gltf", proofViewport));
    await page.goto(`${server.url}/round-e-showcase.html`, { waitUntil: "networkidle" });
    await page.waitForSelector("model-viewer");
    await page.waitForFunction(() => {
      const viewer = document.querySelector("model-viewer");
      return viewer && viewer.loaded;
    }, null, { timeout: 30000 });
    await page.waitForTimeout(250);
    if (process.env.SCENA_ROUND_E_REFERENCE_SHOWCASE) {
      await page.screenshot({ path: process.env.SCENA_ROUND_E_REFERENCE_SHOWCASE });
    }
    for (const preset of presets) {
      const imagePath = path.join(outDir, `${preset.id}.png`);
      await cropReferencePng(page, preset, imagePath);
      references.set(preset.id, {
        path: `tests/visual/references/round_e/${preset.id}.png`,
        sha256: sha256(fs.readFileSync(imagePath)),
      });
    }
    return references;
  } finally {
    await browser.close();
    await new Promise((resolve) => server.close(resolve));
    fs.rmSync(temp, { recursive: true, force: true });
  }
}

async function cropReferencePng(page, preset, imagePath) {
  const crop = pixelCropWindow(preset.id);
  await page.screenshot({
    path: imagePath,
    clip: crop,
  });
}

function createShowcaseGltf() {
  const builder = new GltfBuilder();
  const rootNodes = [];
  for (const preset of presets) {
    const position = presetPosition(preset.id);
    const mainMaterial = builder.addMaterial(preset.material);
    if (preset.geometry.includes("glass")) {
      rootNodes.push(builder.addGlassTargetBars([position[0], position[1], position[2] - 0.14]));
    }
    rootNodes.push(builder.addShapeNode(preset.geometry, mainMaterial, position));
  }
  return builder.finish(rootNodes);
}

function materialPreset(id, label, sourceSurface, geometry, lightingMode, material) {
  return { id, label, sourceSurface, geometry, lightingMode, material };
}

function textureSlots(asset) {
  return {
    baseColor: { uri: referenceTextureUri(asset.color), scale: asset.scale },
    normal: { uri: referenceTextureUri(asset.normal), scale: asset.scale },
    metallicRoughness: { uri: referenceTextureUri(asset.orm), scale: asset.scale },
    occlusion: { uri: referenceTextureUri(asset.orm), scale: asset.scale },
  };
}

function referenceTextureUri(repoRelativePath) {
  return `textures/${path.basename(repoRelativePath)}`;
}

function fixtureToml(references) {
  return presets
    .map((preset) => {
      const reference = references.get(preset.id);
      const cropWindow = pixelCropWindow(preset.id);
      const lanes = claimedLanes(preset.id);
      return `[[presets.${preset.id}]]
label = "${preset.label}"
source_surface = "${preset.sourceSurface}"
geometry = "${preset.geometry}"
crop_window = [${cropWindow.x}, ${cropWindow.y}, ${cropWindow.width}, ${cropWindow.height}]
camera = "${referenceCameraFixture}"
lighting_mode = "${preset.lightingMode}"
environment_hdr_path = "${demoHdrPath}"
environment_hdr_sha256 = "${demoHdrSha256}"
tonemapper = "model-viewer-neutral-reference"
output_color_space = "srgb"
exposure_ev = 0.0
reference_renderer = "@google/model-viewer"
reference_renderer_version = "${modelViewerPackage.version}"
reference_command = "${referenceCommand}"
reference_background = "${referenceBackground}"
reference_path = "${reference.path}"
reference_sha256 = "${reference.sha256}"
claimed_lanes = [${lanes.map((lane) => `"${lane}"`).join(", ")}]
`;
    })
    .join("\n");
}

function pixelCropWindow(presetId) {
  const [cx, cy, w, h] = proofWindows.get(presetId);
  return {
    x: Math.floor(proofViewport.width * (cx - w / 2)),
    y: Math.floor(proofViewport.height * (cy - h / 2)),
    width: Math.ceil(proofViewport.width * w),
    height: Math.ceil(proofViewport.height * h),
  };
}

function thresholdsToml() {
  return Object.entries(thresholds)
    .map(([section, values]) => {
      const body = Object.entries(values)
        .map(([key, value]) => `${key} = ${value.toFixed(2)}`)
        .join("\n");
      return `[${section}]\n${body}\n`;
    })
    .join("\n");
}

function claimedLanes(id) {
  const base = ["cpu-reference", "webgl2-desktop-chromium", "webgpu-desktop-chromium", "native-headless-gpu"];
  if (["chrome", "brushed_steel", "clearcoat_plastic", "clear_glass"].includes(id)) {
    return [...base, "ios-safari", "android-chrome"];
  }
  return base;
}

function renderHtml(source, viewport = referenceViewport) {
  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <script type="module" src="/model-viewer.min.js"></script>
  <style>
    html, body { margin: 0; width: ${viewport.width}px; height: ${viewport.height}px; background: ${referenceBackground}; }
    model-viewer { width: ${viewport.width}px; height: ${viewport.height}px; background: ${referenceBackground}; }
  </style>
</head>
<body>
  <model-viewer
    src="/${source}"
    environment-image="/white_studio_03_1k.hdr"
    exposure="1"
    camera-orbit="${referenceCameraOrbit}"
    camera-target="0m 0m 0m"
    min-camera-orbit="auto auto 0m"
    max-camera-orbit="auto auto 100m"
    field-of-view="46deg"
    shadow-intensity="0"
    interaction-prompt="none"
    disable-zoom
    disable-pan
    disable-tap>
  </model-viewer>
</body>
</html>`;
}

function presetPosition(id) {
  const index = presets.findIndex((preset) => preset.id === id);
  const column = index % 4;
  const row = Math.floor(index / 4);
  return [-0.9 + column * 0.6, 0.58 - row * 0.56, 0.0];
}

class GltfBuilder {
  constructor() {
    this.buffers = [];
    this.bufferViews = [];
    this.accessors = [];
    this.meshes = [];
    this.nodes = [];
    this.materials = [];
    this.images = [];
    this.textures = [];
    this.textureIndexByUri = new Map();
    this.extensionsUsed = new Set();
  }

  addMaterial(material) {
    const prepared = JSON.parse(JSON.stringify(material));
    const roundETextureSlots = prepared.roundETextureSlots || null;
    delete prepared.roundETextureSlots;
    if (roundETextureSlots) {
      prepared.pbrMetallicRoughness = prepared.pbrMetallicRoughness || {};
      prepared.pbrMetallicRoughness.baseColorTexture = this.addTextureInfo(roundETextureSlots.baseColor);
      prepared.pbrMetallicRoughness.metallicRoughnessTexture = this.addTextureInfo(
        roundETextureSlots.metallicRoughness,
      );
      prepared.normalTexture = this.addTextureInfo(roundETextureSlots.normal);
      prepared.occlusionTexture = this.addTextureInfo(roundETextureSlots.occlusion);
    }
    if (prepared.extensions) {
      for (const extension of Object.keys(prepared.extensions)) {
        this.extensionsUsed.add(extension);
      }
    }
    const index = this.materials.length;
    this.materials.push(prepared);
    return index;
  }

  addTextureInfo(spec) {
    const index = this.addTexture(spec.uri);
    const info = { index, texCoord: 0 };
    if (spec.scale && spec.scale !== 1.0) {
      info.extensions = {
        KHR_texture_transform: {
          scale: [spec.scale, spec.scale],
        },
      };
      this.extensionsUsed.add("KHR_texture_transform");
    }
    return info;
  }

  addTexture(uri) {
    const existing = this.textureIndexByUri.get(uri);
    if (existing !== undefined) {
      return existing;
    }
    const source = this.images.length;
    this.images.push({ uri });
    const texture = this.textures.length;
    this.textures.push({ source });
    this.textureIndexByUri.set(uri, texture);
    return texture;
  }

  addGridNode(origin = [0, 0, 0]) {
    const material = this.addMaterial({
      pbrMetallicRoughness: { baseColorFactor: [1, 1, 1, 1], metallicFactor: 0, roughnessFactor: 1 },
      extensions: { KHR_materials_unlit: {} },
    });
    this.extensionsUsed.add("KHR_materials_unlit");
    const parent = this.nodes.length;
    this.nodes.push({ name: "background-grid", children: [] });
    for (let i = 0; i <= 6; i += 1) {
      const offset = -0.6 + i * 0.2;
      const vertical = this.addBox(
        "background-grid-vertical",
        material,
        [origin[0] + offset, origin[1], origin[2]],
        [0.01, 1.1, 0.01],
      );
      const horizontal = this.addBox(
        "background-grid-horizontal",
        material,
        [origin[0], origin[1] + offset, origin[2]],
        [1.3, 0.01, 0.01],
      );
      this.nodes[parent].children.push(vertical, horizontal);
    }
    return parent;
  }

  addGlassTargetBars(origin = [0, 0, 0]) {
    const parent = this.nodes.length;
    this.nodes.push({ name: "glass-background-target", children: [] });
    for (const [index, bar] of glassBackgroundBars.entries()) {
      const material = this.addMaterial({
        pbrMetallicRoughness: { baseColorFactor: bar.color, metallicFactor: 0, roughnessFactor: 1 },
        extensions: { KHR_materials_unlit: {} },
      });
      this.extensionsUsed.add("KHR_materials_unlit");
      const translation = [
        origin[0] + bar.offset[0],
        origin[1] + bar.offset[1],
        origin[2] + bar.offset[2],
      ];
      this.nodes[parent].children.push(
        this.addBox(`glass-background-target-${index}`, material, translation, bar.scale),
      );
    }
    return parent;
  }

  addShapeNode(geometry, material, translation = [0, 0, 0]) {
    switch (geometry) {
      case "curved-panel":
        return this.addSphere(geometry, material, translation, [0.20, 0.15, 0.12]);
      case "curved-part":
        return this.addSphere(geometry, material, translation, [0.18, 0.18, 0.18]);
      case "brushed-plate":
        return this.addBox(geometry, material, translation, [0.34, 0.08, 0.025], -0.12);
      case "folded-sheet":
        return this.addBox(geometry, material, translation, [0.30, 0.13, 0.035], 0.16);
      case "strap-panel":
        return this.addBox(geometry, material, translation, [0.32, 0.08, 0.035], -0.08);
      case "glass-block-grid":
        return this.addBox(geometry, material, translation, [0.22, 0.15, 0.09]);
      case "glass-screen-grid":
        return this.addBox(geometry, material, translation, [0.28, 0.16, 0.025]);
      case "gasket-foot":
        return this.addCylinder(geometry, material, translation, [0.12, 0.06, 0.12]);
      default:
        return this.addSphere("curved-panel", material, translation, [0.20, 0.15, 0.12]);
    }
  }

  addBox(name, material, translation, scale, rotationZ = 0) {
    const geometry = boxGeometry(scale);
    return this.addMeshNode(name, material, translation, geometry, rotationZ);
  }

  addSphere(name, material, translation, scale) {
    return this.addMeshNode(name, material, translation, sphereGeometry(scale, 64, 32));
  }

  addCylinder(name, material, translation, scale) {
    return this.addMeshNode(name, material, translation, cylinderGeometry(scale, 64));
  }

  addMeshNode(name, material, translation, geometry, rotationZ = 0) {
    const { positions, normals, uvs, indices } = geometry;
    const positionAccessor = this.addAccessor(positions, "VEC3", 5126);
    const normalAccessor = this.addAccessor(normals, "VEC3", 5126);
    const uvAccessor = this.addAccessor(uvs, "VEC2", 5126);
    const indexAccessor = this.addAccessor(indices, "SCALAR", 5123);
    const meshIndex = this.meshes.length;
    this.meshes.push({
      name,
      primitives: [{
        attributes: { POSITION: positionAccessor, NORMAL: normalAccessor, TEXCOORD_0: uvAccessor },
        indices: indexAccessor,
        material,
      }],
    });
    const nodeIndex = this.nodes.length;
    const node = { mesh: meshIndex, translation };
    if (rotationZ !== 0) {
      node.rotation = [0, 0, Math.sin(rotationZ / 2), Math.cos(rotationZ / 2)];
    }
    this.nodes.push(node);
    return nodeIndex;
  }

  addAccessor(values, type, componentType) {
    const bytes = componentType === 5123
      ? Buffer.from(new Uint16Array(values).buffer)
      : Buffer.from(new Float32Array(values).buffer);
    const aligned = align4(bytes);
    const byteOffset = this.buffers.reduce((sum, buffer) => sum + buffer.length, 0);
    this.buffers.push(aligned);
    const bufferView = this.bufferViews.length;
    this.bufferViews.push({
      buffer: 0,
      byteOffset,
      byteLength: bytes.length,
      target: componentType === 5123 ? 34963 : 34962,
    });
    const accessor = this.accessors.length;
    const componentCount = { SCALAR: 1, VEC2: 2, VEC3: 3 }[type];
    this.accessors.push({
      bufferView,
      byteOffset: 0,
      componentType,
      count: values.length / componentCount,
      type,
    });
    return accessor;
  }

  finish(nodes) {
    const body = Buffer.concat(this.buffers);
    const gltf = {
      asset: { version: "2.0", generator: "round-e-model-viewer-reference" },
      scene: 0,
      scenes: [{ nodes }],
      nodes: this.nodes,
      meshes: this.meshes,
      materials: this.materials,
      accessors: this.accessors,
      bufferViews: this.bufferViews,
      buffers: [{ uri: `data:application/octet-stream;base64,${body.toString("base64")}`, byteLength: body.length }],
      extensionsUsed: Array.from(this.extensionsUsed).sort(),
    };
    if (this.images.length > 0) {
      gltf.images = this.images;
      gltf.textures = this.textures;
    }
    return gltf;
  }
}

function boxGeometry([sx, sy, sz]) {
  const x = sx / 2;
  const y = sy / 2;
  const z = sz / 2;
  const faces = [
    [[[-x, -y, z], [x, -y, z], [x, y, z], [-x, y, z]], [0, 0, 1]],
    [[[x, -y, -z], [-x, -y, -z], [-x, y, -z], [x, y, -z]], [0, 0, -1]],
    [[[-x, y, z], [x, y, z], [x, y, -z], [-x, y, -z]], [0, 1, 0]],
    [[[-x, -y, -z], [x, -y, -z], [x, -y, z], [-x, -y, z]], [0, -1, 0]],
    [[[x, -y, z], [x, -y, -z], [x, y, -z], [x, y, z]], [1, 0, 0]],
    [[[-x, -y, -z], [-x, -y, z], [-x, y, z], [-x, y, -z]], [-1, 0, 0]],
  ];
  const positions = [];
  const normals = [];
  const uvs = [];
  const indices = [];
  for (const [vertices, normal] of faces) {
    const start = positions.length / 3;
    for (const vertex of vertices) {
      positions.push(...vertex);
      normals.push(...normal);
    }
    uvs.push(0, 0, 1, 0, 1, 1, 0, 1);
    indices.push(start, start + 1, start + 2, start, start + 2, start + 3);
  }
  return { positions, normals, uvs, indices };
}

function sphereGeometry([sx, sy, sz], segments, rings) {
  const positions = [];
  const normals = [];
  const uvs = [];
  const indices = [];
  for (let ring = 0; ring <= rings; ring += 1) {
    const v = ring / rings;
    const theta = v * Math.PI;
    const sinTheta = Math.sin(theta);
    const cosTheta = Math.cos(theta);
    for (let segment = 0; segment <= segments; segment += 1) {
      const u = segment / segments;
      const phi = u * Math.PI * 2;
      const nx = Math.cos(phi) * sinTheta;
      const ny = cosTheta;
      const nz = Math.sin(phi) * sinTheta;
      positions.push(nx * sx, ny * sy, nz * sz);
      normals.push(nx, ny, nz);
      uvs.push(u, 1 - v);
    }
  }
  for (let ring = 0; ring < rings; ring += 1) {
    for (let segment = 0; segment < segments; segment += 1) {
      const a = ring * (segments + 1) + segment;
      const b = a + segments + 1;
      indices.push(a, b, a + 1, b, b + 1, a + 1);
    }
  }
  return { positions, normals, uvs, indices };
}

function cylinderGeometry([sx, sy, sz], segments) {
  const positions = [];
  const normals = [];
  const uvs = [];
  const indices = [];
  const halfY = sy / 2;
  for (let segment = 0; segment <= segments; segment += 1) {
    const u = segment / segments;
    const angle = u * Math.PI * 2;
    const nx = Math.cos(angle);
    const nz = Math.sin(angle);
    positions.push(nx * sx, -halfY, nz * sz, nx * sx, halfY, nz * sz);
    normals.push(nx, 0, nz, nx, 0, nz);
    uvs.push(u, 0, u, 1);
  }
  for (let segment = 0; segment < segments; segment += 1) {
    const a = segment * 2;
    indices.push(a, a + 1, a + 2, a + 1, a + 3, a + 2);
  }

  const bottomCenter = positions.length / 3;
  positions.push(0, -halfY, 0);
  normals.push(0, -1, 0);
  uvs.push(0.5, 0.5);
  const topCenter = bottomCenter + 1;
  positions.push(0, halfY, 0);
  normals.push(0, 1, 0);
  uvs.push(0.5, 0.5);
  for (let segment = 0; segment <= segments; segment += 1) {
    const u = segment / segments;
    const angle = u * Math.PI * 2;
    const nx = Math.cos(angle);
    const nz = Math.sin(angle);
    positions.push(nx * sx, -halfY, nz * sz, nx * sx, halfY, nz * sz);
    normals.push(0, -1, 0, 0, 1, 0);
    uvs.push((nx + 1) / 2, (nz + 1) / 2, (nx + 1) / 2, (nz + 1) / 2);
  }
  const capStart = topCenter + 1;
  for (let segment = 0; segment < segments; segment += 1) {
    const a = capStart + segment * 2;
    const b = a + 2;
    indices.push(bottomCenter, b, a);
    indices.push(topCenter, a + 1, b + 1);
  }
  return { positions, normals, uvs, indices };
}

function align4(buffer) {
  const padding = (4 - (buffer.length % 4)) % 4;
  return padding === 0 ? buffer : Buffer.concat([buffer, Buffer.alloc(padding)]);
}

async function startServer(publicDir) {
  fs.copyFileSync(modelViewerPath, path.join(publicDir, "model-viewer.min.js"));
  fs.copyFileSync(path.join(repo, demoHdrPath), path.join(publicDir, "white_studio_03_1k.hdr"));
  copyReferenceTextures(publicDir);
  const server = http.createServer((request, response) => {
    const url = new URL(request.url, "http://127.0.0.1");
    const safePath = path.normalize(url.pathname).replace(/^(\.\.[/\\])+/, "");
    const filePath = path.join(publicDir, safePath === "/" ? "index.html" : safePath);
    if (!filePath.startsWith(publicDir) || !fs.existsSync(filePath)) {
      response.writeHead(404);
      response.end("not found");
      return;
    }
    response.writeHead(200, { "content-type": contentType(filePath) });
    fs.createReadStream(filePath).pipe(response);
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  server.url = `http://127.0.0.1:${port}`;
  return server;
}

function copyReferenceTextures(publicDir) {
  const textureDir = path.join(publicDir, "textures");
  fs.mkdirSync(textureDir, { recursive: true });
  const copied = new Set();
  for (const asset of Object.values(sourceBackedTextureAssets)) {
    for (const repoRelativePath of [asset.color, asset.normal, asset.orm]) {
      if (copied.has(repoRelativePath)) continue;
      copied.add(repoRelativePath);
      fs.copyFileSync(
        path.join(repo, repoRelativePath),
        path.join(textureDir, path.basename(repoRelativePath)),
      );
    }
  }
}

function contentType(filePath) {
  if (filePath.endsWith(".html")) return "text/html";
  if (filePath.endsWith(".js")) return "text/javascript";
  if (filePath.endsWith(".gltf")) return "model/gltf+json";
  if (filePath.endsWith(".hdr")) return "application/octet-stream";
  if (filePath.endsWith(".png")) return "image/png";
  if (filePath.endsWith(".jpg") || filePath.endsWith(".jpeg")) return "image/jpeg";
  return "application/octet-stream";
}

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

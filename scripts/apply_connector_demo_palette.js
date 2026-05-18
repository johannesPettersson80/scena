#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const REPO = path.resolve(__dirname, "..");
const CONNECTOR_ASSETS = [
  "demo/samples/connector-snap/drive_unit.glb",
  "demo/samples/connector-snap/load_unit.glb",
  "demo/samples/connector-snap/connector_snap_assembly.glb",
];

const PALETTE = {
  "baseplate steel": {
    color: [0.020, 0.026, 0.028, 1.0],
    metallic: 0.25,
    roughness: 0.74,
  },
  "brushed steel": {
    color: [0.075, 0.082, 0.084, 1.0],
    metallic: 0.65,
    roughness: 0.48,
  },
  "machined aluminium": {
    color: [0.070, 0.085, 0.083, 1.0],
    metallic: 0.28,
    roughness: 0.66,
  },
  "navy powder coat": {
    color: [0.012, 0.070, 0.170, 1.0],
    metallic: 0.0,
    roughness: 0.55,
  },
  "stainless bolt": {
    color: [0.48, 0.285, 0.055, 1.0],
    metallic: 0.78,
    roughness: 0.42,
  },
  "cast iron": {
    color: [0.010, 0.011, 0.012, 1.0],
    metallic: 0.12,
    roughness: 0.76,
  },
  "recessed bore": {
    color: [0.003, 0.003, 0.004, 1.0],
    metallic: 0.0,
    roughness: 0.86,
  },
  "isolator pad": {
    color: [0.010, 0.012, 0.014, 1.0],
    metallic: 0.0,
    roughness: 0.92,
  },
  "data plate": {
    color: [0.38, 0.215, 0.040, 1.0],
    metallic: 0.65,
    roughness: 0.46,
  },
};

const ROLE_MATERIALS = {
  "ribbed black rubber": {
    color: [0.005, 0.006, 0.007, 1.0],
    metallic: 0.0,
    roughness: 0.94,
  },
  "turned steel shaft": {
    color: [0.120, 0.128, 0.125, 1.0],
    metallic: 0.70,
    roughness: 0.44,
  },
  "satin gearbox aluminium": {
    color: [0.075, 0.090, 0.085, 1.0],
    metallic: 0.25,
    roughness: 0.68,
  },
  "dark flywheel disc": {
    color: [0.045, 0.048, 0.050, 1.0],
    metallic: 0.45,
    roughness: 0.62,
  },
  "brass fastener": {
    color: [0.56, 0.34, 0.070, 1.0],
    metallic: 0.85,
    roughness: 0.38,
  },
};

const DRIVE_BELLOW_MESHES = new Set([
  "Cylinder.004",
  "Cylinder.005",
  "Cylinder.006",
  "Cylinder.007",
  "Cylinder.008",
  "Cylinder.009",
  "Cylinder.010",
  "Cylinder.011",
  "Cylinder.012",
  "Cylinder.013",
]);

const ASSEMBLY_BELLOW_MESHES = new Set([
  "Cylinder.103",
  "Cylinder.104",
  "Cylinder.105",
  "Cylinder.106",
  "Cylinder.107",
  "Cylinder.108",
  "Cylinder.109",
  "Cylinder.110",
  "Cylinder.111",
  "Cylinder.112",
]);

const DRIVE_SHAFT_MESHES = new Set(["Cylinder.032", "Cylinder.033", "Cylinder.034"]);
const ASSEMBLY_SHAFT_MESHES = new Set(["Cylinder.131", "Cylinder.132", "Cylinder.133"]);
const DRIVE_GEARBOX_MESHES = new Set(["Cube.007", "Cube.008", "Cylinder.014", "Cylinder.025"]);
const ASSEMBLY_GEARBOX_MESHES = new Set([
  "Cube.021",
  "Cube.022",
  "Cylinder.113",
  "Cylinder.124",
]);
const LOAD_FLYWHEEL_MESHES = new Set([
  "Cylinder.052",
  "Cylinder.053",
  "Cylinder.054",
]);
const ASSEMBLY_FLYWHEEL_MESHES = new Set([
  "Cylinder.084",
  "Cylinder.085",
  "Cylinder.086",
]);

function align4(value) {
  return (value + 3) & ~3;
}

function pad(buffer, byte = 0) {
  const aligned = align4(buffer.length);
  if (aligned === buffer.length) return buffer;
  return Buffer.concat([buffer, Buffer.alloc(aligned - buffer.length, byte)]);
}

function parseGlb(filePath) {
  const glb = fs.readFileSync(filePath);
  if (glb.readUInt32LE(0) !== 0x46546c67 || glb.readUInt32LE(4) !== 2) {
    throw new Error(`${filePath} is not a glTF 2.0 binary asset`);
  }

  let offset = 12;
  let json = null;
  let bin = Buffer.alloc(0);
  while (offset < glb.length) {
    const chunkLength = glb.readUInt32LE(offset);
    const chunkType = glb.readUInt32LE(offset + 4);
    const chunk = glb.subarray(offset + 8, offset + 8 + chunkLength);
    if (chunkType === 0x4e4f534a) {
      json = JSON.parse(chunk.toString("utf8").trim());
    } else if (chunkType === 0x004e4942) {
      bin = Buffer.from(chunk);
    }
    offset += 8 + chunkLength;
  }

  if (!json) {
    throw new Error(`${filePath} has no JSON chunk`);
  }
  return { json, bin };
}

function normalizedMaterialName(material) {
  return (material.name || "").split(".")[0];
}

function stripTextureReferences(material) {
  material.pbrMetallicRoughness = material.pbrMetallicRoughness || {};
  delete material.pbrMetallicRoughness.baseColorTexture;
  delete material.pbrMetallicRoughness.metallicRoughnessTexture;
  delete material.normalTexture;
  delete material.occlusionTexture;
  delete material.emissiveTexture;
}

function materialFromPalette(name, palette) {
  return {
    name,
    pbrMetallicRoughness: {
      baseColorFactor: palette.color,
      metallicFactor: palette.metallic,
      roughnessFactor: palette.roughness,
    },
  };
}

function materialIndex(json, name) {
  let index = json.materials.findIndex((material) => material.name === name);
  if (index >= 0) {
    return index;
  }
  index = json.materials.length;
  json.materials.push(materialFromPalette(name, ROLE_MATERIALS[name]));
  return index;
}

function roleForMesh(asset, meshName) {
  if (asset.endsWith("drive_unit.glb")) {
    if (DRIVE_BELLOW_MESHES.has(meshName)) return "ribbed black rubber";
    if (DRIVE_SHAFT_MESHES.has(meshName)) return "turned steel shaft";
    if (DRIVE_GEARBOX_MESHES.has(meshName)) return "satin gearbox aluminium";
  }
  if (asset.endsWith("load_unit.glb")) {
    if (LOAD_FLYWHEEL_MESHES.has(meshName)) return "dark flywheel disc";
  }
  if (asset.endsWith("connector_snap_assembly.glb")) {
    if (ASSEMBLY_BELLOW_MESHES.has(meshName)) return "ribbed black rubber";
    if (ASSEMBLY_SHAFT_MESHES.has(meshName)) return "turned steel shaft";
    if (ASSEMBLY_GEARBOX_MESHES.has(meshName)) return "satin gearbox aluminium";
    if (ASSEMBLY_FLYWHEEL_MESHES.has(meshName)) return "dark flywheel disc";
  }
  return null;
}

function applyPalette(json) {
  delete json.images;
  delete json.textures;
  delete json.samplers;
  if (Array.isArray(json.extensionsUsed)) {
    json.extensionsUsed = json.extensionsUsed.filter(
      (extension) => extension !== "KHR_texture_transform",
    );
    if (json.extensionsUsed.length === 0) {
      delete json.extensionsUsed;
    }
  }

  const missing = new Set(Object.keys(PALETTE));
  for (const material of json.materials || []) {
    const name = normalizedMaterialName(material);
    const palette = PALETTE[name];
    if (!palette) {
      continue;
    }
    missing.delete(name);
    stripTextureReferences(material);
    material.pbrMetallicRoughness.baseColorFactor = palette.color;
    material.pbrMetallicRoughness.metallicFactor = palette.metallic;
    material.pbrMetallicRoughness.roughnessFactor = palette.roughness;
  }

  return [...missing];
}

function applyMeshRoleMaterials(json, asset) {
  json.materials = json.materials || [];
  for (const mesh of json.meshes || []) {
    const role = roleForMesh(asset, mesh.name || "");
    if (!role) {
      continue;
    }
    const index = materialIndex(json, role);
    for (const primitive of mesh.primitives || []) {
      primitive.material = index;
    }
  }
}

function writeGlb(filePath, json, bin) {
  json.buffers = json.buffers || [{}];
  json.buffers[0].byteLength = bin.length;
  delete json.buffers[0].uri;

  const jsonBuffer = pad(Buffer.from(JSON.stringify(json), "utf8"), 0x20);
  const binBuffer = pad(bin);
  const totalLength = 12 + 8 + jsonBuffer.length + 8 + binBuffer.length;
  const header = Buffer.alloc(12);
  header.writeUInt32LE(0x46546c67, 0);
  header.writeUInt32LE(2, 4);
  header.writeUInt32LE(totalLength, 8);
  const jsonHeader = Buffer.alloc(8);
  jsonHeader.writeUInt32LE(jsonBuffer.length, 0);
  jsonHeader.writeUInt32LE(0x4e4f534a, 4);
  const binHeader = Buffer.alloc(8);
  binHeader.writeUInt32LE(binBuffer.length, 0);
  binHeader.writeUInt32LE(0x004e4942, 4);
  fs.writeFileSync(filePath, Buffer.concat([header, jsonHeader, jsonBuffer, binHeader, binBuffer]));
}

for (const asset of CONNECTOR_ASSETS) {
  const filePath = path.join(REPO, asset);
  const { json, bin } = parseGlb(filePath);
  const missing = applyPalette(json);
  applyMeshRoleMaterials(json, asset);
  writeGlb(filePath, json, bin);
  console.log(`applied controlled connector palette to ${asset}`);
  if (missing.length > 0) {
    console.log(`  not present in this asset: ${missing.join(", ")}`);
  }
}

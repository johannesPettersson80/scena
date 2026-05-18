#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const REPO = path.resolve(__dirname, "..");
const MATERIAL_PROFILES = [
  {
    materialName: "brushed steel",
    assetId: "Metal030",
    textureScale: [8.0, 8.0],
    baseColorFactor: [0.82, 0.86, 0.88, 1.0],
    metallicFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "baseplate steel",
    assetId: "Metal030",
    textureScale: [16.0, 16.0],
    baseColorFactor: [0.82, 0.86, 0.88, 1.0],
    metallicFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "machined aluminium",
    assetId: "Metal010",
    textureScale: [16.0, 16.0],
    baseColorFactor: [0.72, 0.76, 0.80, 1.0],
    metallicFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "navy powder coat",
    assetId: "Metal030",
    textureScale: [12.0, 12.0],
    baseColorFactor: [0.08, 0.16, 0.34, 1.0],
    metallicFactor: 0.0,
    roughnessFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "stainless bolt",
    assetId: "Metal030",
    textureScale: [8.0, 8.0],
    baseColorFactor: [0.95, 0.72, 0.26, 1.0],
    metallicFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "cast iron",
    assetId: "Metal010",
    textureScale: [12.0, 12.0],
    baseColorFactor: [0.20, 0.21, 0.22, 1.0],
    metallicFactor: 1.0,
    roughnessFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "recessed bore",
    assetId: "Metal010",
    textureScale: [8.0, 8.0],
    baseColorFactor: [0.08, 0.08, 0.09, 1.0],
    metallicFactor: 1.0,
    roughnessFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "data plate",
    assetId: "Metal030",
    textureScale: [6.0, 6.0],
    baseColorFactor: [0.92, 0.65, 0.18, 1.0],
    metallicFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "isolator pad",
    assetId: "Rubber002",
    textureScale: [10.0, 10.0],
    baseColorFactor: [0.55, 0.55, 0.55, 1.0],
    metallicFactor: 0.0,
    roughnessFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "turned steel shaft",
    assetId: "Metal030",
    textureScale: [10.0, 10.0],
    baseColorFactor: [0.78, 0.82, 0.84, 1.0],
    metallicFactor: 1.0,
    roughnessFactor: 0.72,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal030/demo-512/Metal030_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "satin gearbox aluminium",
    assetId: "Metal010",
    textureScale: [16.0, 16.0],
    baseColorFactor: [0.42, 0.45, 0.47, 1.0],
    metallicFactor: 1.0,
    roughnessFactor: 0.88,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "dark flywheel disc",
    assetId: "Metal010",
    textureScale: [12.0, 12.0],
    baseColorFactor: [0.18, 0.19, 0.20, 1.0],
    metallicFactor: 1.0,
    roughnessFactor: 0.92,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Metal010/demo-512/Metal010_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
  {
    materialName: "ribbed black rubber",
    assetId: "Rubber002",
    textureScale: [10.0, 10.0],
    baseColorFactor: [0.18, 0.18, 0.18, 1.0],
    metallicFactor: 0.0,
    roughnessFactor: 1.0,
    baseColor: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_Color.jpg"),
      mimeType: "image/jpeg",
    },
    normal: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_NormalGL.jpg"),
      mimeType: "image/jpeg",
    },
    orm: {
      path: path.join(REPO, "demo/samples/materials/ambientcg/Rubber002/demo-512/Rubber002_512_OcclusionRoughnessMetallic.png"),
      mimeType: "image/png",
    },
  },
];
const CONNECTOR_ASSETS = [
  "demo/samples/connector-snap/drive_unit.glb",
  "demo/samples/connector-snap/load_unit.glb",
  "demo/samples/connector-snap/connector_snap_assembly.glb",
];

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

function textureByImageName(json, name) {
  const imageIndex = (json.images || []).findIndex((image) => image.name === name);
  if (imageIndex < 0) return null;
  const textureIndex = (json.textures || []).findIndex((texture) => texture.source === imageIndex);
  return textureIndex >= 0 ? textureIndex : null;
}

function appendImageTexture(json, bin, name, texture, sampler) {
  const existing = textureByImageName(json, name);
  if (existing !== null) {
    return { bin, texture: existing };
  }

  const bytes = fs.readFileSync(texture.path);
  const byteOffset = align4(bin.length);
  const paddedPrefix = byteOffset === bin.length ? bin : pad(bin);
  const imageBytes = pad(bytes);
  const bufferView = json.bufferViews.length;
  json.bufferViews.push({
    buffer: 0,
    byteOffset,
    byteLength: bytes.length,
  });
  const image = json.images.length;
  json.images.push({
    name,
    bufferView,
    mimeType: texture.mimeType,
  });
  const textureIndex = json.textures.length;
  json.textures.push({ sampler, source: image });
  return { bin: Buffer.concat([paddedPrefix, imageBytes]), texture: textureIndex };
}

function ensureTextureSet(json, bin, profile) {
  json.samplers = json.samplers || [];
  json.images = json.images || [];
  json.textures = json.textures || [];

  const textureLabel = `${profile.assetId} demo 512`;
  const baseName = `ambientCG ${textureLabel} base color`;
  const normalName = `ambientCG ${textureLabel} normal GL`;
  const ormName = `ambientCG ${textureLabel} neutral-ao roughness metalness`;
  const existingBase = textureByImageName(json, baseName);
  const existingNormal = textureByImageName(json, normalName);
  const existingOrm = textureByImageName(json, ormName);
  if (existingBase !== null && existingNormal !== null && existingOrm !== null) {
    return {
      bin,
      baseColorTexture: existingBase,
      normalTexture: existingNormal,
      ormTexture: existingOrm,
    };
  }

  const sampler = json.samplers.length;
  json.samplers.push({
    magFilter: 9729,
    minFilter: 9987,
    wrapS: 10497,
    wrapT: 10497,
  });

  const base = appendImageTexture(
    json,
    bin,
    baseName,
    profile.baseColor,
    sampler,
  );
  bin = base.bin;
  const normal = appendImageTexture(
    json,
    bin,
    normalName,
    profile.normal,
    sampler,
  );
  bin = normal.bin;
  const orm = appendImageTexture(
    json,
    bin,
    ormName,
    profile.orm,
    sampler,
  );
  bin = orm.bin;

  return {
    bin,
    baseColorTexture: base.texture,
    normalTexture: normal.texture,
    ormTexture: orm.texture,
  };
}

function materialTextureInfos(material) {
  const pbr = material.pbrMetallicRoughness || {};
  return [
    pbr.baseColorTexture,
    pbr.metallicRoughnessTexture,
    material.normalTexture,
    material.occlusionTexture,
    material.emissiveTexture,
  ].filter((texture) => texture && Number.isInteger(texture.index));
}

function compactUnusedImageResources(json, bin) {
  if (!Array.isArray(json.images) || !Array.isArray(json.textures) || !Array.isArray(json.bufferViews)) {
    return { json, bin };
  }

  const usedTextureIndexes = new Set();
  for (const material of json.materials || []) {
    for (const texture of materialTextureInfos(material)) {
      usedTextureIndexes.add(texture.index);
    }
  }

  const oldTextures = json.textures;
  const oldImages = json.images;
  const usedImageIndexes = new Set();
  for (const textureIndex of usedTextureIndexes) {
    const texture = oldTextures[textureIndex];
    if (texture && Number.isInteger(texture.source)) {
      usedImageIndexes.add(texture.source);
    }
  }

  const imageIndexMap = new Map();
  const images = [];
  oldImages.forEach((image, index) => {
    if (!usedImageIndexes.has(index)) {
      return;
    }
    imageIndexMap.set(index, images.length);
    images.push({ ...image });
  });

  const textureIndexMap = new Map();
  const textures = [];
  oldTextures.forEach((texture, index) => {
    if (!usedTextureIndexes.has(index) || !imageIndexMap.has(texture.source)) {
      return;
    }
    textureIndexMap.set(index, textures.length);
    textures.push({ ...texture, source: imageIndexMap.get(texture.source) });
  });

  for (const material of json.materials || []) {
    for (const texture of materialTextureInfos(material)) {
      texture.index = textureIndexMap.get(texture.index);
    }
  }

  json.images = images;
  json.textures = textures;

  const retainedImageBufferViews = new Set(
    images
      .map((image) => image.bufferView)
      .filter((bufferView) => Number.isInteger(bufferView)),
  );
  const oldImageBufferViews = new Set(
    oldImages
      .map((image) => image.bufferView)
      .filter((bufferView) => Number.isInteger(bufferView)),
  );
  const keepBufferView = json.bufferViews.map(
    (_bufferView, index) => !oldImageBufferViews.has(index) || retainedImageBufferViews.has(index),
  );

  const bufferViewIndexMap = new Map();
  let compactBin = Buffer.alloc(0);
  const bufferViews = [];
  json.bufferViews.forEach((bufferView, index) => {
    if (!keepBufferView[index]) {
      return;
    }
    const byteOffset = bufferView.byteOffset || 0;
    const bytes = bin.subarray(byteOffset, byteOffset + bufferView.byteLength);
    compactBin = pad(compactBin);
    const compactOffset = compactBin.length;
    bufferViewIndexMap.set(index, bufferViews.length);
    bufferViews.push({
      ...bufferView,
      byteOffset: compactOffset,
      byteLength: bytes.length,
    });
    compactBin = Buffer.concat([compactBin, bytes]);
  });

  for (const accessor of json.accessors || []) {
    if (Number.isInteger(accessor.bufferView)) {
      accessor.bufferView = remapBufferView(bufferViewIndexMap, accessor.bufferView);
    }
    const sparse = accessor.sparse || {};
    if (sparse.indices && Number.isInteger(sparse.indices.bufferView)) {
      sparse.indices.bufferView = remapBufferView(bufferViewIndexMap, sparse.indices.bufferView);
    }
    if (sparse.values && Number.isInteger(sparse.values.bufferView)) {
      sparse.values.bufferView = remapBufferView(bufferViewIndexMap, sparse.values.bufferView);
    }
  }
  for (const image of json.images || []) {
    if (Number.isInteger(image.bufferView)) {
      image.bufferView = remapBufferView(bufferViewIndexMap, image.bufferView);
    }
  }

  json.bufferViews = bufferViews;
  return { json, bin: compactBin };
}

function remapBufferView(indexMap, oldIndex) {
  if (!indexMap.has(oldIndex)) {
    throw new Error(`bufferView ${oldIndex} was removed while still referenced`);
  }
  return indexMap.get(oldIndex);
}

function normalizedMaterialName(material) {
  return (material.name || "").split(".")[0];
}

function textureInfo(index, profile) {
  const info = { index, texCoord: 0 };
  if (profile.textureScale) {
    info.extensions = {
      KHR_texture_transform: {
        scale: profile.textureScale,
      },
    };
  }
  return info;
}

function materialHasTexcoords(json, materialIndex) {
  for (const mesh of json.meshes || []) {
    for (const primitive of mesh.primitives || []) {
      if (primitive.material === materialIndex && !primitive.attributes?.TEXCOORD_0) {
        return false;
      }
    }
  }
  return true;
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

function applyMaterial(filePath, profile) {
  const absolute = path.join(REPO, filePath);
  const { json, bin } = parseGlb(absolute);
  const materialIndexes = (json.materials || [])
    .map((material, index) => ({ material, index }))
    .filter(({ material }) => normalizedMaterialName(material) === profile.materialName);
  if (materialIndexes.length === 0) {
    return;
  }
  if (materialIndexes.length > 1) {
    throw new Error(`${filePath} must contain at most one ${profile.materialName} material, found ${materialIndexes.length}`);
  }
  for (const { index } of materialIndexes) {
    if (!materialHasTexcoords(json, index)) {
      throw new Error(`${filePath} ${profile.materialName} primitives must have TEXCOORD_0 before applying texture maps`);
    }
  }

  const textureSet = ensureTextureSet(json, bin, profile);
  if (profile.textureScale) {
    json.extensionsUsed = json.extensionsUsed || [];
    if (!json.extensionsUsed.includes("KHR_texture_transform")) {
      json.extensionsUsed.push("KHR_texture_transform");
    }
  }
  for (const { material } of materialIndexes) {
    material.pbrMetallicRoughness = material.pbrMetallicRoughness || {};
    material.pbrMetallicRoughness.baseColorFactor = profile.baseColorFactor || [1.0, 1.0, 1.0, 1.0];
    material.pbrMetallicRoughness.metallicFactor = profile.metallicFactor ?? 1.0;
    material.pbrMetallicRoughness.roughnessFactor = profile.roughnessFactor ?? 1.0;
    material.pbrMetallicRoughness.baseColorTexture = textureInfo(textureSet.baseColorTexture, profile);
    material.pbrMetallicRoughness.metallicRoughnessTexture = textureInfo(textureSet.ormTexture, profile);
    material.normalTexture = textureInfo(textureSet.normalTexture, profile);
    material.occlusionTexture = textureInfo(textureSet.ormTexture, profile);
  }

  const compacted = compactUnusedImageResources(json, textureSet.bin);
  writeGlb(absolute, compacted.json, compacted.bin);
  console.log(`applied ambientCG ${profile.assetId} to ${filePath} ${profile.materialName}`);
}

for (const profile of MATERIAL_PROFILES) {
  for (const texture of [profile.baseColor, profile.normal, profile.orm]) {
    if (!fs.existsSync(texture.path)) {
      throw new Error(`missing material texture ${path.relative(REPO, texture.path)}`);
    }
  }
}

for (const asset of CONNECTOR_ASSETS) {
  for (const profile of MATERIAL_PROFILES) {
    applyMaterial(asset, profile);
  }
}

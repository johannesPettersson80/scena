#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const REPO = path.resolve(__dirname, "..");
const OUT_DIRS = [
  path.join(REPO, "tests/assets/gltf"),
  path.join(REPO, "demo/samples/connector-snap"),
];

const MATERIALS = {
  steel: { color: [0.68, 0.71, 0.72, 1.0], metallic: 0.5, roughness: 0.42 },
  navy: { color: [0.035, 0.078, 0.15, 1.0], metallic: 0.0, roughness: 0.64 },
  machined: { color: [0.42, 0.45, 0.47, 1.0], metallic: 0.35, roughness: 0.38 },
  dark: { color: [0.035, 0.042, 0.048, 1.0], metallic: 0.2, roughness: 0.54 },
  anodized: { color: [0.055, 0.06, 0.068, 1.0], metallic: 0.55, roughness: 0.36 },
  accent: { color: [0.86, 0.68, 0.22, 1.0], metallic: 0.3, roughness: 0.35 },
};

function align4(value) {
  return (value + 3) & ~3;
}

function pad(buffer, byte = 0) {
  const aligned = align4(buffer.length);
  if (aligned === buffer.length) return buffer;
  return Buffer.concat([buffer, Buffer.alloc(aligned - buffer.length, byte)]);
}

function pushVec(out, x, y, z) {
  out.push(x, y, z);
}

function normalize(v) {
  const len = Math.hypot(v[0], v[1], v[2]) || 1;
  return [v[0] / len, v[1] / len, v[2] / len];
}

function cross(a, b) {
  return [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
}

function addQuad(positions, normals, indices, a, b, c, d, normal) {
  const base = positions.length / 3;
  for (const p of [a, b, c, d]) pushVec(positions, p[0], p[1], p[2]);
  for (let i = 0; i < 4; i++) pushVec(normals, normal[0], normal[1], normal[2]);
  indices.push(base, base + 1, base + 2, base, base + 2, base + 3);
}

function createBox(name, center, size, material) {
  const [cx, cy, cz] = center;
  const [sx, sy, sz] = size.map((v) => v / 2);
  const x0 = cx - sx;
  const x1 = cx + sx;
  const y0 = cy - sy;
  const y1 = cy + sy;
  const z0 = cz - sz;
  const z1 = cz + sz;
  const p = [];
  const n = [];
  const i = [];

  addQuad(p, n, i, [x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1], [1, 0, 0]);
  addQuad(p, n, i, [x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0], [-1, 0, 0]);
  addQuad(p, n, i, [x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [0, 1, 0]);
  addQuad(p, n, i, [x0, y0, z1], [x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [0, -1, 0]);
  addQuad(p, n, i, [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1], [0, 0, 1]);
  addQuad(p, n, i, [x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [0, 0, -1]);

  return { name, positions: p, normals: n, indices: i, material };
}

function createCylinderX(name, center, length, radius, material, segments = 96) {
  const [cx, cy, cz] = center;
  const x0 = cx - length / 2;
  const x1 = cx + length / 2;
  const p = [];
  const n = [];
  const i = [];

  for (let s = 0; s < segments; s++) {
    const a0 = (Math.PI * 2 * s) / segments;
    const a1 = (Math.PI * 2 * (s + 1)) / segments;
    const y0 = Math.cos(a0) * radius;
    const z0 = Math.sin(a0) * radius;
    const y1 = Math.cos(a1) * radius;
    const z1 = Math.sin(a1) * radius;
    const normal0 = normalize([0, y0, z0]);
    const normal1 = normalize([0, y1, z1]);
    const base = p.length / 3;
    pushVec(p, x0, cy + y0, cz + z0);
    pushVec(p, x1, cy + y0, cz + z0);
    pushVec(p, x1, cy + y1, cz + z1);
    pushVec(p, x0, cy + y1, cz + z1);
    pushVec(n, ...normal0);
    pushVec(n, ...normal0);
    pushVec(n, ...normal1);
    pushVec(n, ...normal1);
    i.push(base, base + 1, base + 2, base, base + 2, base + 3);
  }

  const leftCenter = p.length / 3;
  pushVec(p, x0, cy, cz);
  pushVec(n, -1, 0, 0);
  const rightCenter = leftCenter + 1;
  pushVec(p, x1, cy, cz);
  pushVec(n, 1, 0, 0);
  for (let s = 0; s < segments; s++) {
    const a0 = (Math.PI * 2 * s) / segments;
    const a1 = (Math.PI * 2 * (s + 1)) / segments;
    const y0 = Math.cos(a0) * radius;
    const z0 = Math.sin(a0) * radius;
    const y1 = Math.cos(a1) * radius;
    const z1 = Math.sin(a1) * radius;

    let base = p.length / 3;
    pushVec(p, x0, cy + y1, cz + z1);
    pushVec(p, x0, cy + y0, cz + z0);
    pushVec(n, -1, 0, 0);
    pushVec(n, -1, 0, 0);
    i.push(leftCenter, base, base + 1);

    base = p.length / 3;
    pushVec(p, x1, cy + y0, cz + z0);
    pushVec(p, x1, cy + y1, cz + z1);
    pushVec(n, 1, 0, 0);
    pushVec(n, 1, 0, 0);
    i.push(rightCenter, base, base + 1);
  }

  return { name, positions: p, normals: n, indices: i, material };
}

function boltPattern(center, radius, count) {
  const shapes = [];
  for (let index = 0; index < count; index++) {
    const angle = (Math.PI * 2 * index) / count;
    shapes.push(
      createCylinderX(
        `flywheel_bolt_${index + 1}`,
        [center[0] - 0.082, center[1] + Math.cos(angle) * radius, center[2] + Math.sin(angle) * radius],
        0.025,
        0.014,
        "accent",
        12,
      ),
    );
  }
  return shapes;
}

function driveShapes() {
  return [
    createBox("drive_baseplate", [0.0, -0.23, 0.0], [1.08, 0.08, 0.42], "dark"),
    createBox("drive_l_bracket_foot", [-0.14, -0.11, -0.17], [0.62, 0.12, 0.08], "navy"),
    createBox("drive_l_bracket_wall", [-0.14, 0.04, -0.18], [0.62, 0.36, 0.075], "navy"),
    createCylinderX("motor_steel_can", [-0.25, 0.07, 0.0], 0.52, 0.16, "steel", 128),
    createBox("motor_dark_housing", [0.05, 0.07, 0.0], [0.28, 0.34, 0.34], "navy"),
    createBox("gearbox_machined_body", [0.31, 0.07, 0.0], [0.26, 0.28, 0.30], "machined"),
    createCylinderX("drive_shaft", [0.55, 0.07, 0.0], 0.22, 0.045, "steel", 96),
  ];
}

function loadShapes() {
  const wheelCenter = [0.16, 0.07, 0.0];
  return [
    createBox("load_baseplate", [0.05, -0.24, 0.0], [0.72, 0.08, 0.46], "dark"),
    createBox("load_pedestal", [0.12, -0.085, 0.0], [0.16, 0.31, 0.18], "machined"),
    createCylinderX("load_flywheel", wheelCenter, 0.13, 0.23, "anodized", 128),
    createCylinderX("load_hub_socket", [0.045, 0.07, 0.0], 0.09, 0.085, "steel", 96),
    ...boltPattern(wheelCenter, 0.15, 8),
  ];
}

function connectorExtras(name, kind, allowedMates, polarity, translation) {
  return {
    scena: {
      connectors: [
        {
          name,
          kind,
          translation,
          forward: [1.0, 0.0, 0.0],
          up: [0.0, 1.0, 0.0],
          allowedMates,
          tags: ["assembly", "canonical-demo"],
          snapTolerance: 0.01,
          clearanceHint: 0.002,
          rollPolicy: "chooseNearest",
          polarity,
          metadata: {
            author: "scena canonical connector demo",
            generatedBy: "scripts/generate_connector_demo_assets.js",
          },
        },
      ],
    },
  };
}

function buildDoc(parts) {
  const buffer = [];
  const bufferViews = [];
  const accessors = [];
  const meshes = [];
  const nodes = [];
  const materialNames = Object.keys(MATERIALS);
  const materialIndex = new Map(materialNames.map((name, index) => [name, index]));

  function appendBuffer(data, componentType, type, count, minMax) {
    const offset = align4(buffer.length);
    while (buffer.length < offset) buffer.push(0);
    let bytes;
    if (componentType === 5126) {
      bytes = Buffer.alloc(data.length * 4);
      data.forEach((value, index) => bytes.writeFloatLE(value, index * 4));
    } else if (componentType === 5125) {
      bytes = Buffer.alloc(data.length * 4);
      data.forEach((value, index) => bytes.writeUInt32LE(value, index * 4));
    } else {
      throw new Error(`unsupported componentType ${componentType}`);
    }
    const byteOffset = buffer.length;
    for (const byte of bytes) buffer.push(byte);
    const view = bufferViews.length;
    bufferViews.push({ buffer: 0, byteOffset, byteLength: bytes.length });
    const accessor = accessors.length;
    accessors.push({
      bufferView: view,
      componentType,
      count,
      type,
      ...(minMax || {}),
    });
    return accessor;
  }

  function minMax3(values) {
    const min = [Infinity, Infinity, Infinity];
    const max = [-Infinity, -Infinity, -Infinity];
    for (let index = 0; index < values.length; index += 3) {
      for (let axis = 0; axis < 3; axis++) {
        min[axis] = Math.min(min[axis], values[index + axis]);
        max[axis] = Math.max(max[axis], values[index + axis]);
      }
    }
    return { min, max };
  }

  for (const part of parts) {
    const root = nodes.length;
    nodes.push({
      name: part.name,
      ...(part.translation ? { translation: part.translation } : {}),
      extras: part.extras,
      children: [],
    });

    for (const shape of part.shapes) {
      const position = appendBuffer(shape.positions, 5126, "VEC3", shape.positions.length / 3, minMax3(shape.positions));
      const normal = appendBuffer(shape.normals, 5126, "VEC3", shape.normals.length / 3);
      const indices = appendBuffer(shape.indices, 5125, "SCALAR", shape.indices.length, {
        min: [0],
        max: [Math.max(...shape.indices)],
      });
      const mesh = meshes.length;
      meshes.push({
        name: shape.name,
        primitives: [
          {
            attributes: { POSITION: position, NORMAL: normal },
            indices,
            material: materialIndex.get(shape.material),
          },
        ],
      });
      const node = nodes.length;
      nodes.push({ name: shape.name, mesh });
      nodes[root].children.push(node);
    }
  }

  const bin = Buffer.from(buffer);
  return {
    json: {
      asset: { version: "2.0", generator: "scena connector snap asset generator" },
      buffers: [{ byteLength: pad(bin).length }],
      bufferViews,
      accessors,
      materials: materialNames.map((name) => ({
        name,
        pbrMetallicRoughness: {
          baseColorFactor: MATERIALS[name].color,
          metallicFactor: MATERIALS[name].metallic,
          roughnessFactor: MATERIALS[name].roughness,
        },
      })),
      meshes,
      nodes,
      scenes: [{ nodes: parts.map((_, index) => parts.slice(0, index).reduce((sum, part) => sum + part.shapes.length + 1, 0)) }],
      scene: 0,
    },
    bin,
  };
}

function writeGlb(target, parts) {
  const { json, bin } = buildDoc(parts);
  const jsonBuffer = pad(Buffer.from(JSON.stringify(json), "utf8"), 0x20);
  const binBuffer = pad(bin, 0);
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
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, Buffer.concat([header, jsonHeader, jsonBuffer, binHeader, binBuffer]));
  console.log(`wrote ${path.relative(REPO, target)}`);
}

function drivePart(translation) {
  return {
    name: "drive_unit",
    translation,
    extras: connectorExtras("shaft", "shaft", ["hub"], "plug", [0.66, 0.07, 0.0]),
    shapes: driveShapes(),
  };
}

function loadPart(translation) {
  return {
    name: "load_unit",
    translation,
    extras: connectorExtras("hub", "hub", ["shaft"], "socket", [0.045, 0.07, 0.0]),
    shapes: loadShapes(),
  };
}

for (const dir of OUT_DIRS) fs.mkdirSync(dir, { recursive: true });

writeGlb(path.join(REPO, "tests/assets/gltf/drive_unit.glb"), [drivePart()]);
writeGlb(path.join(REPO, "tests/assets/gltf/load_unit.glb"), [loadPart()]);
writeGlb(path.join(REPO, "demo/samples/connector-snap/drive_unit.glb"), [drivePart()]);
writeGlb(path.join(REPO, "demo/samples/connector-snap/load_unit.glb"), [loadPart()]);
writeGlb(path.join(REPO, "demo/samples/connector-snap/connector_snap_assembly.glb"), [
  loadPart([0.0, 0.0, 0.0]),
  drivePart([-0.615, 0.0, 0.0]),
]);

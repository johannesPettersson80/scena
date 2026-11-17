#!/usr/bin/env node
// Generates demo/samples/gate-valve/gate_valve_dn100.glb: a DN100 flanged,
// rising-stem gate valve, built from its dimensions.
//
// The valve is modelled the way it is made. Each casting is one mesh of
// overlapping solids (the path tracer's edge rounding blends them into a
// single cast shape), machined faces and fasteners are meshes of their own,
// and every material states its finish in `extras.scena.surface_finish`.
// Units are metres, +Y is up and the pipe runs along X. The valve stands on
// the rims of its two end flanges, which touch the floor at y = -0.1145.
//
// Usage: node scripts/generate_gate_valve_demo_asset.js

const fs = require("fs");
const path = require("path");

const REPO = path.resolve(__dirname, "..");
const OUT = path.join(REPO, "demo/samples/gate-valve/gate_valve_dn100.glb");
const TAU = Math.PI * 2;

// ---------------------------------------------------------------- vectors

const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const length = (a) => Math.hypot(a[0], a[1], a[2]);
const normalize = (a) => {
  const l = length(a);
  return l > 0 ? scale(a, 1 / l) : [0, 1, 0];
};
const X = [1, 0, 0];
const Y = [0, 1, 0];
const Z = [0, 0, 1];

// ------------------------------------------------------------------- mesh

class Mesh {
  constructor() {
    this.positions = [];
    this.normals = [];
    this.indices = [];
  }

  vertex(position, normal) {
    this.positions.push(position);
    this.normals.push(normalize(normal));
    return this.positions.length - 1;
  }

  // Adds a triangle wound so that its face agrees with its vertex normals.
  tri(a, b, c) {
    const [pa, pb, pc] = [this.positions[a], this.positions[b], this.positions[c]];
    const face = cross(sub(pb, pa), sub(pc, pa));
    if (length(face) < 1e-14) return;
    const outward = add(add(this.normals[a], this.normals[b]), this.normals[c]);
    if (dot(face, outward) >= 0) this.indices.push(a, b, c);
    else this.indices.push(a, c, b);
  }

  quad(a, b, c, d) {
    this.tri(a, b, c);
    this.tri(a, c, d);
  }

  // A grid of rows x cols vertices; `wrap` closes the columns into a ring.
  grid(rows, cols, point, wrap) {
    const base = this.positions.length;
    for (let i = 0; i < rows; i++) {
      for (let j = 0; j < cols; j++) {
        const [position, normal] = point(i, j);
        this.vertex(position, normal);
      }
    }
    const last = wrap ? cols : cols - 1;
    for (let i = 0; i + 1 < rows; i++) {
      for (let j = 0; j < last; j++) {
        const k = (j + 1) % cols;
        this.quad(
          base + i * cols + j,
          base + i * cols + k,
          base + (i + 1) * cols + k,
          base + (i + 1) * cols + j,
        );
      }
    }
  }

  // A flat polygon fan about `centre`, facing `normal`.
  fan(centre, ring, normal) {
    const hub = this.vertex(centre, normal);
    const rim = ring.map((position) => this.vertex(position, normal));
    for (let j = 0; j < rim.length; j++) {
      this.tri(hub, rim[j], rim[(j + 1) % rim.length]);
    }
  }

  merge(other) {
    const base = this.positions.length;
    this.positions.push(...other.positions);
    this.normals.push(...other.normals);
    for (const index of other.indices) this.indices.push(base + index);
    return this;
  }
}

// A frame with `axis` and two directions across it.
function frame(axis) {
  const a = normalize(axis);
  const helper = Math.abs(a[1]) < 0.9 ? Y : X;
  const u = normalize(cross(helper, a));
  return { a, u, v: cross(a, u) };
}

// Surface of revolution about `axis` through `origin`. `profile` lists
// [radius, height] points; with the height rising the surface faces away
// from the axis, with it falling it faces the axis, so a closed loop of
// points makes a hollow part. Corners sharper than `crease` degrees stay
// sharp; gentler ones are smoothed.
function lathe(mesh, origin, axis, profile, segments = 96, crease = 32) {
  const { a, u, v } = frame(axis);
  const segmentNormals = [];
  for (let k = 0; k + 1 < profile.length; k++) {
    const dr = profile[k + 1][0] - profile[k][0];
    const dh = profile[k + 1][1] - profile[k][1];
    const l = Math.hypot(dr, dh) || 1;
    segmentNormals.push([dh / l, -dr / l]);
  }
  const limit = Math.cos((crease * Math.PI) / 180);
  const last = profile.length - 1;
  const closed =
    Math.hypot(profile[0][0] - profile[last][0], profile[0][1] - profile[last][1]) < 1e-9;
  // Rings of vertices: a profile point is one ring where the surface is
  // smooth and two where it is creased. A profile that ends where it began
  // is smoothed across the join too.
  const rings = [];
  for (let k = 0; k < profile.length; k++) {
    const before = segmentNormals[k - 1] || (closed ? segmentNormals[last - 1] : undefined);
    const after = segmentNormals[k] || (closed ? segmentNormals[0] : undefined);
    if (closed && (k === 0 || k === last)) {
      const smooth = before[0] * after[0] + before[1] * after[1] >= limit;
      const normal = smooth ? [before[0] + after[0], before[1] + after[1]] : k === 0 ? after : before;
      rings.push({ k, normal, closes: k === last });
      continue;
    }
    if (before && after) {
      const smooth = before[0] * after[0] + before[1] * after[1] >= limit;
      if (smooth) {
        rings.push({ k, normal: [before[0] + after[0], before[1] + after[1]], closes: true });
      } else {
        rings.push({ k, normal: before, closes: true });
        rings.push({ k, normal: after, closes: false });
      }
    } else {
      rings.push({ k, normal: before || after, closes: Boolean(before) });
    }
  }
  const base = mesh.positions.length;
  for (const ring of rings) {
    const [r, h] = profile[ring.k];
    for (let j = 0; j < segments; j++) {
      const t = (j / segments) * TAU;
      const radial = add(scale(u, Math.cos(t)), scale(v, Math.sin(t)));
      mesh.vertex(
        add(add(origin, scale(a, h)), scale(radial, r)),
        add(scale(a, ring.normal[1]), scale(radial, ring.normal[0])),
      );
    }
  }
  for (let i = 0; i + 1 < rings.length; i++) {
    if (!rings[i + 1].closes || rings[i + 1].k === rings[i].k) continue;
    for (let j = 0; j < segments; j++) {
      const k = (j + 1) % segments;
      mesh.quad(
        base + i * segments + j,
        base + i * segments + k,
        base + (i + 1) * segments + k,
        base + (i + 1) * segments + j,
      );
    }
  }
  return mesh;
}

// A solid cylinder with chamfered ends, from `from` to `to` along `axis`.
function cylinder(mesh, origin, axis, radius, from, to, chamfer = 0, segments = 48) {
  const c = Math.min(chamfer, radius * 0.5, (to - from) * 0.45);
  const profile = [[0, from], [radius - c, from]];
  if (c > 0) profile.push([radius, from + c], [radius, to - c]);
  else profile.push([radius, to]);
  if (c > 0) profile.push([radius - c, to]);
  profile.push([0, to]);
  return lathe(mesh, origin, axis, profile, segments);
}

// A point of the superellipse |x/a|^n + |z/b|^n = 1.
function superellipse(a, b, n, t) {
  const c = Math.cos(t);
  const s = Math.sin(t);
  const e = 2 / n;
  return [Math.sign(c) * a * Math.abs(c) ** e, Math.sign(s) * b * Math.abs(s) ** e];
}

// A solid lofted through superellipse sections [y, a, b, n], closed with a
// flat cap at each end. `across` and `along` are the directions of a and b.
function loft(mesh, sections, options = {}) {
  const { across = X, along = Z, up = Y, origin = [0, 0, 0], ring = 96 } = options;
  const place = (y, x, z) =>
    add(add(add(origin, scale(up, y)), scale(across, x)), scale(along, z));
  const points = sections.map(([y, a, b, n]) =>
    Array.from({ length: ring }, (_, j) => {
      const [x, z] = superellipse(a, b, n, (j / ring) * TAU);
      return place(y, x, z);
    }),
  );
  const rows = sections.length;
  mesh.grid(
    rows,
    ring,
    (i, j) => {
      const here = points[i][j];
      const around = sub(points[i][(j + 1) % ring], points[i][(j + ring - 1) % ring]);
      const upward = sub(points[Math.min(i + 1, rows - 1)][j], points[Math.max(i - 1, 0)][j]);
      let normal = cross(around, upward);
      const centre = place(sections[i][0], 0, 0);
      if (dot(normal, sub(here, centre)) < 0) normal = scale(normal, -1);
      return [here, normal];
    },
    true,
  );
  mesh.fan(place(sections[0][0], 0, 0), points[0], scale(up, -1));
  mesh.fan(place(sections[rows - 1][0], 0, 0), points[rows - 1], up);
  return mesh;
}

// A flat plate with a superellipse outline between two heights.
function plate(mesh, y0, y1, a, b, n, options = {}) {
  return loft(mesh, [[y0, a, b, n], [y1, a, b, n]], options);
}

// A box with flat faces.
function box(mesh, centre, size) {
  const h = scale(size, 0.5);
  for (const [axis, sign] of [[0, 1], [0, -1], [1, 1], [1, -1], [2, 1], [2, -1]]) {
    const normal = [0, 0, 0];
    normal[axis] = sign;
    const [p, q] = [(axis + 1) % 3, (axis + 2) % 3];
    const corners = [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([s, t]) => {
      const point = [...centre];
      point[axis] += sign * h[axis];
      point[p] += s * h[p];
      point[q] += t * h[q];
      return mesh.vertex(point, normal);
    });
    mesh.quad(...corners);
  }
  return mesh;
}

// A hexagonal prism (a nut or a plug head) along `axis`.
function hexPrism(mesh, origin, axis, acrossFlats, from, to) {
  const { a, u, v } = frame(axis);
  const radius = acrossFlats / Math.sqrt(3);
  const corner = (k, h) => {
    const t = (k / 6) * TAU;
    return add(
      add(origin, scale(a, h)),
      add(scale(u, radius * Math.cos(t)), scale(v, radius * Math.sin(t))),
    );
  };
  for (let k = 0; k < 6; k++) {
    const t = ((k + 0.5) / 6) * TAU;
    const normal = add(scale(u, Math.cos(t)), scale(v, Math.sin(t)));
    mesh.quad(
      mesh.vertex(corner(k, from), normal),
      mesh.vertex(corner(k + 1, from), normal),
      mesh.vertex(corner(k + 1, to), normal),
      mesh.vertex(corner(k, to), normal),
    );
  }
  const ring = (h) => Array.from({ length: 6 }, (_, k) => corner(k, h));
  mesh.fan(add(origin, scale(a, from)), ring(from), scale(a, -1));
  mesh.fan(add(origin, scale(a, to)), ring(to), a);
  return mesh;
}

// A tube swept along `path`, a list of { at, across: [half, half], n }, with
// a superellipse section whose first half-width lies along `side`.
function sweep(mesh, path, side, ring = 40) {
  const rows = path.length;
  const frames = path.map((station, i) => {
    const tangent = normalize(
      sub(path[Math.min(i + 1, rows - 1)].at, path[Math.max(i - 1, 0)].at),
    );
    const s = normalize(sub(side, scale(tangent, dot(side, tangent))));
    return { tangent, s, t: cross(tangent, s) };
  });
  const point = (i, j) => {
    const { at, across, n } = path[i];
    const [x, z] = superellipse(across[0], across[1], n, (j / ring) * TAU);
    return add(at, add(scale(frames[i].s, x), scale(frames[i].t, z)));
  };
  mesh.grid(
    rows,
    ring,
    (i, j) => {
      const here = point(i, j);
      const around = sub(point(i, (j + 1) % ring), point(i, (j + ring - 1) % ring));
      const onward = sub(point(Math.min(i + 1, rows - 1), j), point(Math.max(i - 1, 0), j));
      let normal = cross(around, onward);
      if (dot(normal, sub(here, path[i].at)) < 0) normal = scale(normal, -1);
      return [here, normal];
    },
    true,
  );
  const end = (i, direction) =>
    mesh.fan(
      path[i].at,
      Array.from({ length: ring }, (_, j) => point(i, j)),
      scale(frames[i].tangent, direction),
    );
  end(0, -1);
  end(rows - 1, 1);
  return mesh;
}

// A flange: a ring plate across the X axis from x0 to x1 with `count` bolt
// holes on a circle. Each hole sits in its own sector of the ring; the plate
// is meshed from the edge of the hole out to the edge of the sector, so the
// faces, the hole walls, the rim and the bore share their vertices.
function flange(mesh, x0, x1, inner, outer, boltCircle, holeRadius, count) {
  const half = Math.PI / count;
  const [ARC, SIDE, BORE] = [40, 10, 12];
  const polar = (r, t) => [r * Math.cos(t), r * Math.sin(t)];
  for (let s = 0; s < count; s++) {
    const mid = (s / count) * TAU;
    const centre = polar(boltCircle, mid);
    const edge = [];
    for (let k = 0; k < ARC / 2; k++) edge.push(polar(outer, mid + (k / (ARC / 2)) * half));
    for (let k = 0; k < SIDE; k++) {
      edge.push(polar(outer + (inner - outer) * (k / SIDE), mid + half));
    }
    for (let k = 0; k < BORE; k++) edge.push(polar(inner, mid + half - (k / BORE) * 2 * half));
    for (let k = 0; k < SIDE; k++) {
      edge.push(polar(inner + (outer - inner) * (k / SIDE), mid - half));
    }
    for (let k = 0; k < ARC / 2; k++) {
      edge.push(polar(outer, mid - half + (k / (ARC / 2)) * half));
    }
    const hole = edge.map(([y, z]) => {
      const angle = Math.atan2(z - centre[1], y - centre[0]);
      return [centre[0] + holeRadius * Math.cos(angle), centre[1] + holeRadius * Math.sin(angle)];
    });
    const count2 = edge.length;
    for (const [x, sign] of [[x0, -1], [x1, 1]]) {
      const normal = [sign, 0, 0];
      const rim = edge.map(([y, z]) => mesh.vertex([x, y, z], normal));
      const lip = hole.map(([y, z]) => mesh.vertex([x, y, z], normal));
      for (let k = 0; k < count2; k++) {
        const next = (k + 1) % count2;
        mesh.quad(lip[k], lip[next], rim[next], rim[k]);
      }
    }
    // Walls: the bolt hole faces its own axis, the rim faces outward and
    // the bore faces the pipe axis.
    const wall = (loop, from, to, normalOf) => {
      for (let k = from; k < to; k++) {
        const next = (k + 1) % count2;
        const corners = [[loop[k], x0], [loop[next], x0], [loop[next], x1], [loop[k], x1]];
        mesh.quad(...corners.map(([[y, z], x]) => mesh.vertex([x, y, z], normalOf(y, z))));
      }
    };
    wall(hole, 0, count2, (y, z) => [0, centre[0] - y, centre[1] - z]);
    wall(edge, 0, ARC / 2, (y, z) => [0, y, z]);
    wall(edge, count2 - ARC / 2, count2, (y, z) => [0, y, z]);
    wall(edge, ARC / 2 + SIDE, ARC / 2 + SIDE + BORE, (y, z) => [0, -y, -z]);
  }
  return mesh;
}

// A screw thread about the Y axis: the trapezoid of an ACME thread swept
// along a helix from y0 up to y1.
function thread(mesh, y0, y1, major, minor, pitch, segments = 40) {
  const flat = pitch * 0.18;
  const flank = pitch * 0.5 - flat;
  // One pitch of the profile as [radius, height] pieces.
  const pieces = [
    [[minor, 0], [minor, flat]],
    [[minor, flat], [major, flat + flank]],
    [[major, flat + flank], [major, 2 * flat + flank]],
    [[major, 2 * flat + flank], [minor, pitch]],
  ];
  const turns = (y1 - y0) / pitch;
  const steps = Math.ceil(turns * segments);
  for (const [[r0, h0], [r1, h1]] of pieces) {
    const [dr, dh] = [r1 - r0, h1 - h0];
    mesh.grid(
      steps + 1,
      2,
      (i, j) => {
        const t = (i / segments) * TAU;
        const [r, h] = j === 0 ? [r0, h0] : [r1, h1];
        const y = y0 + (i / segments) * pitch + h;
        const radial = [Math.cos(t), 0, Math.sin(t)];
        const tangent = [-r * Math.sin(t), pitch / TAU, r * Math.cos(t)];
        const slope = [dr * Math.cos(t), dh, dr * Math.sin(t)];
        let normal = cross(slope, tangent);
        if (dot(normal, radial) < 0) normal = scale(normal, -1);
        return [[r * Math.cos(t), y, r * Math.sin(t)], normal];
      },
      false,
    );
  }
  return mesh;
}

// Raised block lettering on one of the body's broad sides, which face +Z
// and -Z. `wall(y, x)` gives how far the side stands from the pipe axis,
// and each letter is built from bars sunk into it.
const GLYPHS = {
  D: ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
  N: ["10001", "11001", "11001", "10101", "10011", "10011", "10001"],
  P: ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
  0: ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
  1: ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
  6: ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
};

function lettering(mesh, text, centreY, height, facing, wall, relief) {
  const pixel = height / 7;
  const advance = pixel * 6.2;
  const width = advance * text.length - pixel * 1.2;
  [...text].forEach((letter, index) => {
    GLYPHS[letter].forEach((row, line) => {
      let column = 0;
      while (column < 5) {
        if (row[column] !== "1") {
          column += 1;
          continue;
        }
        let run = 1;
        while (column + run < 5 && row[column + run] === "1") run += 1;
        const along = -width / 2 + index * advance + (column + run / 2) * pixel;
        const y = centreY + height / 2 - (line + 0.5) * pixel;
        // Seen from outside, text on the side facing +Z reads along +X.
        const x = facing * along;
        const depth = 0.006;
        const z = facing * (wall(y, x) + relief - depth / 2);
        box(mesh, [x, y, z], [run * pixel * 1.04, pixel * 1.08, depth]);
        column += run;
      }
    });
  });
  return mesh;
}

// ------------------------------------------------------------- dimensions

// End flanges, to the proportions of a 4 inch class 150 flange.
const FACE = 0.1145; // half the face-to-face length
const FLANGE_THICKNESS = 0.024;
const FLANGE_RADIUS = 0.1145;
const BORE = 0.05;
const BOLT_CIRCLE = 0.09525;
const BOLT_HOLE = 0.0095;
const RAISED_FACE = 0.0785;
const RAISED_HEIGHT = 0.0016;
// The body's chamber, which houses the wedge, and the joint above it.
const CHAMBER = { a: 0.052, b: 0.1, n: 6 };
const JOINT = { a: 0.095, b: 0.145, n: 5 };
const BODY_FLANGE = [0.15, 0.172];
const GASKET = [0.172, 0.1745];
const BONNET_FLANGE = [0.1745, 0.1965];
const STUD_X = 0.075;
const STUD_Z = [-0.1, -0.05, 0, 0.05, 0.1];
const YOKE_Z = 0.072;
const HUB = [0.478, 0.52];
const WHEEL_Y = 0.578;
const WHEEL_RADIUS = 0.125;

// How far the chamber's broad side stands from the pipe axis at `x`.
const chamberSide = (y, x) =>
  CHAMBER.b * (1 - Math.min(Math.abs(x) / CHAMBER.a, 1) ** CHAMBER.n) ** (1 / CHAMBER.n);

function body() {
  const mesh = new Mesh();
  // The pipe run between the flanges: a barrel with a fillet up to each
  // flange, and the bore through it.
  const back = FACE - FLANGE_THICKNESS;
  lathe(mesh, [0, 0, 0], X, [
    [0.082, -back],
    [0.073, -back + 0.005],
    [0.068, -back + 0.014],
    [0.0655, -back + 0.03],
    [0.0655, back - 0.03],
    [0.068, back - 0.014],
    [0.073, back - 0.005],
    [0.082, back],
  ]);
  lathe(mesh, [0, 0, 0], X, [[BORE, back], [BORE, -back]]);
  flange(mesh, -FACE, -back, BORE, FLANGE_RADIUS, BOLT_CIRCLE, BOLT_HOLE, 8);
  flange(mesh, back, FACE, BORE, FLANGE_RADIUS, BOLT_CIRCLE, BOLT_HOLE, 8);
  // The chamber: a rounded belly below the pipe, straight walls beside it
  // and a flare up to the joint.
  loft(mesh, [
    [-0.092, 0.012, 0.024, 2.5],
    [-0.089, 0.03, 0.058, 3],
    [-0.079, 0.044, 0.084, 3.5],
    [-0.06, 0.05, 0.096, 5],
    [-0.03, CHAMBER.a, CHAMBER.b, CHAMBER.n],
    [0.135, CHAMBER.a, CHAMBER.b, CHAMBER.n],
    [0.144, 0.055, 0.104, 6],
    [BODY_FLANGE[0] + 0.002, 0.058, 0.108, 6],
  ]);
  plate(mesh, BODY_FLANGE[0], BODY_FLANGE[1], JOINT.a, JOINT.b, JOINT.n);
  // A boss for the drain plug under the belly.
  cylinder(mesh, [0, 0, 0], Y, 0.017, -0.098, -0.088, 0.0015);
  return mesh;
}

function bodyLettering() {
  const mesh = new Mesh();
  lettering(mesh, "DN100", 0.064, 0.017, 1, chamberSide, 0.0018);
  lettering(mesh, "PN16", 0.036, 0.017, 1, chamberSide, 0.0018);
  return mesh;
}

function bonnet() {
  const mesh = new Mesh();
  plate(mesh, BONNET_FLANGE[0], BONNET_FLANGE[1], JOINT.a, JOINT.b, JOINT.n);
  // The dome narrows from the joint to the round neck of the stuffing box.
  loft(mesh, [
    [BONNET_FLANGE[1] - 0.002, 0.058, 0.108, 6],
    [0.215, 0.054, 0.1, 4.5],
    [0.24, 0.048, 0.082, 3],
    [0.262, 0.041, 0.06, 2.6],
    [0.28, 0.036, 0.042, 2.2],
    [0.292, 0.034, 0.034, 2],
    [0.335, 0.034, 0.034, 2],
  ]);
  // The ears that carry the gland studs.
  plate(mesh, 0.317, 0.335, 0.03, 0.056, 2.6);
  // The yoke: two arms from the dome's shoulders up to the hub that holds
  // the stem nut.
  for (const side of [-1, 1]) {
    const arm = [
      [0.225, YOKE_Z, 0.0165, 0.012],
      [0.3, YOKE_Z, 0.015, 0.011],
      [0.4, YOKE_Z, 0.014, 0.0105],
      [0.428, 0.0705, 0.014, 0.0105],
      [0.452, 0.064, 0.014, 0.0105],
      [0.471, 0.052, 0.0145, 0.011],
      [0.484, 0.037, 0.015, 0.0115],
      [0.492, 0.018, 0.015, 0.012],
    ].map(([y, z, alongX, inPlane]) => ({
      at: [0, y, side * z],
      across: [alongX, inPlane],
      n: 3.6,
    }));
    sweep(mesh, arm, X);
  }
  cylinder(mesh, [0, 0, 0], Y, 0.036, HUB[0], HUB[1], 0.003, 64);
  return mesh;
}

function gland() {
  const mesh = new Mesh();
  cylinder(mesh, [0, 0, 0], Y, 0.0215, 0.33, 0.354, 0, 48);
  plate(mesh, 0.352, 0.366, 0.029, 0.056, 2.6);
  cylinder(mesh, [0, 0, 0], Y, 0.02, 0.364, 0.372, 0.0015, 48);
  return mesh;
}

function raisedFaces() {
  const mesh = new Mesh();
  for (const side of [-1, 1]) {
    // The gasket face stands proud of the flange and is turned to a
    // serrated finish.
    const [from, to] = [side * FACE, side * (FACE + RAISED_HEIGHT)];
    const profile = [
      [BORE, from],
      [RAISED_FACE, from],
      [RAISED_FACE, to],
      [BORE, to],
      [BORE, from],
    ];
    lathe(mesh, [0, 0, 0], X, side > 0 ? profile : profile.reverse(), 128);
  }
  return mesh;
}

function trim() {
  const mesh = new Mesh();
  // The closed wedge, seen down the bore, and the seat ring it shuts on.
  for (const side of [-1, 1]) {
    const faces = [side * 0.011, side * 0.016];
    cylinder(mesh, [0, 0, 0], X, 0.0495, Math.min(...faces), Math.max(...faces), 0, 64);
  }
  return mesh;
}

function seats() {
  const mesh = new Mesh();
  for (const side of [-1, 1]) {
    const [from, to] = side > 0 ? [0.016, 0.024] : [-0.024, -0.016];
    lathe(mesh, [0, 0, 0], X, [[BORE, to], [0.043, to], [0.043, from], [BORE, from]], 64);
  }
  return mesh;
}

function gasket() {
  return plate(new Mesh(), GASKET[0], GASKET[1], JOINT.a - 0.004, JOINT.b - 0.004, JOINT.n);
}

function stem() {
  const mesh = new Mesh();
  cylinder(mesh, [0, 0, 0], Y, 0.0125, 0.33, 0.398, 0, 48);
  thread(mesh, 0.395, 0.642, 0.013, 0.0104, 0.005);
  cylinder(mesh, [0, 0, 0], Y, 0.0104, 0.39, 0.652, 0.0012, 40);
  return mesh;
}

function stemNut() {
  const mesh = new Mesh();
  cylinder(mesh, [0, 0, 0], Y, 0.03, HUB[1], HUB[1] + 0.008, 0.001, 64);
  cylinder(mesh, [0, 0, 0], Y, 0.0235, HUB[1] + 0.006, 0.556, 0.001, 64);
  return mesh;
}

function handwheel() {
  const mesh = new Mesh();
  // Rim: a ring of round section.
  const rim = Array.from({ length: 25 }, (_, k) => {
    const t = (k / 24) * TAU;
    return [WHEEL_RADIUS + 0.0105 * Math.cos(t), WHEEL_Y + 0.0105 * Math.sin(t)];
  });
  lathe(mesh, [0, 0, 0], Y, rim, 128, 80);
  // Hub, with a waist.
  lathe(mesh, [0, 0, 0], Y, [
    [0, 0.552],
    [0.027, 0.552],
    [0.03, 0.555],
    [0.03, 0.566],
    [0.026, 0.574],
    [0.026, 0.586],
    [0.0235, 0.59],
    [0, 0.59],
  ], 64);
  // Five spokes, oval in section and tapering towards the rim.
  for (let k = 0; k < 5; k++) {
    const t = (k / 5) * TAU + 0.3;
    const outward = [Math.cos(t), 0, Math.sin(t)];
    const side = [-Math.sin(t), 0, Math.cos(t)];
    const stations = [0.02, 0.045, 0.075, 0.1, 0.122].map((r, i, all) => {
      const share = i / (all.length - 1);
      return {
        at: add(scale(outward, r), [0, 0.57 + (WHEEL_Y - 0.57) * share, 0]),
        across: [0.0115 - 0.003 * share, 0.008 - 0.002 * share],
        n: 2.2,
      };
    });
    sweep(mesh, stations, side, 28);
  }
  return mesh;
}

function fasteners() {
  const mesh = new Mesh();
  // Studs through the body-bonnet joint, with a nut above and below.
  for (const x of [-STUD_X, STUD_X]) {
    for (const z of STUD_Z) {
      const at = [x, 0, z];
      cylinder(mesh, at, Y, 0.008, BODY_FLANGE[0] - 0.024, BONNET_FLANGE[1] + 0.024, 0.0012, 24);
      hexPrism(mesh, at, Y, 0.024, BONNET_FLANGE[1], BONNET_FLANGE[1] + 0.016);
      hexPrism(mesh, at, Y, 0.024, BODY_FLANGE[0] - 0.016, BODY_FLANGE[0]);
    }
  }
  // The two gland studs and their nuts.
  for (const z of [-0.042, 0.042]) {
    const at = [0, 0, z];
    cylinder(mesh, at, Y, 0.006, 0.318, 0.388, 0.001, 24);
    hexPrism(mesh, at, Y, 0.019, 0.366, 0.377);
  }
  // The nut that holds the handwheel, and the drain plug.
  hexPrism(mesh, [0, 0, 0], Y, 0.036, 0.59, 0.604);
  hexPrism(mesh, [0, 0, 0], Y, 0.022, -0.106, -0.098);
  return mesh;
}

function nameplate() {
  const mesh = new Mesh();
  // Riveted to the broad side that faces -Z.
  const [y, z] = [0.05, -(CHAMBER.b + 0.0009)];
  box(mesh, [0, y, z], [0.062, 0.03, 0.0012]);
  for (const [dy, dx] of [[-1, -1], [-1, 1], [1, -1], [1, 1]]) {
    const dome = Array.from({ length: 7 }, (_, k) => {
      const t = (k / 6) * (Math.PI / 2);
      return [0.0022 * Math.cos(t), 0.0022 * Math.sin(t) * 0.7];
    });
    lathe(mesh, [dx * 0.0275, y + dy * 0.0115, z - 0.0006], [0, 0, -1], dome, 16, 80);
  }
  return mesh;
}

// -------------------------------------------------------------- materials

// Base colours are linear. Each finish is read by scena's path tracer.
const MATERIALS = {
  bodyPaint: {
    name: "epoxy coating, blue",
    color: [0.013, 0.105, 0.36],
    metallic: 0,
    roughness: 0.34,
    finish: { kind: "powder_coated_metal", scale: 1.4, relief: 1.2, wear: 0.1, grime: 0.3, edge_radius_m: 0.0026 },
  },
  lettering: {
    name: "epoxy coating, blue, cast lettering",
    color: [0.013, 0.105, 0.36],
    metallic: 0,
    roughness: 0.34,
    finish: { kind: "powder_coated_metal", scale: 1.4, relief: 1.2, wear: 0.2, grime: 0.3, edge_radius_m: 0.0008 },
  },
  wheelPaint: {
    name: "enamel, signal red",
    color: [0.42, 0.018, 0.012],
    metallic: 0,
    roughness: 0.32,
    finish: { kind: "powder_coated_metal", scale: 1.1, relief: 0.9, wear: 0.4, grime: 0.2, edge_radius_m: 0.002, seed: 4 },
  },
  machined: {
    name: "carbon steel, turned gasket face",
    color: [0.58, 0.58, 0.6],
    metallic: 1,
    roughness: 0.3,
    finish: { kind: "machined_metal", scale: 3, axis: [1, 0, 0], grime: 0.15 },
  },
  trim: {
    name: "stainless steel wedge",
    color: [0.5, 0.5, 0.52],
    metallic: 1,
    roughness: 0.36,
    finish: { kind: "satin_metal", grime: 0.3 },
  },
  bronze: {
    name: "bronze, turned",
    color: [0.62, 0.36, 0.14],
    metallic: 1,
    roughness: 0.3,
    finish: { kind: "machined_metal", scale: 1.6, axis: [0, 1, 0], wear: 0.2, grime: 0.2 },
  },
  seatBronze: {
    name: "bronze seat ring",
    color: [0.62, 0.36, 0.14],
    metallic: 1,
    roughness: 0.32,
    finish: { kind: "machined_metal", scale: 1.6, axis: [1, 0, 0], grime: 0.3 },
  },
  stem: {
    name: "stainless steel stem",
    color: [0.6, 0.6, 0.62],
    metallic: 1,
    roughness: 0.24,
    finish: { kind: "machined_metal", scale: 0.8, relief: 0.7, axis: [0, 1, 0], grime: 0.25, edge_radius_m: 0.0003 },
  },
  zinc: {
    name: "zinc-plated steel",
    color: [0.66, 0.67, 0.68],
    metallic: 1,
    roughness: 0.42,
    finish: { kind: "satin_metal", scale: 1.5, wear: 0.15, grime: 0.3, edge_radius_m: 0.0006 },
  },
  gasket: {
    name: "graphite gasket",
    color: [0.022, 0.022, 0.024],
    metallic: 0,
    roughness: 0.8,
    finish: { kind: "rubber" },
  },
  plate: {
    name: "aluminium nameplate",
    color: [0.78, 0.78, 0.79],
    metallic: 1,
    roughness: 0.32,
    finish: { kind: "brushed_metal", axis: [1, 0, 0], edge_radius_m: 0.0002 },
  },
};

const PARTS = [
  ["body", body, "bodyPaint"],
  ["body_lettering", bodyLettering, "lettering"],
  ["bonnet_and_yoke", bonnet, "bodyPaint"],
  ["gland", gland, "bodyPaint"],
  ["raised_faces", raisedFaces, "machined"],
  ["wedge", trim, "trim"],
  ["seat_rings", seats, "seatBronze"],
  ["gasket", gasket, "gasket"],
  ["stem", stem, "stem"],
  ["stem_nut", stemNut, "bronze"],
  ["handwheel", handwheel, "wheelPaint"],
  ["fasteners", fasteners, "zinc"],
  ["nameplate", nameplate, "plate"],
];

// ---------------------------------------------------------------- writing

function check(name, mesh) {
  const fail = (reason) => {
    throw new Error(`${name}: ${reason}`);
  };
  if (mesh.indices.length === 0) fail("no triangles");
  mesh.positions.forEach((position, index) => {
    if (!position.every(Number.isFinite)) fail(`vertex ${index} is not finite`);
    if (Math.abs(length(mesh.normals[index]) - 1) > 1e-6) fail(`normal ${index} is not unit`);
  });
  if (mesh.indices.some((index) => index >= mesh.positions.length)) fail("index out of range");
}

function writeGlb() {
  const chunks = [];
  let offset = 0;
  const bufferViews = [];
  const accessors = [];
  const push = (typed, target) => {
    const bytes = Buffer.from(typed.buffer, typed.byteOffset, typed.byteLength);
    const padded = Buffer.concat([bytes, Buffer.alloc((4 - (bytes.length % 4)) % 4)]);
    bufferViews.push({ buffer: 0, byteOffset: offset, byteLength: bytes.length, target });
    chunks.push(padded);
    offset += padded.length;
    return bufferViews.length - 1;
  };
  const materialNames = Object.keys(MATERIALS);
  const meshes = [];
  const nodes = [];
  let triangles = 0;
  const bounds = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] };
  for (const [name, build, material] of PARTS) {
    const mesh = build();
    check(name, mesh);
    triangles += mesh.indices.length / 3;
    const min = [Infinity, Infinity, Infinity];
    const max = [-Infinity, -Infinity, -Infinity];
    for (const position of mesh.positions) {
      for (let axis = 0; axis < 3; axis++) {
        const value = Math.fround(position[axis]);
        min[axis] = Math.min(min[axis], value);
        max[axis] = Math.max(max[axis], value);
        bounds.min[axis] = Math.min(bounds.min[axis], value);
        bounds.max[axis] = Math.max(bounds.max[axis], value);
      }
    }
    const count = mesh.positions.length;
    const position = push(new Float32Array(mesh.positions.flat()), 34962);
    const normal = push(new Float32Array(mesh.normals.flat()), 34962);
    const index = push(new Uint32Array(mesh.indices), 34963);
    accessors.push(
      { bufferView: position, componentType: 5126, count, type: "VEC3", min, max },
      { bufferView: normal, componentType: 5126, count, type: "VEC3" },
      { bufferView: index, componentType: 5125, count: mesh.indices.length, type: "SCALAR" },
    );
    meshes.push({
      name,
      primitives: [
        {
          attributes: { POSITION: accessors.length - 3, NORMAL: accessors.length - 2 },
          indices: accessors.length - 1,
          material: materialNames.indexOf(material),
          mode: 4,
        },
      ],
    });
    nodes.push({ name, mesh: meshes.length - 1 });
    console.log(`  ${name.padEnd(18)} ${String(mesh.indices.length / 3).padStart(7)} triangles`);
  }
  nodes.push({ name: "gate_valve_dn100", children: nodes.map((_, index) => index) });
  const json = {
    asset: {
      version: "2.0",
      generator: "scena scripts/generate_gate_valve_demo_asset.js",
      copyright: "Generated by the scena project; see demo/samples/SOURCES.md",
    },
    scene: 0,
    scenes: [{ name: "gate_valve", nodes: [nodes.length - 1] }],
    nodes,
    meshes,
    materials: materialNames.map((key) => {
      const material = MATERIALS[key];
      return {
        name: material.name,
        pbrMetallicRoughness: {
          baseColorFactor: [...material.color, 1],
          metallicFactor: material.metallic,
          roughnessFactor: material.roughness,
        },
        extras: { scena: { surface_finish: material.finish } },
      };
    }),
    accessors,
    bufferViews,
    buffers: [{ byteLength: offset }],
  };
  let jsonBytes = Buffer.from(JSON.stringify(json), "utf8");
  jsonBytes = Buffer.concat([jsonBytes, Buffer.alloc((4 - (jsonBytes.length % 4)) % 4, 0x20)]);
  const binary = Buffer.concat(chunks);
  const header = Buffer.alloc(12);
  header.writeUInt32LE(0x46546c67, 0);
  header.writeUInt32LE(2, 4);
  header.writeUInt32LE(12 + 8 + jsonBytes.length + 8 + binary.length, 8);
  const chunkHeader = (byteLength, type) => {
    const head = Buffer.alloc(8);
    head.writeUInt32LE(byteLength, 0);
    head.writeUInt32LE(type, 4);
    return head;
  };
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(
    OUT,
    Buffer.concat([
      header,
      chunkHeader(jsonBytes.length, 0x4e4f534a),
      jsonBytes,
      chunkHeader(binary.length, 0x004e4942),
      binary,
    ]),
  );
  const size = bounds.max.map((value, axis) => (value - bounds.min[axis]).toFixed(4));
  console.log(`${path.relative(REPO, OUT)}: ${triangles} triangles, ${size.join(" x ")} m`);
  console.log(`  lowest point y = ${bounds.min[1].toFixed(4)}`);
}

writeGlb();

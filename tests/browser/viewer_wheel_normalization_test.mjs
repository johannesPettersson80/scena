import fs from "node:fs";

const source = fs.readFileSync("src/viewer_element/element.js", "utf8");
const moduleUrl = `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const viewerElement = await import(moduleUrl);
const normalize = viewerElement.normalizeScenaViewerWheelDelta;

if (typeof normalize !== "function") {
  throw new Error("element.js must export normalizeScenaViewerWheelDelta");
}

const cases = [
  { name: "pixel mouse wheel", deltaY: 100, deltaMode: 0, expected: 1 },
  { name: "pixel trackpad", deltaY: 5, deltaMode: 0, expected: 0.05 },
  { name: "line wheel", deltaY: 3, deltaMode: 1, expected: 1 },
  { name: "page wheel", deltaY: 1, deltaMode: 2, expected: 1 },
  { name: "reverse pixel wheel", deltaY: -100, deltaMode: 0, expected: -1 },
];

for (const testCase of cases) {
  const actual = normalize(testCase.deltaY, testCase.deltaMode);
  if (Math.abs(actual - testCase.expected) > 1e-9) {
    throw new Error(`${testCase.name}: expected ${testCase.expected}, got ${actual}`);
  }
}

for (const invalid of [NaN, Infinity, -Infinity]) {
  if (normalize(invalid, 0) !== 0) {
    throw new Error(`non-finite delta ${invalid} must normalize to zero`);
  }
}

if (normalize(100000, 0) !== 4 || normalize(-100000, 0) !== -4) {
  throw new Error("wheel normalization must clamp pathological events symmetrically");
}

console.log("viewer wheel normalization: pass");

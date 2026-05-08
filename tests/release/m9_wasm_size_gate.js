const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const root = process.cwd();
const input = path.join(root, "target", "m9-browser-pkg", "scena_bg.wasm");
const jsGlue = path.join(root, "target", "m9-browser-pkg", "scena.js");
const artifactDir = path.join(root, "target", "gate-artifacts");
const optimized = path.join(artifactDir, "scena_bg.opt.wasm");
const compressed = `${optimized}.br`;
const limitBytes = 2 * 1024 * 1024;

function bin(name) {
  const suffix = process.platform === "win32" ? ".cmd" : "";
  return path.join(root, "node_modules", ".bin", `${name}${suffix}`);
}

function packageVersion(name) {
  const packageJson = path.join(root, "node_modules", name, "package.json");
  return JSON.parse(fs.readFileSync(packageJson, "utf8")).version;
}

fs.mkdirSync(artifactDir, { recursive: true });
if (!fs.existsSync(input)) {
  throw new Error(`missing WASM input: ${input}`);
}
if (!fs.existsSync(jsGlue)) {
  throw new Error(`missing WASM JS glue: ${jsGlue}`);
}

const jsGlueSource = fs.readFileSync(jsGlue, "utf8");
const requiredProbeExports = [
  "m6RenderWebgl2Probe",
  "m6RenderWebgpuProbe",
  "m6RenderWorkflowProbe",
  "m6RenderStateLifecycleProbe",
];
const missingProbeExports = requiredProbeExports.filter(
  (name) => !jsGlueSource.includes(name),
);
if (missingProbeExports.length > 0) {
  throw new Error(
    `WASM size gate must measure the browser-probe renderer bundle; missing exports: ${missingProbeExports.join(", ")}`,
  );
}

execFileSync(bin("wasm-opt"), [
  "-Oz",
  "--strip-debug",
  "--strip-producers",
  input,
  "-o",
  optimized,
], { stdio: "inherit" });

if (fs.existsSync(compressed)) {
  fs.unlinkSync(compressed);
}
execFileSync(bin("brotli-cli"), ["compress", "--quality", "11", optimized], {
  stdio: "inherit",
});

const artifact = {
  schema: "scena.m9.wasm_size.v1",
  status: fs.statSync(compressed).size <= limitBytes ? "passed" : "failed",
  input,
  js_glue: jsGlue,
  required_features: ["browser-probe"],
  required_probe_exports: requiredProbeExports,
  feature_enabled_probe_exports_present: true,
  optimized,
  compressed,
  raw_wasm_bytes: fs.statSync(input).size,
  optimized_wasm_bytes: fs.statSync(optimized).size,
  brotli_bytes: fs.statSync(compressed).size,
  limit_bytes: limitBytes,
  wasm_opt_package: packageVersion("binaryen"),
  brotli_package: packageVersion("brotli-cli"),
};
fs.writeFileSync(
  path.join(artifactDir, "m9-wasm-size.json"),
  `${JSON.stringify(artifact, null, 2)}\n`,
);
console.log(JSON.stringify(artifact, null, 2));
if (artifact.status !== "passed") {
  process.exit(1);
}

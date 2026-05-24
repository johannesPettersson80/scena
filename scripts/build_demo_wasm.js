#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { spawn, spawnSync } = require("child_process");
const zlib = require("zlib");

const HEARTBEAT_MS = Number(process.env.SCENA_BUILD_HEARTBEAT_MS || 20_000);
const command = process.platform === "win32" ? "wasm-pack.cmd" : "wasm-pack";
const mode = process.argv[2] || "demo";
const bundle = mode === "proof"
  ? {
      label: "scena-proof-build",
      name: "proof",
      outDir: "demo/proof/pkg",
      features: "demo-page,proof-harness,browser-probe",
    }
  : {
      label: "scena-demo-build",
      name: "public",
      outDir: "demo/pkg",
      features: "demo-page",
    };
if (!["demo", "proof"].includes(mode)) {
  console.error(`[scena-demo-build] unknown bundle '${mode}', expected demo or proof`);
  process.exit(1);
}
const args = [
  "build",
  "--release",
  "--target",
  "web",
  "--out-dir",
  bundle.outDir,
  ".",
  "--features",
  bundle.features,
];

let lastOutputAt = Date.now();
const startedAt = lastOutputAt;

console.log(`[${bundle.label}] running: ${command} ${args.join(" ")}`);

const child = spawn(command, args, {
  cwd: process.cwd(),
  env: {
    ...process.env,
    // Some builder hosts set native linker flags globally in Cargo config.
    // wasm32-unknown-unknown links with rust-lld and rejects native linker
    // arguments such as -fuse-ld=mold, so keep this wasm-pack build target-clean.
    CARGO_ENCODED_RUSTFLAGS: "",
  },
  stdio: ["ignore", "pipe", "pipe"],
});

const markOutput = () => {
  lastOutputAt = Date.now();
};

child.stdout.on("data", (chunk) => {
  markOutput();
  process.stdout.write(chunk);
});

child.stderr.on("data", (chunk) => {
  markOutput();
  process.stderr.write(chunk);
});

child.on("error", (error) => {
  clearInterval(heartbeat);
  console.error(`[${bundle.label}] failed to start ${command}: ${error.message}`);
  process.exit(1);
});

const heartbeat = setInterval(() => {
  const now = Date.now();
  const quietSeconds = Math.round((now - lastOutputAt) / 1000);
  const totalSeconds = Math.round((now - startedAt) / 1000);
  console.log(
    `[${bundle.label}] still running (${totalSeconds}s elapsed, ${quietSeconds}s since last output)`,
  );
}, HEARTBEAT_MS);

const forwardSignal = (signal) => {
  if (!child.killed) {
    child.kill(signal);
  }
};

function runWasmOpt() {
  const wasmPath = path.join(bundle.outDir, "scena_bg.wasm");
  const optimizedPath = `${wasmPath}.opt`;
  const wasmOptCommand = path.join(
    "node_modules",
    ".bin",
    process.platform === "win32" ? "wasm-opt.cmd" : "wasm-opt",
  );
  console.log(
    `[${bundle.label}] running: ${wasmOptCommand} -Oz --strip-debug --strip-dwarf --strip-producers ${wasmPath}`,
  );
  const result = spawnSync(
    wasmOptCommand,
    [
      "-Oz",
      "--strip-debug",
      "--strip-dwarf",
      "--strip-producers",
      wasmPath,
      "-o",
      optimizedPath,
    ],
    {
      cwd: process.cwd(),
      stdio: "inherit",
    },
  );
  if (result.error) {
    console.error(`[${bundle.label}] failed to start wasm-opt: ${result.error.message}`);
    return false;
  }
  if (result.status !== 0) {
    console.error(`[${bundle.label}] wasm-opt exited with ${result.status}`);
    return false;
  }
  fs.renameSync(optimizedPath, wasmPath);
  return true;
}

function writeSizeManifest() {
  const wasmPath = path.join(bundle.outDir, "scena_bg.wasm");
  const manifestPath = `${wasmPath}.size.json`;
  const bytes = fs.readFileSync(wasmPath);
  const brotli = zlib.brotliCompressSync(bytes, {
    params: {
      [zlib.constants.BROTLI_PARAM_QUALITY]: 11,
    },
  });
  const manifest = {
    bundle: bundle.name,
    features: bundle.features,
    wasm: "scena_bg.wasm",
    raw_bytes: bytes.length,
    brotli_quality: 11,
    brotli_bytes: brotli.length,
  };
  fs.writeFileSync(`${manifestPath}.tmp`, `${JSON.stringify(manifest, null, 2)}\n`);
  fs.renameSync(`${manifestPath}.tmp`, manifestPath);
  console.log(
    `[${bundle.label}] size raw=${manifest.raw_bytes} brotli=${manifest.brotli_bytes} manifest=${manifestPath}`,
  );
}

process.on("SIGINT", () => forwardSignal("SIGINT"));
process.on("SIGTERM", () => forwardSignal("SIGTERM"));

child.on("close", (code, signal) => {
  clearInterval(heartbeat);
  const totalSeconds = Math.round((Date.now() - startedAt) / 1000);
  if (signal) {
    console.error(`[${bundle.label}] ${command} terminated by ${signal} after ${totalSeconds}s`);
    process.exit(1);
  }
  console.log(`[${bundle.label}] ${command} exited with ${code} after ${totalSeconds}s`);
  if (code !== 0) {
    process.exit(code ?? 1);
  }
  if (!runWasmOpt()) {
    process.exit(1);
  }
  writeSizeManifest();
  process.exit(0);
});

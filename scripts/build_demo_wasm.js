#!/usr/bin/env node

const { spawn } = require("child_process");

const HEARTBEAT_MS = Number(process.env.SCENA_BUILD_HEARTBEAT_MS || 20_000);
const command = process.platform === "win32" ? "wasm-pack.cmd" : "wasm-pack";
const args = [
  "build",
  "--release",
  "--target",
  "web",
  "--out-dir",
  "demo/pkg",
  ".",
  "--features",
  "demo-page",
];

let lastOutputAt = Date.now();
const startedAt = lastOutputAt;

console.log(`[scena-demo-build] running: ${command} ${args.join(" ")}`);

const child = spawn(command, args, {
  cwd: process.cwd(),
  env: process.env,
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
  console.error(`[scena-demo-build] failed to start ${command}: ${error.message}`);
  process.exit(1);
});

const heartbeat = setInterval(() => {
  const now = Date.now();
  const quietSeconds = Math.round((now - lastOutputAt) / 1000);
  const totalSeconds = Math.round((now - startedAt) / 1000);
  console.log(
    `[scena-demo-build] still running (${totalSeconds}s elapsed, ${quietSeconds}s since last output)`,
  );
}, HEARTBEAT_MS);

const forwardSignal = (signal) => {
  if (!child.killed) {
    child.kill(signal);
  }
};

process.on("SIGINT", () => forwardSignal("SIGINT"));
process.on("SIGTERM", () => forwardSignal("SIGTERM"));

child.on("close", (code, signal) => {
  clearInterval(heartbeat);
  const totalSeconds = Math.round((Date.now() - startedAt) / 1000);
  if (signal) {
    console.error(`[scena-demo-build] ${command} terminated by ${signal} after ${totalSeconds}s`);
    process.exit(1);
  }
  console.log(`[scena-demo-build] ${command} exited with ${code} after ${totalSeconds}s`);
  process.exit(code ?? 1);
});

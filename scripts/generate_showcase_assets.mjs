#!/usr/bin/env node

import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";

const materials = [
  "matte",
  "plastic",
  "metal",
  "rough_metal",
  "chrome",
  "brushed_steel",
  "clearcoat_plastic",
  "satin",
  "leather",
  "clear_glass",
  "frosted_glass",
  "rubber",
];

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with ${result.status}`);
  }
}

function magick(...args) {
  run("magick", args);
}

mkdirSync("demo/assets/showcase/materials", { recursive: true });
mkdirSync("demo/assets/showcase/models", { recursive: true });
mkdirSync("demo/assets/showcase/easy", { recursive: true });

for (const preset of materials) {
  magick(
    `tests/visual/references/round_e/${preset}.png`,
    "-resize",
    "360x240^",
    "-gravity",
    "center",
    "-extent",
    "360x240",
    `demo/assets/showcase/materials/${preset}.png`,
  );
}

magick(
  "docs/assets/easy-scene-showcase/hero-connector-assembly.jpg",
  "-resize",
  "360x240^",
  "-gravity",
  "center",
  "-extent",
  "360x240",
  "demo/assets/showcase/models/connector.png",
);

const easy = [
  ["docs/assets/easy-scene-showcase/lens-presets.jpg", "camera-portrait.png"],
  ["docs/assets/easy-scene-showcase/light-presets.jpg", "key-light.png"],
  ["docs/assets/easy-scene-showcase/environment-presets.jpg", "environment-studio.png"],
  ["docs/assets/easy-scene-showcase/background-presets.jpg", "background-dark-studio.png"],
  ["docs/assets/easy-scene-showcase/auto-exposure-presets.jpg", "auto-exposure-product.png"],
  ["docs/assets/easy-scene-showcase/material-chrome.png", "material-chrome.png"],
];

for (const [source, output] of easy) {
  magick(
    source,
    "-resize",
    "320x200",
    "-background",
    "#dfe5e4",
    "-gravity",
    "center",
    "-extent",
    "320x200",
    `demo/assets/showcase/easy/${output}`,
  );
}

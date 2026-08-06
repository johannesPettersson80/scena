#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const API =
  "https://ambientcg.com/api/v3/assets?type=material&limit=500&include=title,tags,technique,maps,downloads";
const OUTPUT =
  process.argv[2] ||
  path.join(
    __dirname,
    "..",
    "src",
    "assets",
    "material_library",
    "catalog_snapshot.json",
  );
const FAMILY =
  /^(?:Metal(?:[0-9]|Plates|Walkway)|Plastic|Fabric|Leather|Rubber)/;
const REQUIRED_MAPS = ["color", "normal", "roughness"];

async function page(offset) {
  const response = await fetch(`${API}&offset=${offset}`, {
    headers: { "User-Agent": "scena material-library catalog updater" },
  });
  if (!response.ok) {
    throw new Error(`ambientCG API returned HTTP ${response.status}`);
  }
  return response.json();
}

async function main() {
  const first = await page(0);
  const pages = [first];
  for (let offset = 500; offset < first.totalResults; offset += 500) {
    pages.push(await page(offset));
  }
  const assets = pages.flatMap((result) => result.assets);
  if (assets.length !== first.totalResults) {
    throw new Error(
      `ambientCG result count mismatch: expected ${first.totalResults}, received ${assets.length}`,
    );
  }

  const entries = assets
    .filter((asset) => FAMILY.test(asset.id))
    .map((asset) => {
      const requiredMaps =
        asset.id.startsWith("Metal") ?
          [...REQUIRED_MAPS, "metalness"]
        : REQUIRED_MAPS;
      for (const map of requiredMaps) {
        if (!asset.maps.includes(map)) {
          throw new Error(`${asset.id} is missing required ${map} map`);
        }
      }
      const archive = asset.downloads.find(
        (download) =>
          download.attributes === "1K-JPG" && download.extension === "zip",
      );
      if (!archive) {
        throw new Error(`${asset.id} has no 1K-JPG ZIP`);
      }
      return {
        provider_asset_id: asset.id,
        label: asset.title,
        creation_method: asset.technique,
        tags: asset.tags,
        archive_uri: archive.url,
        archive_bytes: archive.size,
      };
    })
    .sort((left, right) =>
      left.provider_asset_id.localeCompare(right.provider_asset_id, "en", {
        numeric: true,
      }),
    );

  const snapshot = {
    schema: "scena.material_library_provider_snapshot.v1",
    provider: "ambientcg",
    provider_api: API,
    retrieved_utc: new Date().toISOString(),
    provider_material_count: first.totalResults,
    entries,
  };
  fs.writeFileSync(OUTPUT, `${JSON.stringify(snapshot, null, 2)}\n`);
  process.stdout.write(
    `wrote ${entries.length} audited product/industrial materials to ${OUTPUT}\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});

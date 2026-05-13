#!/usr/bin/env python3
"""Build a local visual review gallery from scena gate artifacts.

The script intentionally has no third-party Python dependencies. It converts
binary PPM (P6) artifacts to PNG using the Python standard library, copies PNG
artifacts, records basic image sanity metrics, and writes both an HTML gallery
and a JSON manifest under target/release-readiness/gallery/.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import html
import json
import os
from pathlib import Path
import shutil
import struct
import time
import zlib


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def read_ppm(path: Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    index = 0

    def token() -> bytes:
        nonlocal index
        while index < len(data) and data[index] in b" \t\r\n":
            index += 1
        if index < len(data) and data[index] == ord("#"):
            while index < len(data) and data[index] not in b"\r\n":
                index += 1
            return token()
        start = index
        while index < len(data) and data[index] not in b" \t\r\n":
            index += 1
        return data[start:index]

    magic = token()
    if magic != b"P6":
        raise ValueError(f"{path} is not a binary PPM/P6 file")
    width = int(token())
    height = int(token())
    max_value = int(token())
    if max_value != 255:
        raise ValueError(f"{path} uses unsupported PPM max value {max_value}")
    while index < len(data) and data[index] in b" \t\r\n":
        index += 1
    expected = width * height * 3
    pixels = data[index : index + expected]
    if len(pixels) != expected:
        raise ValueError(f"{path} has {len(pixels)} pixel bytes, expected {expected}")
    return width, height, pixels


def write_png_rgb(path: Path, width: int, height: int, rgb: bytes) -> None:
    def chunk(kind: bytes, payload: bytes) -> bytes:
        checksum = zlib.crc32(kind)
        checksum = zlib.crc32(payload, checksum) & 0xFFFFFFFF
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", checksum)

    rows = []
    stride = width * 3
    for y in range(height):
        rows.append(b"\x00" + rgb[y * stride : (y + 1) * stride])
    raw = b"".join(rows)
    payload = [
        b"\x89PNG\r\n\x1a\n",
        chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)),
        chunk(b"IDAT", zlib.compress(raw, level=9)),
        chunk(b"IEND", b""),
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"".join(payload))


def parse_png_size(path: Path) -> tuple[int | None, int | None]:
    data = path.read_bytes()[:32]
    if data.startswith(b"\x89PNG\r\n\x1a\n") and data[12:16] == b"IHDR":
        width, height = struct.unpack(">II", data[16:24])
        return width, height
    return None, None


def ppm_metrics(width: int, height: int, rgb: bytes) -> dict[str, object]:
    colors = set()
    non_black = 0
    non_white = 0
    luma_min = 255
    luma_max = 0
    row_hashes = set()
    col_hashes = set()
    stride = width * 3
    for y in range(height):
        row = rgb[y * stride : (y + 1) * stride]
        row_hashes.add(hashlib.sha256(row).digest())
        for x in range(width):
            offset = y * stride + x * 3
            r, g, b = rgb[offset], rgb[offset + 1], rgb[offset + 2]
            colors.add((r, g, b))
            if (r, g, b) != (0, 0, 0):
                non_black += 1
            if (r, g, b) != (255, 255, 255):
                non_white += 1
            luma = int(0.2126 * r + 0.7152 * g + 0.0722 * b)
            luma_min = min(luma_min, luma)
            luma_max = max(luma_max, luma)
    for x in range(width):
        h = hashlib.sha256()
        for y in range(height):
            offset = y * stride + x * 3
            h.update(rgb[offset : offset + 3])
        col_hashes.add(h.digest())
    return {
        "width": width,
        "height": height,
        "distinct_rgb": len(colors),
        "non_black_pixels": non_black,
        "non_white_pixels": non_white,
        "luma_min": luma_min,
        "luma_max": luma_max,
        "unique_rows": len(row_hashes),
        "unique_columns": len(col_hashes),
        "constant_image": len(colors) <= 1,
    }


def review_focus(rel: str) -> str:
    name = Path(rel).stem
    if "cad_plate" in name:
        return "CAD proof: confirm a flat 120 mm x 60 mm plate, correct aspect ratio, visible framing, and no unit/axis swap."
    if "waterbottle" in name:
        return "Real-asset proof: compare silhouette, bottle body material, cap/label/floor tones, and normal-map surface detail against the bundled references."
    if "industrial_connector_assembly" in name:
        return "Industrial assembly proof: confirm connector alignment, assembly spacing, visible materials, and no misplaced imported hierarchy."
    if "industrial_static_scene" in name:
        return "Industrial scene proof: confirm the model reads as a structured industrial visualization rather than flat primitives."
    if "glb_model_viewer" in name:
        return "glTF viewer proof: confirm imported geometry is visible, centered, and not a placeholder triangle."
    if "animation" in name:
        return "Animation proof: confirm morphed/skinned pose is visible and not the bind/rest pose only."
    if "anchor" in name or "connector" in name:
        return "Anchor/connector proof: confirm markers or connected parts align exactly without sideways rotation."
    if "texture" in name or "material" in rel or "m8-" in name:
        return "Material proof: confirm texture roles, color variation, lighting response, and no all-black/all-white fallback."
    if "pbr" in name or "shadow" in name:
        return "Lighting proof: confirm the named light/shadow contribution is visible and non-trivial."
    if "layers" in name or "picking" in name:
        return "Interaction proof: confirm the highlighted/selected/layer-visible object is unambiguous."
    if "primitive" in name:
        return "Primitive proof: confirm multiple shape types are visible and spatially separated."
    return "General proof: confirm non-empty rendered content, correct framing, and no placeholder/flat-color output."


def collect_artifacts(artifact_root: Path, output: Path, repo: Path) -> list[dict[str, object]]:
    images_dir = output / "images"
    entries: list[dict[str, object]] = []
    candidates = sorted(
        path
        for path in artifact_root.rglob("*")
        if path.is_file() and path.suffix.lower() in {".ppm", ".png"}
    )

    reference_candidates = [
        repo / "tests/assets/gltf/khronos/WaterBottle/reference_blender_cycles_512.png",
        repo / "tests/assets/gltf/khronos/WaterBottle/reference_512.png",
    ]
    for ref in reference_candidates:
        if ref.is_file():
            candidates.append(ref)

    for source in candidates:
        rel = source.relative_to(repo).as_posix() if source.is_relative_to(repo) else source.name
        safe_name = rel.replace("/", "__").replace(" ", "_")
        source_sha = sha256(source)
        metrics: dict[str, object]
        if source.suffix.lower() == ".ppm":
            width, height, pixels = read_ppm(source)
            review_path = images_dir / f"{safe_name}.png"
            write_png_rgb(review_path, width, height, pixels)
            metrics = ppm_metrics(width, height, pixels)
            converted = True
        else:
            width, height = parse_png_size(source)
            review_path = images_dir / safe_name
            review_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, review_path)
            metrics = {
                "width": width,
                "height": height,
                "distinct_rgb": None,
                "non_black_pixels": None,
                "non_white_pixels": None,
                "luma_min": None,
                "luma_max": None,
                "unique_rows": None,
                "unique_columns": None,
                "constant_image": None,
            }
            converted = False
        entries.append(
            {
                "source": rel,
                "source_sha256": source_sha,
                "review_image": review_path.relative_to(output).as_posix(),
                "review_image_sha256": sha256(review_path),
                "converted_from_ppm": converted,
                "metrics": metrics,
                "review_focus": review_focus(rel),
            }
        )
    return entries


def write_html(output: Path, entries: list[dict[str, object]]) -> None:
    cards = []
    for entry in entries:
        metrics = entry["metrics"]
        metrics_text = ", ".join(
            f"{html.escape(str(k))}: {html.escape(str(v))}" for k, v in metrics.items()
        )
        cards.append(
            f"""
            <article class="card">
              <a href="{html.escape(entry['review_image'])}"><img src="{html.escape(entry['review_image'])}" loading="lazy"></a>
              <h2>{html.escape(entry['source'])}</h2>
              <p><strong>Review:</strong> {html.escape(entry['review_focus'])}</p>
              <p><strong>Metrics:</strong> {metrics_text}</p>
              <p><strong>Source SHA-256:</strong> <code>{html.escape(entry['source_sha256'])}</code></p>
              <p><strong>Review image SHA-256:</strong> <code>{html.escape(entry['review_image_sha256'])}</code></p>
            </article>
            """
        )
    html_text = f"""<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>scena local release visual gallery</title>
<style>
body {{ margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; background: #101314; color: #eef2ef; }}
header {{ padding: 32px; background: linear-gradient(135deg, #203b31, #101314); }}
main {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 18px; padding: 18px; }}
.card {{ background: #181d1e; border: 1px solid #34413d; border-radius: 14px; padding: 14px; }}
img {{ width: 100%; image-rendering: auto; background: #050606; border-radius: 10px; border: 1px solid #2c3333; }}
h1 {{ margin: 0 0 8px; }}
h2 {{ font-size: 15px; word-break: break-word; }}
p {{ font-size: 13px; line-height: 1.4; }}
code {{ word-break: break-all; color: #b5f0cc; }}
</style>
<header>
  <h1>scena local release visual gallery</h1>
  <p>Generated at {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}. Open each image and compare it against the review note.</p>
</header>
<main>
{''.join(cards)}
</main>
</html>
"""
    (output / "index.html").write_text(html_text)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", default=".", type=Path)
    parser.add_argument("--artifact-root", default="target/gate-artifacts", type=Path)
    parser.add_argument("--output", default="target/release-readiness/gallery", type=Path)
    args = parser.parse_args()

    repo = args.repo.resolve()
    artifact_root = (repo / args.artifact_root).resolve()
    output = (repo / args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)

    entries = collect_artifacts(artifact_root, output, repo)
    manifest = {
        "schema": "scena.local_release_visual_gallery.v1",
        "generated_at_unix": int(time.time()),
        "artifact_root": artifact_root.relative_to(repo).as_posix(),
        "entry_count": len(entries),
        "entries": entries,
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    write_html(output, entries)
    print(f"gallery: {output / 'index.html'}")
    print(f"manifest: {output / 'manifest.json'}")
    print(f"images: {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

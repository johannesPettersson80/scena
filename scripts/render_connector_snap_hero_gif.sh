#!/usr/bin/env bash
set -euo pipefail

out_dir="target/gate-artifacts/connector-snap-hero"
frames_dir="${out_dir}/frames"
review_frames_dir="${out_dir}/review-frames"
palette="${out_dir}/connector-snap-palette.png"
gif="${out_dir}/connector-snap.gif"

overlay=$'let assets = Assets::new();\nlet drive_part = assets.load_scene("drive_unit.glb").await?;\nlet load_part  = assets.load_scene("load_unit.glb").await?;\nlet mut scene = Scene::new();\nlet drive = scene.instantiate(&drive_part)?;\nlet load  = scene.instantiate(&load_part)?;\nscene.mate(&drive, "shaft", &load, "hub")?;'

mkdir -p "${review_frames_dir}"

for frame in $(seq -f "%03g" 0 79); do
  src="${frames_dir}/frame_${frame}.png"
  dst="${review_frames_dir}/frame_${frame}.png"
  if ((10#${frame} >= 65)); then
    magick "${src}" \
      -fill '#0f1115ee' -draw 'rectangle 616,70 1258,316' \
      -font DejaVu-Sans-Mono -pointsize 15 -fill '#f4f0e8' \
      -annotate +644+102 "${overlay}" \
      "PNG32:${dst}"
  else
    magick "${src}" "PNG32:${dst}"
  fi
done

ffmpeg -y -framerate 10 -i "${review_frames_dir}/frame_%03d.png" \
  -vf "palettegen=stats_mode=full:max_colors=256" "${palette}"
ffmpeg -y -framerate 10 -i "${review_frames_dir}/frame_%03d.png" -i "${palette}" \
  -lavfi "paletteuse=dither=floyd_steinberg:diff_mode=rectangle" \
  -loop 0 "${gif}"

printf 'wrote %s\n' "${gif}"

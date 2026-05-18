#!/usr/bin/env bash
set -euo pipefail

out_dir="target/gate-artifacts/connector-snap-hero"
frames_dir="${out_dir}/frames"
review_frames_dir="${out_dir}/review-frames"
palette="${out_dir}/connector-snap-palette.png"
gif="${out_dir}/connector-snap.gif"

overlay_lines=(
  'let assets = Assets::new();'
  'let drive_part = assets.load_scene("drive_unit.glb").await?;'
  'let load_part  = assets.load_scene("load_unit.glb").await?;'
  ''
  'let mut scene = Scene::new();'
  'let drive = scene.instantiate(&drive_part)?;'
  'let load  = scene.instantiate(&load_part)?;'
  ''
  'scene.mate(&drive, "shaft", &load, "hub")?;'
)

mkdir -p "${review_frames_dir}"

for frame in $(seq -f "%03g" 0 79); do
  src="${frames_dir}/frame_${frame}.png"
  dst="${review_frames_dir}/frame_${frame}.png"
  if ((10#${frame} >= 65)); then
    draw_args=(
      -fill '#0f1115ee' -draw 'rectangle 674,58 1258,344'
      -font DejaVu-Sans-Mono -pointsize 14 -fill '#f4f0e8'
    )
    y=90
    for line in "${overlay_lines[@]}"; do
      if [[ -n "${line}" ]]; then
        draw_args+=(-annotate "+704+${y}" "${line}")
      fi
      y=$((y + 25))
    done
    magick "${src}" \
      "${draw_args[@]}" \
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

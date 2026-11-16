// scena.fxaa_resolve.wgsl
//
// Fast approximate anti-aliasing after Timothy Lottes' FXAA 3.11 quality
// path, shared by the FXAA pass and the combined bloom + FXAA surface pass.
// The including shader declares `source_texture` and the `post` uniform.
//
// The pass runs before the display transform, on scene-linear radiance, so
// two things differ from the reference:
//
// - Luma is display-referred: exposed luminance clamped to display white and
//   taken to gamma 2.0. FXAA's thresholds assume a perceptual luma, and a step
//   between two values that both display as white is not a visible edge.
// - Every sample is a texel load. The reference lets bilinear filtering
//   average the two texels across the edge, which is only an average of
//   perceptual luma on an encoded target; here each pair is loaded and
//   averaged after the luma transform.

// Edges below this share of the local maximum luma are left alone.
const FXAA_EDGE_THRESHOLD: f32 = 0.166;
// Edges below this absolute luma contrast are left alone.
const FXAA_EDGE_THRESHOLD_MIN: f32 = 0.0625;
// Strength of the sub-pixel (single-pixel contrast) blend.
const FXAA_SUBPIXEL_QUALITY: f32 = 0.75;
// Steps walked along the edge in each direction; the table below sets how far
// each one advances, so the search reaches 26 pixels.
const FXAA_SEARCH_STEPS: i32 = 12;

fn fxaa_load(coord: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    return textureLoad(source_texture, clamp(coord, vec2<i32>(0), dims - vec2<i32>(1)), 0);
}

fn fxaa_luma(rgb: vec3<f32>, exposure: f32) -> f32 {
    let luminance = dot(max(rgb, vec3<f32>(0.0)), vec3<f32>(0.2126, 0.7152, 0.0722));
    return sqrt(clamp(luminance * exposure, 0.0, 1.0));
}

fn fxaa_luma_at(coord: vec2<i32>, dims: vec2<i32>, exposure: f32) -> f32 {
    return fxaa_luma(fxaa_load(coord, dims).rgb, exposure);
}

// Mean luma of the texel at `coord` and its neighbour across the edge.
fn fxaa_pair_luma(coord: vec2<i32>, across: vec2<i32>, dims: vec2<i32>, exposure: f32) -> f32 {
    return 0.5 * (fxaa_luma_at(coord, dims, exposure) + fxaa_luma_at(coord + across, dims, exposure));
}

// Weight that blends two radiance values as the display will show them: a
// value brighter than display white counts as white, so a highlight cannot
// swamp its neighbour. In-range values keep weight one and blend linearly.
fn fxaa_display_weight(rgb: vec3<f32>, exposure: f32) -> f32 {
    return 1.0 / max(1.0, max(rgb.r, max(rgb.g, rgb.b)) * exposure);
}

fn fxaa_blend(near: vec4<f32>, far: vec4<f32>, amount: f32, exposure: f32) -> vec4<f32> {
    let near_weight = fxaa_display_weight(near.rgb, exposure) * (1.0 - amount);
    let far_weight = fxaa_display_weight(far.rgb, exposure) * amount;
    let rgb = (near.rgb * near_weight + far.rgb * far_weight) / (near_weight + far_weight);
    return vec4<f32>(rgb, mix(near.a, far.a, amount));
}

fn fxaa_resolve(coord: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    // `post.config.y` carries the display exposure scale; zero means unset.
    let exposure = select(1.0, post.config.y, post.config.y > 0.0);
    let center = fxaa_load(coord, dims);
    let luma_m = fxaa_luma(center.rgb, exposure);
    let luma_n = fxaa_luma_at(coord + vec2<i32>(0, -1), dims, exposure);
    let luma_s = fxaa_luma_at(coord + vec2<i32>(0, 1), dims, exposure);
    let luma_w = fxaa_luma_at(coord + vec2<i32>(-1, 0), dims, exposure);
    let luma_e = fxaa_luma_at(coord + vec2<i32>(1, 0), dims, exposure);

    let luma_max = max(luma_m, max(max(luma_n, luma_s), max(luma_w, luma_e)));
    let luma_min = min(luma_m, min(min(luma_n, luma_s), min(luma_w, luma_e)));
    let luma_range = luma_max - luma_min;
    if luma_range < max(FXAA_EDGE_THRESHOLD_MIN, luma_max * FXAA_EDGE_THRESHOLD) {
        return center;
    }

    let luma_nw = fxaa_luma_at(coord + vec2<i32>(-1, -1), dims, exposure);
    let luma_ne = fxaa_luma_at(coord + vec2<i32>(1, -1), dims, exposure);
    let luma_sw = fxaa_luma_at(coord + vec2<i32>(-1, 1), dims, exposure);
    let luma_se = fxaa_luma_at(coord + vec2<i32>(1, 1), dims, exposure);

    // Edge orientation from the second differences of the 3x3 neighbourhood.
    let luma_ns = luma_n + luma_s;
    let luma_we = luma_w + luma_e;
    let edge_horizontal = abs(luma_nw + luma_sw - 2.0 * luma_w)
        + 2.0 * abs(luma_ns - 2.0 * luma_m)
        + abs(luma_ne + luma_se - 2.0 * luma_e);
    let edge_vertical = abs(luma_nw + luma_ne - 2.0 * luma_n)
        + 2.0 * abs(luma_we - 2.0 * luma_m)
        + abs(luma_sw + luma_se - 2.0 * luma_s);
    let is_horizontal = edge_horizontal >= edge_vertical;

    // The edge lies between this pixel and the neighbour with the steeper
    // luma step: north or south of a horizontal edge, west or east otherwise.
    let luma_before = select(luma_w, luma_n, is_horizontal);
    let luma_after = select(luma_e, luma_s, is_horizontal);
    let gradient_before = abs(luma_before - luma_m);
    let gradient_after = abs(luma_after - luma_m);
    let steeper_before = gradient_before >= gradient_after;
    let side = select(1, -1, steeper_before);
    let across = select(vec2<i32>(side, 0), vec2<i32>(0, side), is_horizontal);
    let along = select(vec2<i32>(0, 1), vec2<i32>(1, 0), is_horizontal);
    let luma_edge = 0.5 * (select(luma_after, luma_before, steeper_before) + luma_m);
    let gradient_scaled = 0.25 * max(gradient_before, gradient_after);
    let center_is_darker = luma_m < luma_edge;

    // Walk both ways along the edge until the pair across it stops straddling
    // the edge luma: that is where this run of the staircase ends.
    let step_sizes = array<i32, 12>(1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 8);
    var reach = 0;
    var distance_back = 0.0;
    var distance_forward = 0.0;
    var end_back = 0.0;
    var end_forward = 0.0;
    var done_back = false;
    var done_forward = false;
    for (var step = 0; step < FXAA_SEARCH_STEPS; step = step + 1) {
        reach = reach + step_sizes[step];
        if !done_back {
            end_back = fxaa_pair_luma(coord - along * reach, across, dims, exposure) - luma_edge;
            distance_back = f32(reach);
            done_back = abs(end_back) >= gradient_scaled;
        }
        if !done_forward {
            end_forward = fxaa_pair_luma(coord + along * reach, across, dims, exposure) - luma_edge;
            distance_forward = f32(reach);
            done_forward = abs(end_forward) >= gradient_scaled;
        }
        if done_back && done_forward {
            break;
        }
    }

    // Blend toward the neighbour by how close the nearer end of the run is,
    // but only when the run ends by stepping away from this pixel's side.
    let back_is_nearer = distance_back < distance_forward;
    let nearest = min(distance_back, distance_forward);
    let end_nearest = select(end_forward, end_back, back_is_nearer);
    let run_turns_away = (end_nearest < 0.0) != center_is_darker;
    let edge_blend = select(
        0.0,
        0.5 - nearest / (distance_back + distance_forward),
        run_turns_away,
    );

    // Sub-pixel term: how far the pixel sits from its low-passed neighbourhood.
    let neighbourhood = (2.0 * (luma_ns + luma_we) + luma_nw + luma_ne + luma_sw + luma_se) / 12.0;
    let contrast = clamp(abs(neighbourhood - luma_m) / luma_range, 0.0, 1.0);
    let smoothed = (3.0 - 2.0 * contrast) * contrast * contrast;
    let subpixel_blend = smoothed * smoothed * FXAA_SUBPIXEL_QUALITY;

    let amount = max(edge_blend, subpixel_blend);
    return fxaa_blend(center, fxaa_load(coord + across, dims), amount, exposure);
}

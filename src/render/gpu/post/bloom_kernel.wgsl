// scena.bloom_kernel.wgsl
//
// Bloom threshold and blur width shared by the bloom passes and the combined
// bloom + FXAA surface pass. The including shader declares `source_texture`
// and the `post` uniform:
//
//   viewport.z  linear luminance threshold
//   viewport.w  intensity
//   config.x    radius in target pixels
//   config.z    knee, as a share of the threshold

// Largest radius the passes accept, matching `PostBloomConfig`.
const BLOOM_MAX_RADIUS: f32 = 12.0;

// What a pixel contributes to bloom. Nothing at or below the threshold
// contributes. The share then rises smoothly across the knee, a band above
// the threshold `knee` times the threshold wide, and is the whole colour
// beyond it. A zero knee is a hard cut.
fn bloom_bright(rgb: vec3<f32>) -> vec3<f32> {
    let threshold = max(post.viewport.z, 0.0);
    let knee_end = threshold * (1.0 + clamp(post.config.z, 0.0, 1.0));
    let color = max(rgb, vec3<f32>(0.0));
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    if knee_end <= threshold {
        return color * step(threshold, luminance);
    }
    return color * smoothstep(threshold, knee_end, luminance);
}

// Standard deviation of the blur in pixels. The Gaussian has the variance of
// a box filter reaching `radius` pixels each way, which is the filter the CPU
// renderer applies, so both spread a highlight equally far.
fn bloom_sigma() -> f32 {
    let radius = clamp(post.config.x, 0.0, BLOOM_MAX_RADIUS);
    return sqrt(radius * (radius + 1.0) / 3.0);
}

fn bloom_weight(distance: f32, sigma: f32) -> f32 {
    return exp(-0.5 * distance * distance / (sigma * sigma));
}

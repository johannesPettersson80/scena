// scena.color_contract.wgsl
//
// Shared display-transform contract for GPU/WebGPU/WebGL2 shaders. The Rust
// reference implementation lives in `src/render/color_contract.rs`; keep the
// constants and formulas byte-equivalent there and here.

const SCENA_PBR_NEUTRAL_F90: f32 = 0.04;
const SCENA_PBR_NEUTRAL_START_COMPRESSION: f32 = 0.8 - SCENA_PBR_NEUTRAL_F90;
const SCENA_PBR_NEUTRAL_DESATURATION: f32 = 0.15;

fn apply_tonemapper(color: vec3<f32>, color_management_mode: f32) -> vec3<f32> {
    if color_management_mode < 0.5 {
        return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    if color_management_mode > 1.5 {
        return pbr_neutral_tonemap(color);
    }
    return aces_tonemap(color);
}

fn encode_post_target_rgb(color: vec3<f32>, output_transfer_mode: f32) -> vec3<f32> {
    if output_transfer_mode <= 0.5 {
        return color;
    }
    return linear_to_srgb(color);
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb_channel(color.r),
        linear_to_srgb_channel(color.g),
        linear_to_srgb_channel(color.b),
    );
}

fn linear_to_srgb_channel(channel: f32) -> f32 {
    let value = clamp(channel, 0.0, 1.0);
    if value <= 0.0031308 {
        return value * 12.92;
    }
    return 1.055 * pow(value, 1.0 / 2.4) - 0.055;
}

fn srgb_to_linear_channel(channel: f32) -> f32 {
    let value = clamp(channel, 0.0, 1.0);
    if value <= 0.04045 {
        return value / 12.92;
    }
    return pow((value + 0.055) / 1.055, 2.4);
}

fn apply_srgb8_dither(color: vec3<f32>, noise_lsb: f32) -> vec3<f32> {
    let encoded = linear_to_srgb(color) + vec3<f32>(noise_lsb / 255.0);
    return vec3<f32>(
        srgb_to_linear_channel(encoded.r),
        srgb_to_linear_channel(encoded.g),
        srgb_to_linear_channel(encoded.b),
    );
}

fn pbr_neutral_tonemap(color_in: vec3<f32>) -> vec3<f32> {
    var color = max(color_in, vec3<f32>(0.0));
    let x = min(color.r, min(color.g, color.b));
    let offset = select(
        SCENA_PBR_NEUTRAL_F90,
        x - (x * x) / (4.0 * SCENA_PBR_NEUTRAL_F90),
        x < 2.0 * SCENA_PBR_NEUTRAL_F90,
    );
    color -= vec3<f32>(offset);
    let peak = max(color.r, max(color.g, color.b));
    if peak < SCENA_PBR_NEUTRAL_START_COMPRESSION {
        return color;
    }
    let compression_range = 1.0 - SCENA_PBR_NEUTRAL_START_COMPRESSION;
    let new_peak = 1.0 - compression_range * compression_range /
        (peak + compression_range - SCENA_PBR_NEUTRAL_START_COMPRESSION);
    color *= new_peak / peak;
    let desaturation_mix = 1.0 - 1.0 /
        (SCENA_PBR_NEUTRAL_DESATURATION * (peak - new_peak) + 1.0);
    return mix(color, new_peak * vec3<f32>(1.0), desaturation_mix);
}

fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let input = vec3<f32>(
        dot(vec3<f32>(0.59719, 0.35458, 0.04823), color),
        dot(vec3<f32>(0.076, 0.90834, 0.01566), color),
        dot(vec3<f32>(0.0284, 0.13383, 0.83777), color),
    );
    let fitted = vec3<f32>(
        rrt_and_odt_fit(input.r),
        rrt_and_odt_fit(input.g),
        rrt_and_odt_fit(input.b),
    );
    let output = vec3<f32>(
        dot(vec3<f32>(1.60475, -0.53108, -0.07367), fitted),
        dot(vec3<f32>(-0.10208, 1.10813, -0.00605), fitted),
        dot(vec3<f32>(-0.00327, -0.07276, 1.07602), fitted),
    );
    return clamp(output, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn rrt_and_odt_fit(value: f32) -> f32 {
    let numerator = value * (value + 0.0245786) - 0.000090537;
    let denominator = value * (0.983729 * value + 0.432951) + 0.238081;
    return numerator / denominator;
}

use super::super::prepare::PreparedGpuLightUniform;

pub(super) const GPU_TRIANGLE_SHADER: &str = r#"
const PI: f32 = 3.141592653589793;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tex_coord0: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) shadow_visibility: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord0: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) shadow_visibility: f32,
};

struct LightingUniform {
    directional_light_direction_intensity: vec4<f32>,
    directional_light_color_count: vec4<f32>,
    point_light_position_intensity: vec4<f32>,
    point_light_color_range: vec4<f32>,
    spot_light_position_intensity: vec4<f32>,
    spot_light_direction_cones: vec4<f32>,
    spot_light_cone_range: vec4<f32>,
    spot_light_color_range: vec4<f32>,
    environment_diffuse_intensity: vec4<f32>,
    environment_specular_intensity: vec4<f32>,
};

struct CameraUniform {
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    camera_position_exposure: vec4<f32>,
    viewport_near_far: vec4<f32>,
    color_management: vec4<f32>,
    lighting: LightingUniform,
};

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
};

struct MaterialUniform {
    base_color_uv_offset_scale: vec4<f32>,
    base_color_uv_rotation: vec4<f32>,
    base_color_factor: vec4<f32>,
    emissive_strength: vec4<f32>,
    metallic_roughness_alpha: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

@group(1) @binding(0)
var base_color_sampler: sampler;

@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(1) @binding(2)
var<uniform> material: MaterialUniform;

@group(1) @binding(3)
var normal_sampler: sampler;

@group(1) @binding(4)
var normal_texture: texture_2d<f32>;

@group(1) @binding(5)
var metallic_roughness_sampler: sampler;

@group(1) @binding(6)
var metallic_roughness_texture: texture_2d<f32>;

@group(1) @binding(7)
var occlusion_sampler: sampler;

@group(1) @binding(8)
var occlusion_texture: texture_2d<f32>;

@group(1) @binding(9)
var emissive_sampler: sampler;

@group(1) @binding(10)
var emissive_texture: texture_2d<f32>;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world_position = draw.world_from_model * vec4<f32>(in.position, 1.0);
    out.position = camera.clip_from_view * camera.view_from_world * world_position;
    out.color = in.color;
    out.normal = (draw.normal_from_model * vec4<f32>(in.normal, 0.0)).xyz;
    out.tex_coord0 = in.tex_coord0;
    out.world_position = world_position.xyz;
    out.tangent = vec4<f32>((draw.normal_from_model * vec4<f32>(in.tangent.xyz, 0.0)).xyz, in.tangent.w);
    out.shadow_visibility = clamp(in.shadow_visibility, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let scaled_uv = in.tex_coord0 * material.base_color_uv_offset_scale.zw;
    let transformed_uv = vec2<f32>(
        scaled_uv.x * material.base_color_uv_rotation.y - scaled_uv.y * material.base_color_uv_rotation.x,
        scaled_uv.x * material.base_color_uv_rotation.x + scaled_uv.y * material.base_color_uv_rotation.y,
    ) + material.base_color_uv_offset_scale.xy;
    let base_color_sample = textureSample(base_color_texture, base_color_sampler, transformed_uv);
    let normal_texture_sample = textureSample(normal_texture, normal_sampler, in.tex_coord0).rgb;
    let metallic_roughness_sample = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.tex_coord0);
    let occlusion_sample = textureSample(occlusion_texture, occlusion_sampler, in.tex_coord0).r;
    let emissive_sample = textureSample(emissive_texture, emissive_sampler, in.tex_coord0).rgb;
    let normal_sample = normalize(normal_texture_sample * 2.0 - vec3<f32>(1.0));
    let world_normal = normalize(in.normal);
    let world_tangent = normalize(in.tangent.xyz);
    let bitangent = normalize(cross(world_normal, world_tangent) * in.tangent.w);
    let normal = normalize(normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal);
    let normal_visibility = clamp(normal_sample.z, 0.2, 1.0);
    let metallic = clamp(material.metallic_roughness_alpha.x * metallic_roughness_sample.b, 0.0, 1.0);
    let roughness = clamp(material.metallic_roughness_alpha.y * metallic_roughness_sample.g, 0.04, 1.0);
    let material_response = normal_visibility * occlusion_sample * mix(0.92, 1.0, roughness) * (1.0 - metallic * 0.08);
    let base = in.color * material.base_color_factor * base_color_sample;
    if material.metallic_roughness_alpha.z > 0.0 && base.a < material.metallic_roughness_alpha.z {
        discard;
    }
    let emissive = material.emissive_strength.rgb * emissive_sample * material.emissive_strength.w;
    let view = normalize(camera.camera_position_exposure.xyz - in.world_position);
    var shaded_rgb = base.rgb;
    if material.metallic_roughness_alpha.w < 0.5 {
        shaded_rgb = base.rgb * material_response;
        let direct = pbr_punctual_lighting(
            base.rgb,
            metallic,
            roughness,
            normal,
            view,
            in.world_position,
            in.shadow_visibility,
        );
        let environment = pbr_environment_lighting(base.rgb, metallic, roughness, normal, view);
        if has_punctual_light() || has_environment_light() {
            shaded_rgb = (direct + environment) * occlusion_sample;
        }
    }
    let shaded = vec4<f32>(shaded_rgb + emissive, base.a);
    return vec4<f32>(aces_tonemap(shaded.rgb * camera.camera_position_exposure.w), shaded.a);
}

fn pbr_punctual_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    world_position: vec3<f32>,
    shadow_visibility: f32,
) -> vec3<f32> {
    var shaded = vec3<f32>(0.0);
    if camera.lighting.directional_light_direction_intensity.w > 0.0 {
        let incoming = normalize(-camera.lighting.directional_light_direction_intensity.xyz);
        let radiance = camera.lighting.directional_light_color_count.rgb *
            camera.lighting.directional_light_direction_intensity.w * shadow_visibility;
        shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    if camera.lighting.point_light_position_intensity.w > 0.0 {
        let to_light = camera.lighting.point_light_position_intensity.xyz - world_position;
        let incoming = normalize(to_light);
        let attenuation = distance_attenuation(to_light, camera.lighting.point_light_color_range.w);
        let radiance = camera.lighting.point_light_color_range.rgb *
            camera.lighting.point_light_position_intensity.w * attenuation;
        shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    if camera.lighting.spot_light_position_intensity.w > 0.0 {
        let to_light = camera.lighting.spot_light_position_intensity.xyz - world_position;
        let incoming = normalize(to_light);
        let to_surface = -incoming;
        let cone = spot_cone_attenuation(
            dot(to_surface, normalize(camera.lighting.spot_light_direction_cones.xyz)),
            camera.lighting.spot_light_cone_range.x,
            camera.lighting.spot_light_cone_range.y,
        );
        let attenuation = distance_attenuation(to_light, camera.lighting.spot_light_cone_range.z);
        let radiance = camera.lighting.spot_light_color_range.rgb *
            camera.lighting.spot_light_position_intensity.w * attenuation * cone;
        shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    return shaded;
}

fn has_punctual_light() -> bool {
    return camera.lighting.directional_light_direction_intensity.w > 0.0 ||
        camera.lighting.point_light_position_intensity.w > 0.0 ||
        camera.lighting.spot_light_position_intensity.w > 0.0;
}

fn has_environment_light() -> bool {
    return camera.lighting.environment_diffuse_intensity.w > 0.0 ||
        camera.lighting.environment_specular_intensity.w > 0.0;
}

fn pbr_environment_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
) -> vec3<f32> {
    if !has_environment_light() {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(n_dot_v, f0);
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    let diffuse = diffuse_energy * base * camera.lighting.environment_diffuse_intensity.rgb;
    let specular_strength = clamp(1.25 - roughness * 0.75, 0.2, 1.25);
    let specular = fresnel * camera.lighting.environment_specular_intensity.rgb * specular_strength;
    let intensity = max(
        camera.lighting.environment_diffuse_intensity.w,
        camera.lighting.environment_specular_intensity.w,
    );
    return (diffuse + specular) * intensity;
}

fn pbr_light_contribution(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
) -> vec3<f32> {
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let alpha = roughness * roughness;
    let distribution = distribution_ggx(n_dot_h, alpha);
    let geometry = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let specular = fresnel * (distribution * geometry / max(4.0 * n_dot_v * n_dot_l, 0.0001));
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    let diffuse = diffuse_energy * base / PI;
    return (diffuse + specular) * radiance * n_dot_l;
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha_squared = alpha * alpha;
    let denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / max(PI * denominator * denominator, 0.0001);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let k = ((roughness + 1.0) * (roughness + 1.0)) / 8.0;
    return geometry_schlick_ggx(n_dot_v, k) * geometry_schlick_ggx(n_dot_l, k);
}

fn geometry_schlick_ggx(n_dot: f32, k: f32) -> f32 {
    return n_dot / max(n_dot * (1.0 - k) + k, 0.0001);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

fn distance_attenuation(to_light: vec3<f32>, range: f32) -> f32 {
    if range <= 0.0 {
        return 1.0;
    }
    let distance = length(to_light);
    return pow(clamp(1.0 - distance / range, 0.0, 1.0), 2.0);
}

fn spot_cone_attenuation(cos_angle: f32, inner_cone_cos: f32, outer_cone_cos: f32) -> f32 {
    if cos_angle >= inner_cone_cos {
        return 1.0;
    }
    if cos_angle <= outer_cone_cos {
        return 0.0;
    }
    return clamp((cos_angle - outer_cone_cos) / (inner_cone_cos - outer_cone_cos), 0.0, 1.0);
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
"#;

pub(super) const OUTPUT_UNIFORM_BYTE_LEN: u64 = 400;

/// One DrawUniform entry packs world_from_model + normal_from_model = 32
/// floats = 128 bytes. WebGPU requires dynamic-offset uniform binding offsets
/// to be aligned to `min_uniform_buffer_offset_alignment`, which is 256 on
/// every wgpu adapter we target. We pad each entry up to 256 bytes so the
/// runtime stride matches the alignment requirement; the trailing 128 bytes
/// per entry are zero-padding.
pub(super) const DRAW_UNIFORM_ENTRY_SIZE: u64 = 128;
pub(super) const DRAW_UNIFORM_ENTRY_STRIDE: u64 = 256;

pub(super) fn create_draw_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.draw.bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: std::num::NonZeroU64::new(DRAW_UNIFORM_ENTRY_SIZE),
            },
            count: None,
        }],
    })
}

pub(super) fn create_draw_uniform_buffer(device: &wgpu::Device, entry_count: u64) -> wgpu::Buffer {
    let size = DRAW_UNIFORM_ENTRY_STRIDE.saturating_mul(entry_count.max(1));
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.draw.uniform"),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub(super) fn create_draw_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.draw.bind_group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: uniform,
                offset: 0,
                size: std::num::NonZeroU64::new(DRAW_UNIFORM_ENTRY_SIZE),
            }),
        }],
    })
}

/// Encodes a `Vec<DrawUniformValue>` into a packed byte buffer where each
/// entry occupies `DRAW_UNIFORM_ENTRY_STRIDE` bytes. The first
/// `DRAW_UNIFORM_ENTRY_SIZE` bytes of each entry hold the world_from_model +
/// normal_from_model matrices; the trailing bytes are zero padding required
/// by `min_uniform_buffer_offset_alignment` for dynamic-offset binding.
pub(super) fn encode_draw_uniform_bytes(
    values: &[(/*world*/ [f32; 16], /*normal*/ [f32; 16])],
) -> Vec<u8> {
    let mut bytes = vec![0u8; values.len().max(1) * DRAW_UNIFORM_ENTRY_STRIDE as usize];
    for (entry_index, (world_from_model, normal_from_model)) in values.iter().enumerate() {
        let entry_offset = entry_index * DRAW_UNIFORM_ENTRY_STRIDE as usize;
        for (i, value) in world_from_model.iter().enumerate() {
            let byte_offset = entry_offset + i * 4;
            bytes[byte_offset..byte_offset + 4].copy_from_slice(&value.to_ne_bytes());
        }
        for (i, value) in normal_from_model.iter().enumerate() {
            let byte_offset = entry_offset + 64 + i * 4;
            bytes[byte_offset..byte_offset + 4].copy_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

pub(super) fn create_output_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.output.bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub(super) fn create_output_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.output.uniform"),
        size: OUTPUT_UNIFORM_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub(super) fn create_output_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.output.bind_group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    })
}

pub(super) struct OutputUniformUpload {
    pub(super) exposure_ev: f32,
    pub(super) view_from_world: [f32; 16],
    pub(super) clip_from_view: [f32; 16],
    pub(super) clip_from_world: [f32; 16],
    pub(super) camera_position: [f32; 3],
    pub(super) viewport: [f32; 2],
    pub(super) near_far: [f32; 2],
    pub(super) color_management: [f32; 4],
    pub(super) lighting: PreparedGpuLightUniform,
}

pub(super) fn encode_output_uniform(
    upload: OutputUniformUpload,
) -> [u8; OUTPUT_UNIFORM_BYTE_LEN as usize] {
    let exposure_ev = if upload.exposure_ev.is_finite() {
        upload.exposure_ev
    } else {
        0.0
    };
    let mut values = [0.0; 100];
    values[0..16].copy_from_slice(&upload.view_from_world);
    values[16..32].copy_from_slice(&upload.clip_from_view);
    values[32..48].copy_from_slice(&upload.clip_from_world);
    values[48] = upload.camera_position[0];
    values[49] = upload.camera_position[1];
    values[50] = upload.camera_position[2];
    values[51] = 2.0_f32.powf(exposure_ev);
    values[52] = upload.viewport[0];
    values[53] = upload.viewport[1];
    values[54] = upload.near_far[0];
    values[55] = upload.near_far[1];
    values[56..60].copy_from_slice(&upload.color_management);
    values[60..64].copy_from_slice(&upload.lighting.directional_light_direction_intensity);
    values[64..68].copy_from_slice(&upload.lighting.directional_light_color_count);
    values[68..72].copy_from_slice(&upload.lighting.point_light_position_intensity);
    values[72..76].copy_from_slice(&upload.lighting.point_light_color_range);
    values[76..80].copy_from_slice(&upload.lighting.spot_light_position_intensity);
    values[80..84].copy_from_slice(&upload.lighting.spot_light_direction_cones);
    values[84..88].copy_from_slice(&upload.lighting.spot_light_cone_range);
    values[88..92].copy_from_slice(&upload.lighting.spot_light_color_range);
    values[92..96].copy_from_slice(&upload.lighting.environment_diffuse_intensity);
    values[96..100].copy_from_slice(&upload.lighting.environment_specular_intensity);
    let mut bytes = [0; OUTPUT_UNIFORM_BYTE_LEN as usize];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_uniform_buffer_matches_wgsl_uniform_layout() {
        assert_eq!(
            OUTPUT_UNIFORM_BYTE_LEN, 400,
            "CameraUniform stores view, projection, and view-projection matrices plus \
             camera/exposure, viewport/depth, color-management, punctual-light, and \
             environment uniforms — per-draw model + normal matrices live on the new \
             DrawUniform bind group at @group(2)"
        );
        assert_eq!(
            encode_output_uniform(OutputUniformUpload {
                exposure_ev: 0.0,
                view_from_world: identity_clip_from_world(),
                clip_from_view: identity_clip_from_world(),
                clip_from_world: identity_clip_from_world(),
                camera_position: [0.0, 0.0, 2.0],
                viewport: [128.0, 64.0],
                near_far: [0.1, 1000.0],
                color_management: [1.0, 0.0, 0.0, 0.0],
                lighting: PreparedGpuLightUniform::default(),
            })
            .len(),
            OUTPUT_UNIFORM_BYTE_LEN as usize
        );
    }

    #[test]
    fn triangle_shader_uses_camera_projection_uniform() {
        let raw_clip_space_assignment =
            ["out.position = vec4<f32>(in.position", ", 1.0);"].join("");
        assert!(
            GPU_TRIANGLE_SHADER.contains("clip_from_world")
                && GPU_TRIANGLE_SHADER.contains("world_from_model")
                && GPU_TRIANGLE_SHADER.contains("normal_from_model")
                && GPU_TRIANGLE_SHADER.contains("view_from_world")
                && GPU_TRIANGLE_SHADER.contains("clip_from_view")
                && GPU_TRIANGLE_SHADER.contains("camera_position_exposure")
                && GPU_TRIANGLE_SHADER.contains("viewport_near_far")
                && GPU_TRIANGLE_SHADER.contains("color_management"),
            "GPU shader uniform must expose model, normal, view, projection, view-projection, \
             camera position, viewport/depth, and color-management metadata"
        );
        assert!(
            !GPU_TRIANGLE_SHADER.contains(&raw_clip_space_assignment),
            "GPU vertex shader must not treat world-space positions as clip-space coordinates"
        );
        assert!(
            GPU_TRIANGLE_SHADER.contains("@location(2) normal: vec3<f32>")
                && GPU_TRIANGLE_SHADER.contains("@location(3) tex_coord0: vec2<f32>")
                && GPU_TRIANGLE_SHADER.contains("base_color_uv_offset_scale")
                && GPU_TRIANGLE_SHADER.contains("base_color_uv_rotation")
                && GPU_TRIANGLE_SHADER.contains(
                    "textureSample(base_color_texture, base_color_sampler, transformed_uv)"
                ),
            "GPU shader must receive normals and TEXCOORD_0 from prepared vertex data"
        );
    }

    #[test]
    fn triangle_shader_declares_material_texture_bindings() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("@group(1) @binding(0)")
                && GPU_TRIANGLE_SHADER.contains("var base_color_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(1)")
                && GPU_TRIANGLE_SHADER.contains("var base_color_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(2)")
                && GPU_TRIANGLE_SHADER.contains("var<uniform> material: MaterialUniform")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(3)")
                && GPU_TRIANGLE_SHADER.contains("var normal_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(4)")
                && GPU_TRIANGLE_SHADER.contains("var normal_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(5)")
                && GPU_TRIANGLE_SHADER.contains("var metallic_roughness_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(6)")
                && GPU_TRIANGLE_SHADER.contains("var metallic_roughness_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(7)")
                && GPU_TRIANGLE_SHADER.contains("var occlusion_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(8)")
                && GPU_TRIANGLE_SHADER.contains("var occlusion_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(9)")
                && GPU_TRIANGLE_SHADER.contains("var emissive_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(10)")
                && GPU_TRIANGLE_SHADER.contains("var emissive_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER.contains("textureSample(base_color_texture"),
            "GPU fragment shader must expose material texture bindings before backend material \
             sampling can be claimed"
        );
    }

    #[test]
    fn triangle_shader_samples_all_material_texture_roles() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("textureSample(base_color_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(normal_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(metallic_roughness_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(occlusion_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(emissive_texture")
                && GPU_TRIANGLE_SHADER.contains("base_color_factor")
                && GPU_TRIANGLE_SHADER.contains("emissive_strength")
                && GPU_TRIANGLE_SHADER.contains("metallic_roughness_alpha"),
            "GPU material shader must sample every prepared glTF material texture role and \
             consume material factor uniforms before backend material parity can be claimed"
        );
    }

    #[test]
    fn triangle_shader_discards_alpha_masked_fragments() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("material.metallic_roughness_alpha.z > 0.0")
                && GPU_TRIANGLE_SHADER.contains("base.a < material.metallic_roughness_alpha.z")
                && GPU_TRIANGLE_SHADER.contains("discard;"),
            "GPU material shader must apply alpha-mask cutoff after base-color texture sampling"
        );
    }

    #[test]
    fn triangle_shader_consumes_gpu_punctual_light_uniforms() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("struct LightingUniform")
                && GPU_TRIANGLE_SHADER.contains("directional_light_direction_intensity")
                && GPU_TRIANGLE_SHADER.contains("point_light_position_intensity")
                && GPU_TRIANGLE_SHADER.contains("spot_light_direction_cones")
                && GPU_TRIANGLE_SHADER.contains("pbr_light_contribution")
                && GPU_TRIANGLE_SHADER.contains("fresnel_schlick")
                && GPU_TRIANGLE_SHADER.contains("distribution_ggx")
                && GPU_TRIANGLE_SHADER.contains("geometry_smith"),
            "GPU PBR shader must consume prepared directional, point, and spot light uniforms \
             through a GGX/Smith/Schlick BRDF before backend PBR lighting can be claimed"
        );
    }

    #[test]
    fn triangle_shader_consumes_gpu_environment_light_uniforms() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("environment_diffuse_intensity")
                && GPU_TRIANGLE_SHADER.contains("environment_specular_intensity")
                && GPU_TRIANGLE_SHADER.contains("has_environment_light")
                && GPU_TRIANGLE_SHADER.contains("pbr_environment_lighting"),
            "GPU PBR shader must consume prepared environment irradiance/specular uniforms \
             before backend IBL lighting can be claimed"
        );
    }

    #[test]
    fn triangle_shader_consumes_prepared_directional_shadow_visibility() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("shadow_visibility")
                && GPU_TRIANGLE_SHADER.contains("* shadow_visibility"),
            "GPU PBR lighting must consume prepared directional shadow visibility instead of \
             silently ignoring opt-in shadowed directional lights"
        );
    }

    #[test]
    fn triangle_shader_builds_tangent_space_normal_from_normal_map() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("@location(4) tangent: vec4<f32>")
                && GPU_TRIANGLE_SHADER.contains("let normal_texture_sample = textureSample(normal_texture")
                && !GPU_TRIANGLE_SHADER.contains("let normal_sample = textureSample(normal_texture")
                && GPU_TRIANGLE_SHADER.contains(
                    "let bitangent = normalize(cross(world_normal, world_tangent) * in.tangent.w);"
                )
                && GPU_TRIANGLE_SHADER.contains(
                    "normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal",
                ),
            "GPU normal mapping must use a prepared tangent basis instead of treating the \
             normal texture as a scalar visibility multiplier"
        );
    }

    fn identity_clip_from_world() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ]
    }
}

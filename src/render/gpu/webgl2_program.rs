use web_sys::WebGl2RenderingContext;

use super::vertices::PrimitiveDrawBatch;

pub(super) const VERTEX_SHADER: &str = r#"#version 300 es
in vec3 position;
in vec4 color;
in vec3 normal;
in vec2 tex_coord0;
in vec4 tangent;
in float shadow_visibility;
uniform mat4 world_from_model;
uniform mat4 normal_from_model;
uniform mat4 view_from_world;
uniform mat4 clip_from_view;
uniform mat4 clip_from_world;
uniform vec4 camera_position_exposure;
uniform vec4 viewport_near_far;
uniform vec4 color_management;
uniform vec4 base_color_uv_offset_scale;
uniform vec4 base_color_uv_rotation;
out vec4 v_color;
out vec3 v_normal;
out vec2 v_tex_coord0;
out vec3 v_world_position;
out vec4 v_tangent;
out float v_shadow_visibility;
void main() {
    vec4 world_position = world_from_model * vec4(position, 1.0);
    gl_Position = clip_from_view * view_from_world * world_position;
    v_color = color;
    v_normal = mat3(normal_from_model) * normal;
    v_tex_coord0 = tex_coord0;
    v_world_position = world_position.xyz;
    v_tangent = vec4(mat3(normal_from_model) * tangent.xyz, tangent.w);
    v_shadow_visibility = clamp(shadow_visibility, 0.0, 1.0);
}"#;

pub(super) const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
in vec4 v_color;
in vec3 v_normal;
in vec2 v_tex_coord0;
in vec3 v_world_position;
in vec4 v_tangent;
in float v_shadow_visibility;
uniform vec4 camera_position_exposure;
uniform vec4 directional_light_direction_intensity;
uniform vec4 directional_light_color_count;
uniform vec4 point_light_position_intensity;
uniform vec4 point_light_color_range;
uniform vec4 spot_light_position_intensity;
uniform vec4 spot_light_direction_cones;
uniform vec4 spot_light_cone_range;
uniform vec4 spot_light_color_range;
uniform vec4 environment_diffuse_intensity;
uniform vec4 environment_specular_intensity;
uniform sampler2D base_color_texture;
uniform sampler2D normal_texture;
uniform sampler2D metallic_roughness_texture;
uniform sampler2D occlusion_texture;
uniform sampler2D emissive_texture;
uniform vec4 base_color_uv_offset_scale;
uniform vec4 base_color_uv_rotation;
uniform vec4 base_color_factor;
uniform vec4 emissive_strength;
uniform vec4 metallic_roughness_alpha;
// Phase 5.1: glTF spec scalar texture strengths.
// .x = normalTexture.scale (default 1.0)
// .y = occlusionTexture.strength (default 1.0)
uniform vec4 texture_strengths;
out vec4 out_color;
const float PI = 3.141592653589793;

float rrt_and_odt_fit(float value) {
    float numerator = value * (value + 0.0245786) - 0.000090537;
    float denominator = value * (0.983729 * value + 0.432951) + 0.238081;
    return numerator / denominator;
}

vec3 aces_tonemap(vec3 color) {
    vec3 input_color = vec3(
        dot(vec3(0.59719, 0.35458, 0.04823), color),
        dot(vec3(0.076, 0.90834, 0.01566), color),
        dot(vec3(0.0284, 0.13383, 0.83777), color)
    );
    vec3 fitted = vec3(
        rrt_and_odt_fit(input_color.r),
        rrt_and_odt_fit(input_color.g),
        rrt_and_odt_fit(input_color.b)
    );
    vec3 output_color = vec3(
        dot(vec3(1.60475, -0.53108, -0.07367), fitted),
        dot(vec3(-0.10208, 1.10813, -0.00605), fitted),
        dot(vec3(-0.00327, -0.07276, 1.07602), fitted)
    );
    return clamp(output_color, vec3(0.0), vec3(1.0));
}

float distributionGgx(float n_dot_h, float alpha) {
    float alpha_squared = alpha * alpha;
    float denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / max(PI * denominator * denominator, 0.0001);
}

float geometrySchlickGgx(float n_dot, float k) {
    return n_dot / max(n_dot * (1.0 - k) + k, 0.0001);
}

float geometrySmith(float n_dot_v, float n_dot_l, float roughness) {
    float k = ((roughness + 1.0) * (roughness + 1.0)) / 8.0;
    return geometrySchlickGgx(n_dot_v, k) * geometrySchlickGgx(n_dot_l, k);
}

vec3 fresnelSchlick(float cos_theta, vec3 f0) {
    return f0 + (vec3(1.0) - f0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

float distanceAttenuation(vec3 to_light, float range) {
    if (range <= 0.0) {
        return 1.0;
    }
    float distance_to_light = length(to_light);
    return pow(clamp(1.0 - distance_to_light / range, 0.0, 1.0), 2.0);
}

float spotConeAttenuation(float cos_angle, float inner_cone_cos, float outer_cone_cos) {
    if (cos_angle >= inner_cone_cos) {
        return 1.0;
    }
    if (cos_angle <= outer_cone_cos) {
        return 0.0;
    }
    return clamp((cos_angle - outer_cone_cos) / (inner_cone_cos - outer_cone_cos), 0.0, 1.0);
}

vec3 pbrLightContribution(
    vec3 base,
    float metallic,
    float roughness,
    vec3 normal,
    vec3 view,
    vec3 incoming,
    vec3 radiance
) {
    float n_dot_l = max(dot(normal, incoming), 0.0);
    if (n_dot_l <= 0.0) {
        return vec3(0.0);
    }
    float n_dot_v = max(dot(normal, view), 0.001);
    vec3 half_vector = normalize(view + incoming);
    float n_dot_h = max(dot(normal, half_vector), 0.0);
    float v_dot_h = max(dot(view, half_vector), 0.0);
    float alpha = roughness * roughness;
    float distribution = distributionGgx(n_dot_h, alpha);
    float geometry = geometrySmith(n_dot_v, n_dot_l, roughness);
    vec3 f0 = mix(vec3(0.04), base, metallic);
    vec3 fresnel = fresnelSchlick(v_dot_h, f0);
    vec3 specular = fresnel * (distribution * geometry / max(4.0 * n_dot_v * n_dot_l, 0.0001));
    vec3 diffuse_energy = (vec3(1.0) - fresnel) * (1.0 - metallic);
    vec3 diffuse = diffuse_energy * base / PI;
    return (diffuse + specular) * radiance * n_dot_l;
}

vec3 pbrPunctualLighting(
    vec3 base,
    float metallic,
    float roughness,
    vec3 normal,
    vec3 view,
    vec3 world_position,
    float shadow_visibility
) {
    vec3 shaded = vec3(0.0);
    if (directional_light_direction_intensity.w > 0.0) {
        vec3 incoming = normalize(-directional_light_direction_intensity.xyz);
        vec3 radiance = directional_light_color_count.rgb * directional_light_direction_intensity.w * shadow_visibility;
        shaded += pbrLightContribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    if (point_light_position_intensity.w > 0.0) {
        vec3 to_light = point_light_position_intensity.xyz - world_position;
        vec3 incoming = normalize(to_light);
        float attenuation = distanceAttenuation(to_light, point_light_color_range.w);
        vec3 radiance = point_light_color_range.rgb * point_light_position_intensity.w * attenuation;
        shaded += pbrLightContribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    if (spot_light_position_intensity.w > 0.0) {
        vec3 to_light = spot_light_position_intensity.xyz - world_position;
        vec3 incoming = normalize(to_light);
        float cone = spotConeAttenuation(
            dot(-incoming, normalize(spot_light_direction_cones.xyz)),
            spot_light_cone_range.x,
            spot_light_cone_range.y
        );
        float attenuation = distanceAttenuation(to_light, spot_light_cone_range.z);
        vec3 radiance = spot_light_color_range.rgb * spot_light_position_intensity.w * attenuation * cone;
        shaded += pbrLightContribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    return shaded;
}

bool hasPunctualLight() {
    return directional_light_direction_intensity.w > 0.0 ||
        point_light_position_intensity.w > 0.0 ||
        spot_light_position_intensity.w > 0.0;
}

bool hasEnvironmentLight() {
    return environment_diffuse_intensity.w > 0.0 ||
        environment_specular_intensity.w > 0.0;
}

vec3 pbrEnvironmentLighting(
    vec3 base,
    float metallic,
    float roughness,
    vec3 normal,
    vec3 view
) {
    if (!hasEnvironmentLight()) {
        return vec3(0.0);
    }
    float n_dot_v = max(dot(normal, view), 0.001);
    vec3 f0 = mix(vec3(0.04), base, metallic);
    vec3 fresnel = fresnelSchlick(n_dot_v, f0);
    vec3 diffuse_energy = (vec3(1.0) - fresnel) * (1.0 - metallic);
    vec3 diffuse = diffuse_energy * base * environment_diffuse_intensity.rgb;
    float specular_strength = clamp(1.25 - roughness * 0.75, 0.2, 1.25);
    vec3 specular = fresnel * environment_specular_intensity.rgb * specular_strength;
    float intensity = max(environment_diffuse_intensity.w, environment_specular_intensity.w);
    return (diffuse + specular) * intensity;
}

void main() {
    vec2 scaled_uv = v_tex_coord0 * base_color_uv_offset_scale.zw;
    vec2 transformed_uv = vec2(
        scaled_uv.x * base_color_uv_rotation.y - scaled_uv.y * base_color_uv_rotation.x,
        scaled_uv.x * base_color_uv_rotation.x + scaled_uv.y * base_color_uv_rotation.y
    ) + base_color_uv_offset_scale.xy;
    vec4 base_color_sample = texture(base_color_texture, transformed_uv);
    vec3 normal_sample = texture(normal_texture, v_tex_coord0).rgb;
    vec4 metallic_roughness_sample = texture(metallic_roughness_texture, v_tex_coord0);
    float occlusion_sample = texture(occlusion_texture, v_tex_coord0).r;
    vec3 emissive_sample = texture(emissive_texture, v_tex_coord0).rgb;
    // Phase 5.1: apply normalTexture.scale (texture_strengths.x) to the
    // tangent-space normal X/Y components.
    vec3 raw_normal = normal_sample * 2.0 - vec3(1.0);
    float normal_scale = texture_strengths.x;
    vec3 scaled_tangent_normal = vec3(
        raw_normal.x * normal_scale,
        raw_normal.y * normal_scale,
        raw_normal.z
    );
    vec3 normal_sample_tangent_space = normalize(scaled_tangent_normal);
    vec3 world_normal = normalize(v_normal);
    vec3 world_tangent = normalize(v_tangent.xyz);
    vec3 bitangent = normalize(cross(world_normal, world_tangent) * v_tangent.w);
    vec3 normal = normalize(normal_sample_tangent_space.x * world_tangent + normal_sample_tangent_space.y * bitangent + normal_sample_tangent_space.z * world_normal);
    float normal_visibility = clamp(normal_sample_tangent_space.z, 0.2, 1.0);
    float metallic = clamp(metallic_roughness_alpha.x * metallic_roughness_sample.b, 0.0, 1.0);
    float roughness = clamp(metallic_roughness_alpha.y * metallic_roughness_sample.g, 0.04, 1.0);
    // Phase 5.1: occlusionTexture.strength (texture_strengths.y).
    float occlusion_strength = texture_strengths.y;
    float occlusion_applied = mix(1.0, occlusion_sample, occlusion_strength);
    float material_response = normal_visibility * occlusion_applied * mix(0.92, 1.0, roughness) * (1.0 - metallic * 0.08);
    vec4 base = v_color * base_color_factor * base_color_sample;
    if (metallic_roughness_alpha.z > 0.0 && base.a < metallic_roughness_alpha.z) {
        discard;
    }
    vec3 emissive = emissive_strength.rgb * emissive_sample * emissive_strength.w;
    vec3 view = normalize(camera_position_exposure.xyz - v_world_position);
    vec3 shaded_rgb = base.rgb;
    if (metallic_roughness_alpha.w < 0.5) {
        shaded_rgb = base.rgb * material_response;
        vec3 direct = pbrPunctualLighting(
            base.rgb,
            metallic,
            roughness,
            normal,
            view,
            v_world_position,
            v_shadow_visibility
        );
        vec3 environment = pbrEnvironmentLighting(base.rgb, metallic, roughness, normal, view);
        if (hasPunctualLight() || hasEnvironmentLight()) {
            shaded_rgb = (direct + environment) * occlusion_applied;
        }
    }
    vec4 shaded = vec4(shaded_rgb + emissive, base.a);
    out_color = vec4(aces_tonemap(shaded.rgb * camera_position_exposure.w), shaded.a);
}"#;

pub(super) fn context_options() -> js_sys::Object {
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&options, &"antialias".into(), &wasm_bindgen::JsValue::FALSE);
    let _ = js_sys::Reflect::set(&options, &"depth".into(), &wasm_bindgen::JsValue::TRUE);
    options
}

pub(super) fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<web_sys::WebGlShader, wasm_bindgen::JsValue> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 shader allocation failed"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(wasm_bindgen::JsValue::from_str(
            &gl.get_shader_info_log(&shader)
                .unwrap_or_else(|| "webgl2 shader compile failed".to_string()),
        ))
    }
}

pub(super) fn link_program(
    gl: &WebGl2RenderingContext,
    vertex_shader: &web_sys::WebGlShader,
    fragment_shader: &web_sys::WebGlShader,
) -> Result<web_sys::WebGlProgram, wasm_bindgen::JsValue> {
    let program = gl
        .create_program()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 program allocation failed"))?;
    gl.attach_shader(&program, vertex_shader);
    gl.attach_shader(&program, fragment_shader);
    gl.link_program(&program);
    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(wasm_bindgen::JsValue::from_str(
            &gl.get_program_info_log(&program)
                .unwrap_or_else(|| "webgl2 program link failed".to_string()),
        ))
    }
}

pub(super) fn vertex_stream_hash(vertices: &[f32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ vertices.len() as u64;
    for value in vertices {
        hash ^= u64::from(value.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub(super) fn draw_batch_hash(draw_batches: &[PrimitiveDrawBatch]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ draw_batches.len() as u64;
    for batch in draw_batches {
        for value in [batch.start_vertex, batch.vertex_count, batch.material_slot] {
            hash ^= u64::from(value);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::FRAGMENT_SHADER;
    use super::VERTEX_SHADER;

    #[test]
    fn webgl2_fragment_shader_samples_all_material_texture_roles() {
        assert!(
            FRAGMENT_SHADER.contains("texture(base_color_texture")
                && FRAGMENT_SHADER.contains("texture(normal_texture")
                && FRAGMENT_SHADER.contains("texture(metallic_roughness_texture")
                && FRAGMENT_SHADER.contains("texture(occlusion_texture")
                && FRAGMENT_SHADER.contains("texture(emissive_texture")
                && FRAGMENT_SHADER.contains("base_color_factor")
                && FRAGMENT_SHADER.contains("emissive_strength")
                && FRAGMENT_SHADER.contains("metallic_roughness_alpha"),
            "WebGL2 material shader must sample every prepared glTF material texture role and \
             consume material factor uniforms before compatibility material parity can be claimed"
        );
    }

    #[test]
    fn webgl2_fragment_shader_discards_alpha_masked_fragments() {
        assert!(
            FRAGMENT_SHADER.contains("metallic_roughness_alpha.z > 0.0")
                && FRAGMENT_SHADER.contains("base.a < metallic_roughness_alpha.z")
                && FRAGMENT_SHADER.contains("discard;"),
            "WebGL2 material shader must apply alpha-mask cutoff after base-color texture sampling"
        );
    }

    #[test]
    fn webgl2_fragment_shader_consumes_gpu_punctual_light_uniforms() {
        assert!(
            FRAGMENT_SHADER.contains("uniform vec4 directional_light_direction_intensity")
                && FRAGMENT_SHADER.contains("uniform vec4 point_light_position_intensity")
                && FRAGMENT_SHADER.contains("uniform vec4 spot_light_direction_cones")
                && FRAGMENT_SHADER.contains("pbrLightContribution")
                && FRAGMENT_SHADER.contains("fresnelSchlick")
                && FRAGMENT_SHADER.contains("distributionGgx")
                && FRAGMENT_SHADER.contains("geometrySmith"),
            "WebGL2 PBR shader must consume prepared directional, point, and spot light \
             uniforms through a GGX/Smith/Schlick BRDF before compatibility PBR lighting can be claimed"
        );
    }

    #[test]
    fn webgl2_fragment_shader_consumes_gpu_environment_light_uniforms() {
        assert!(
            FRAGMENT_SHADER.contains("uniform vec4 environment_diffuse_intensity")
                && FRAGMENT_SHADER.contains("uniform vec4 environment_specular_intensity")
                && FRAGMENT_SHADER.contains("hasEnvironmentLight")
                && FRAGMENT_SHADER.contains("pbrEnvironmentLighting"),
            "WebGL2 PBR shader must consume prepared environment lighting uniforms before \
             compatibility IBL lighting can be claimed"
        );
    }

    #[test]
    fn webgl2_fragment_shader_consumes_prepared_directional_shadow_visibility() {
        assert!(
            VERTEX_SHADER.contains("shadow_visibility")
                && FRAGMENT_SHADER.contains("shadow_visibility")
                && FRAGMENT_SHADER.contains("* shadow_visibility"),
            "WebGL2 compatibility PBR must consume prepared directional shadow visibility instead \
             of silently ignoring opt-in shadowed directional lights"
        );
    }

    #[test]
    fn webgl2_shader_builds_tangent_space_normal_from_normal_map() {
        assert!(
            FRAGMENT_SHADER.contains("in vec4 v_tangent")
                && FRAGMENT_SHADER.contains("vec3 bitangent = normalize(cross(world_normal, world_tangent) * v_tangent.w);")
                && FRAGMENT_SHADER.contains("normal_sample_tangent_space.x * world_tangent + normal_sample_tangent_space.y * bitangent + normal_sample_tangent_space.z * world_normal"),
            "WebGL2 normal mapping must use the prepared tangent basis"
        );
    }

    #[test]
    fn webgl2_vertex_shader_consumes_model_normal_view_and_projection_uniforms() {
        assert!(
            VERTEX_SHADER.contains("uniform mat4 world_from_model")
                && VERTEX_SHADER.contains("uniform mat4 normal_from_model")
                && VERTEX_SHADER.contains("uniform mat4 view_from_world")
                && VERTEX_SHADER.contains("uniform mat4 clip_from_view")
                && VERTEX_SHADER.contains("uniform mat4 clip_from_world")
                && VERTEX_SHADER.contains("clip_from_view * view_from_world * world_position")
                && VERTEX_SHADER.contains("mat3(normal_from_model) * normal"),
            "WebGL2 vertex shader must expose the same model/normal/view/projection vocabulary as native GPU"
        );
    }
}

struct LtcTableSample {
    inverse_matrix: vec4<f32>,
    fresnel_terms: vec4<f32>,
}

struct LtcSpecularProbe {
    irradiance: f32,
    fresnel_scale: vec3<f32>,
}

struct LtcClippedQuad {
    vertices: array<vec3<f32>, 5>,
    count: u32,
}

fn ltc_safe_normalize(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let len_sq = dot(value, value);
    return select(fallback, value * inverseSqrt(len_sq), len_sq > 0.00000001);
}

fn ltc_lerp_vec4(left: vec4<f32>, right: vec4<f32>, t: f32) -> vec4<f32> {
    return left * (1.0 - t) + right * t;
}

fn ltc_bilinear_sample(
    table_00: vec4<f32>,
    table_10: vec4<f32>,
    table_01: vec4<f32>,
    table_11: vec4<f32>,
    tx: f32,
    ty: f32,
) -> vec4<f32> {
    return ltc_lerp_vec4(
        ltc_lerp_vec4(table_00, table_10, tx),
        ltc_lerp_vec4(table_01, table_11, tx),
        ty,
    );
}

fn ltc_lookup_tables(roughness_input: f32, n_dot_v_input: f32) -> LtcTableSample {
    let roughness = clamp(roughness_input, 0.0, 1.0);
    let view = sqrt(max(1.0 - clamp(n_dot_v_input, 0.0, 1.0), 0.0));
    let x = roughness * LTC_LUT_LAST_F;
    let y = view * LTC_LUT_LAST_F;
    let x0 = u32(floor(x));
    let y0 = u32(floor(y));
    let x1 = min(x0 + 1u, LTC_LUT_LAST_U);
    let y1 = min(y0 + 1u, LTC_LUT_LAST_U);
    let tx = x - f32(x0);
    let ty = y - f32(y0);
    return LtcTableSample(
        ltc_bilinear_sample(
            ltc_tables.table_1[y0 * LTC_LUT_STRIDE_U + x0],
            ltc_tables.table_1[y0 * LTC_LUT_STRIDE_U + x1],
            ltc_tables.table_1[y1 * LTC_LUT_STRIDE_U + x0],
            ltc_tables.table_1[y1 * LTC_LUT_STRIDE_U + x1],
            tx,
            ty,
        ),
        ltc_bilinear_sample(
            ltc_tables.table_2[y0 * LTC_LUT_STRIDE_U + x0],
            ltc_tables.table_2[y0 * LTC_LUT_STRIDE_U + x1],
            ltc_tables.table_2[y1 * LTC_LUT_STRIDE_U + x0],
            ltc_tables.table_2[y1 * LTC_LUT_STRIDE_U + x1],
            tx,
            ty,
        ),
    );
}

fn ltc_apply_inverse_matrix(matrix: vec4<f32>, value: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        matrix.x * value.x + matrix.z * value.z,
        value.y,
        matrix.y * value.x + matrix.w * value.z,
    );
}

fn ltc_horizon_intersection(inside: vec3<f32>, outside: vec3<f32>) -> vec3<f32> {
    return -outside.z * inside + inside.z * outside;
}

fn ltc_clip_quad_to_horizon(vertices: array<vec3<f32>, 4>) -> LtcClippedQuad {
    var clipped: array<vec3<f32>, 5>;
    clipped[0] = vertices[0];
    clipped[1] = vertices[1];
    clipped[2] = vertices[2];
    clipped[3] = vertices[3];
    clipped[4] = vec3<f32>(0.0);
    var config = 0u;
    if vertices[0].z > 0.0 { config = config + 1u; }
    if vertices[1].z > 0.0 { config = config + 2u; }
    if vertices[2].z > 0.0 { config = config + 4u; }
    if vertices[3].z > 0.0 { config = config + 8u; }
    var count = 0u;

    if config == 1u {
        count = 3u;
        clipped[1] = ltc_horizon_intersection(clipped[0], clipped[1]);
        clipped[2] = ltc_horizon_intersection(clipped[0], clipped[3]);
    } else if config == 2u {
        count = 3u;
        clipped[0] = ltc_horizon_intersection(clipped[1], clipped[0]);
        clipped[2] = ltc_horizon_intersection(clipped[1], clipped[2]);
    } else if config == 3u {
        count = 4u;
        clipped[2] = ltc_horizon_intersection(clipped[1], clipped[2]);
        clipped[3] = ltc_horizon_intersection(clipped[0], clipped[3]);
    } else if config == 4u {
        count = 3u;
        clipped[0] = ltc_horizon_intersection(clipped[2], clipped[3]);
        clipped[1] = ltc_horizon_intersection(clipped[2], clipped[1]);
    } else if config == 6u {
        count = 4u;
        clipped[0] = ltc_horizon_intersection(clipped[1], clipped[0]);
        clipped[3] = ltc_horizon_intersection(clipped[2], clipped[3]);
    } else if config == 7u {
        count = 5u;
        clipped[4] = ltc_horizon_intersection(clipped[0], clipped[3]);
        clipped[3] = ltc_horizon_intersection(clipped[2], clipped[3]);
    } else if config == 8u {
        count = 3u;
        clipped[0] = ltc_horizon_intersection(clipped[3], clipped[0]);
        clipped[1] = ltc_horizon_intersection(clipped[3], clipped[2]);
        clipped[2] = clipped[3];
    } else if config == 9u {
        count = 4u;
        clipped[1] = ltc_horizon_intersection(clipped[0], clipped[1]);
        clipped[2] = ltc_horizon_intersection(clipped[3], clipped[2]);
    } else if config == 11u {
        count = 5u;
        clipped[4] = clipped[3];
        clipped[3] = ltc_horizon_intersection(clipped[3], clipped[2]);
        clipped[2] = ltc_horizon_intersection(clipped[1], clipped[2]);
    } else if config == 12u {
        count = 4u;
        clipped[1] = ltc_horizon_intersection(clipped[2], clipped[1]);
        clipped[0] = ltc_horizon_intersection(clipped[3], clipped[0]);
    } else if config == 13u {
        count = 5u;
        clipped[4] = clipped[3];
        clipped[3] = clipped[2];
        clipped[2] = ltc_horizon_intersection(clipped[2], clipped[1]);
        clipped[1] = ltc_horizon_intersection(clipped[0], clipped[1]);
    } else if config == 14u {
        count = 5u;
        clipped[4] = ltc_horizon_intersection(clipped[3], clipped[0]);
        clipped[0] = ltc_horizon_intersection(clipped[1], clipped[0]);
    } else if config == 15u {
        count = 4u;
    }

    if count == 3u {
        clipped[3] = clipped[0];
    }
    if count == 4u {
        clipped[4] = clipped[0];
    }
    return LtcClippedQuad(clipped, count);
}

fn ltc_integrate_edge(left: vec3<f32>, right: vec3<f32>) -> f32 {
    let cosine = clamp(dot(left, right), -0.9999, 0.9999);
    let y = abs(cosine);
    let numerator = 0.8543985 + (0.4965155 + 0.0145206 * y) * y;
    let denominator = max(3.417594 + (4.1616724 + y) * y, 0.0001);
    let approximation = numerator / denominator;
    let theta_sin_theta = select(
        0.5 / sqrt(max(1.0 - cosine * cosine, 0.0000001)) - approximation,
        approximation,
        cosine > 0.0,
    );
    return cross(left, right).z * theta_sin_theta;
}

fn ltc_integrate_transformed_quad(vertices: array<vec3<f32>, 4>, two_sided: bool) -> f32 {
    var clipped = ltc_clip_quad_to_horizon(vertices);
    if clipped.count == 0u {
        return 0.0;
    }
    // Keep these indices literal. ANGLE's D3D11 backend forces the former
    // runtime-bounded local-array loop to unroll, but the generated HLSL loop
    // cannot be unrolled and fails with X3511 before the first material draw.
    // Entries outside the clipped polygon are never consumed below.
    clipped.vertices[0] = ltc_safe_normalize(clipped.vertices[0], vec3<f32>(0.0, 0.0, 1.0));
    clipped.vertices[1] = ltc_safe_normalize(clipped.vertices[1], vec3<f32>(0.0, 0.0, 1.0));
    clipped.vertices[2] = ltc_safe_normalize(clipped.vertices[2], vec3<f32>(0.0, 0.0, 1.0));
    clipped.vertices[3] = ltc_safe_normalize(clipped.vertices[3], vec3<f32>(0.0, 0.0, 1.0));
    clipped.vertices[4] = ltc_safe_normalize(clipped.vertices[4], vec3<f32>(0.0, 0.0, 1.0));
    var sum = ltc_integrate_edge(clipped.vertices[0], clipped.vertices[1]) +
        ltc_integrate_edge(clipped.vertices[1], clipped.vertices[2]) +
        ltc_integrate_edge(clipped.vertices[2], clipped.vertices[3]);
    if clipped.count >= 4u {
        sum += ltc_integrate_edge(clipped.vertices[3], clipped.vertices[4]);
    }
    if clipped.count == 5u {
        sum += ltc_integrate_edge(clipped.vertices[4], clipped.vertices[0]);
    }
    return select(max(sum, 0.0), abs(sum), two_sided);
}

fn ltc_area_light_polygon(index: u32, world_position: vec3<f32>, normal: vec3<f32>) -> array<vec3<f32>, 4> {
    let center = camera.lighting.area_light_position_flux[index].xyz;
    let axis_x_shape = camera.lighting.area_light_axis_x_shape[index];
    let axis_x = axis_x_shape.xyz;
    let axis_y = camera.lighting.area_light_axis_y_range[index].xyz;
    var polygon: array<vec3<f32>, 4>;
    if axis_x_shape.w < 0.5 {
        polygon[0] = center - axis_x - axis_y;
        polygon[1] = center + axis_x - axis_y;
        polygon[2] = center + axis_x + axis_y;
        polygon[3] = center - axis_x + axis_y;
        return polygon;
    }
    if axis_x_shape.w < 1.5 {
        polygon[0] = center - axis_x;
        polygon[1] = center - axis_y;
        polygon[2] = center + axis_x;
        polygon[3] = center + axis_y;
        return polygon;
    }
    let radius = max(max(length(axis_x), length(axis_y)), 0.001);
    let to_surface = ltc_safe_normalize(world_position - center, -normal);
    let tangent = ltc_safe_normalize(cross(normal, to_surface), vec3<f32>(1.0, 0.0, 0.0)) * radius;
    let bitangent = ltc_safe_normalize(cross(to_surface, tangent), vec3<f32>(0.0, 0.0, 1.0)) * radius;
    polygon[0] = center - tangent - bitangent;
    polygon[1] = center + tangent - bitangent;
    polygon[2] = center + tangent + bitangent;
    polygon[3] = center - tangent + bitangent;
    return polygon;
}

fn ltc_evaluate_specular_polygon(
    polygon: array<vec3<f32>, 4>,
    world_position: vec3<f32>,
    normal_input: vec3<f32>,
    view_input: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>,
) -> LtcSpecularProbe {
    let normal = ltc_safe_normalize(normal_input, vec3<f32>(0.0, 1.0, 0.0));
    let view = ltc_safe_normalize(view_input, normal);
    let n_dot_v = clamp(dot(normal, view), 0.0, 1.0);
    if n_dot_v <= 0.000001 {
        return LtcSpecularProbe(0.0, vec3<f32>(0.0));
    }
    let sample = ltc_lookup_tables(roughness, n_dot_v);
    let fallback_axis = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(normal.z) < 0.9);
    let fallback_tangent = ltc_safe_normalize(cross(fallback_axis, normal), vec3<f32>(1.0, 0.0, 0.0));
    let tangent = ltc_safe_normalize(view - normal * dot(view, normal), fallback_tangent);
    let bitangent = ltc_safe_normalize(cross(normal, tangent), vec3<f32>(0.0, 0.0, 1.0));
    var transformed: array<vec3<f32>, 4>;
    for (var i = 0u; i < 4u; i = i + 1u) {
        let local = polygon[i] - world_position;
        let basis = vec3<f32>(dot(local, tangent), dot(local, bitangent), dot(local, normal));
        transformed[i] = ltc_apply_inverse_matrix(sample.inverse_matrix, basis);
    }
    let irradiance = ltc_integrate_transformed_quad(transformed, true);
    let fresnel_scale = f0 * sample.fresnel_terms.x +
        (vec3<f32>(1.0) - f0) * sample.fresnel_terms.y;
    return LtcSpecularProbe(irradiance, fresnel_scale);
}

fn ltc_area_light_specular_contribution(
    index: u32,
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    world_position: vec3<f32>,
    shadow_visibility: f32,
) -> vec3<f32> {
    let visibility = clamp(shadow_visibility, 0.0, 1.0);
    if visibility <= 0.000001 {
        return vec3<f32>(0.0);
    }
    let polygon = ltc_area_light_polygon(index, world_position, normal);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let probe = ltc_evaluate_specular_polygon(
        polygon,
        world_position,
        normal,
        view,
        roughness,
        f0,
    );
    if probe.irradiance <= 0.000001 {
        return vec3<f32>(0.0);
    }
    let center = camera.lighting.area_light_position_flux[index].xyz;
    let to_light = center - world_position;
    let range = camera.lighting.area_light_axis_y_range[index].w;
    let radiance = camera.lighting.area_light_color[index].rgb *
        camera.lighting.area_light_position_flux[index].w /
        (4.0 * PI) *
        distance_attenuation(to_light, range);
    return probe.fresnel_scale * radiance * probe.irradiance * visibility;
}

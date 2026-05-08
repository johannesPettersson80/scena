use web_sys::{WebGl2RenderingContext, WebGlProgram};

pub(super) fn encode_vertices(primitives: &[crate::geometry::Primitive]) -> Vec<f32> {
    let mut vertices = Vec::with_capacity(primitives.len() * 3 * 17);
    for primitive in primitives {
        for (vertex, attributes) in primitive
            .vertices()
            .iter()
            .zip(primitive.vertex_attributes().iter())
        {
            vertices.extend_from_slice(&[
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
                vertex.color.r,
                vertex.color.g,
                vertex.color.b,
                vertex.color.a,
                attributes.normal.x,
                attributes.normal.y,
                attributes.normal.z,
                attributes.tex_coord0[0],
                attributes.tex_coord0[1],
                attributes.tangent.x,
                attributes.tangent.y,
                attributes.tangent.z,
                attributes.tangent_handedness,
                attributes.shadow_visibility.clamp(0.0, 1.0),
            ]);
        }
    }
    vertices
}

pub(super) fn configure_vertex_attributes(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
) -> Result<(), wasm_bindgen::JsValue> {
    let stride = (17 * std::mem::size_of::<f32>()) as i32;
    let position = gl.get_attrib_location(program, "position");
    let color = gl.get_attrib_location(program, "color");
    let normal = gl.get_attrib_location(program, "normal");
    let tex_coord0 = gl.get_attrib_location(program, "tex_coord0");
    let tangent = gl.get_attrib_location(program, "tangent");
    let shadow_visibility = gl.get_attrib_location(program, "shadow_visibility");
    if position < 0
        || color < 0
        || normal < 0
        || tex_coord0 < 0
        || tangent < 0
        || shadow_visibility < 0
    {
        return Err(wasm_bindgen::JsValue::from_str(
            "webgl2 shader is missing required vertex attributes",
        ));
    }
    let position = position as u32;
    let color = color as u32;
    let normal = normal as u32;
    let tex_coord0 = tex_coord0 as u32;
    let tangent = tangent as u32;
    let shadow_visibility = shadow_visibility as u32;
    gl.enable_vertex_attrib_array(position);
    gl.vertex_attrib_pointer_with_i32(position, 3, WebGl2RenderingContext::FLOAT, false, stride, 0);
    gl.enable_vertex_attrib_array(color);
    gl.vertex_attrib_pointer_with_i32(
        color,
        4,
        WebGl2RenderingContext::FLOAT,
        false,
        stride,
        (3 * std::mem::size_of::<f32>()) as i32,
    );
    gl.enable_vertex_attrib_array(normal);
    gl.vertex_attrib_pointer_with_i32(
        normal,
        3,
        WebGl2RenderingContext::FLOAT,
        false,
        stride,
        (7 * std::mem::size_of::<f32>()) as i32,
    );
    gl.enable_vertex_attrib_array(tex_coord0);
    gl.vertex_attrib_pointer_with_i32(
        tex_coord0,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        stride,
        (10 * std::mem::size_of::<f32>()) as i32,
    );
    gl.enable_vertex_attrib_array(tangent);
    gl.vertex_attrib_pointer_with_i32(
        tangent,
        4,
        WebGl2RenderingContext::FLOAT,
        false,
        stride,
        (12 * std::mem::size_of::<f32>()) as i32,
    );
    gl.enable_vertex_attrib_array(shadow_visibility);
    gl.vertex_attrib_pointer_with_i32(
        shadow_visibility,
        1,
        WebGl2RenderingContext::FLOAT,
        false,
        stride,
        (16 * std::mem::size_of::<f32>()) as i32,
    );
    Ok(())
}

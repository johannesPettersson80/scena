use crate::geometry::{Primitive, PrimitiveVertexAttributes, Vertex};
use crate::render::prepare::transforms::{
    invert_matrix4, unbake_normal_to_model_space, unbake_position_to_model_space,
};
use web_sys::{WebGl2RenderingContext, WebGlProgram};

pub(super) fn encode_vertices(primitives: &[Primitive]) -> Vec<f32> {
    let mut vertices = Vec::with_capacity(primitives.len() * 3 * 17);
    for primitive in primitives {
        let world_from_model = primitive.world_from_model();
        let normal_from_model = primitive.normal_from_model();
        let position_inverse = invert_matrix4(&world_from_model);
        let normal_inverse = invert_matrix4(&normal_from_model);
        for (vertex, attributes) in primitive
            .vertices()
            .iter()
            .zip(primitive.vertex_attributes().iter())
        {
            let model_vertex = match position_inverse {
                Some(inv) => Vertex {
                    position: unbake_position_to_model_space(vertex.position, &inv),
                    color: vertex.color,
                },
                None => *vertex,
            };
            let model_attributes = match normal_inverse {
                Some(inv) => PrimitiveVertexAttributes {
                    normal: unbake_normal_to_model_space(attributes.normal, &inv),
                    tex_coord0: attributes.tex_coord0,
                    tangent: unbake_normal_to_model_space(attributes.tangent, &inv),
                    tangent_handedness: attributes.tangent_handedness,
                    shadow_visibility: attributes.shadow_visibility,
                },
                None => *attributes,
            };
            vertices.extend_from_slice(&[
                model_vertex.position.x,
                model_vertex.position.y,
                model_vertex.position.z,
                model_vertex.color.r,
                model_vertex.color.g,
                model_vertex.color.b,
                model_vertex.color.a,
                model_attributes.normal.x,
                model_attributes.normal.y,
                model_attributes.normal.z,
                model_attributes.tex_coord0[0],
                model_attributes.tex_coord0[1],
                model_attributes.tangent.x,
                model_attributes.tangent.y,
                model_attributes.tangent.z,
                model_attributes.tangent_handedness,
                model_attributes.shadow_visibility.clamp(0.0, 1.0),
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

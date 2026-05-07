use wasm_bindgen::JsCast;
use web_sys::WebGl2RenderingContext;

pub(super) fn encode_vertices(primitives: &[crate::geometry::Primitive]) -> Vec<f32> {
    let mut vertices = Vec::with_capacity(primitives.len() * 3 * 7);
    for primitive in primitives {
        for vertex in primitive.vertices() {
            vertices.extend_from_slice(&[
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
                vertex.color.r,
                vertex.color.g,
                vertex.color.b,
                vertex.color.a,
            ]);
        }
    }
    vertices
}

pub(super) fn render_canvas(
    canvas: &web_sys::HtmlCanvasElement,
    vertices: &[f32],
) -> Result<(), wasm_bindgen::JsValue> {
    let gl = canvas
        .get_context_with_context_options("webgl2", &context_options())?
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 context unavailable"))?
        .dyn_into::<WebGl2RenderingContext>()?;
    let vertex_shader = compile_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, VERTEX_SHADER)?;
    let fragment_shader = compile_shader(
        &gl,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        FRAGMENT_SHADER,
    )?;
    let program = link_program(&gl, &vertex_shader, &fragment_shader)?;
    let buffer = gl
        .create_buffer()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 buffer allocation failed"))?;
    let vertex_array = js_sys::Float32Array::from(vertices);

    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
    gl.use_program(Some(&program));
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &vertex_array,
        WebGl2RenderingContext::STATIC_DRAW,
    );

    let stride = (7 * std::mem::size_of::<f32>()) as i32;
    let position = gl.get_attrib_location(&program, "position") as u32;
    let color = gl.get_attrib_location(&program, "color") as u32;
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
    gl.draw_arrays(
        WebGl2RenderingContext::TRIANGLES,
        0,
        (vertices.len() / 7) as i32,
    );
    gl.flush();
    Ok(())
}

const VERTEX_SHADER: &str = r#"#version 300 es
in vec3 position;
in vec4 color;
out vec4 v_color;
void main() {
    gl_Position = vec4(position, 1.0);
    v_color = color;
}"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
in vec4 v_color;
out vec4 out_color;
void main() {
    out_color = v_color;
}"#;

fn context_options() -> js_sys::Object {
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&options, &"antialias".into(), &wasm_bindgen::JsValue::FALSE);
    options
}

fn compile_shader(
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

fn link_program(
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

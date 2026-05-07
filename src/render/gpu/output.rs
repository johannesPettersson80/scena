pub(super) const GPU_TRIANGLE_SHADER: &str = r#"
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct OutputUniform {
    exposure_multiplier: f32,
    padding: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> output: OutputUniform;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(aces_tonemap(in.color.rgb * output.exposure_multiplier), in.color.a);
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

pub(super) const OUTPUT_UNIFORM_BYTE_LEN: u64 = 32;

pub(super) fn create_output_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.output.bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
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

pub(super) fn encode_output_uniform(exposure_ev: f32) -> [u8; OUTPUT_UNIFORM_BYTE_LEN as usize] {
    let exposure_ev = if exposure_ev.is_finite() {
        exposure_ev
    } else {
        0.0
    };
    let values = [2.0_f32.powf(exposure_ev), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
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
            OUTPUT_UNIFORM_BYTE_LEN, 32,
            "OutputUniform has an f32 followed by a vec3<f32>; the vec3 starts at 16-byte \
             alignment and the struct rounds up to 32 bytes on Metal/wgpu"
        );
        assert_eq!(
            encode_output_uniform(0.0).len(),
            OUTPUT_UNIFORM_BYTE_LEN as usize
        );
    }
}

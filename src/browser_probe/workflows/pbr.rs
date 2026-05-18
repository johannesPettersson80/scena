use std::io::Cursor;

use base64::Engine;
use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::{WorkflowScene, add_default_camera};
use crate::{
    Angle, Assets, Color, DirectionalLight, GeometryDesc, GeometryTopology, GeometryVertex,
    MaterialDesc, PointLight, Scene, SpotLight, Transform, Vec3,
};

pub(super) fn point_light_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.25, 0.25, 0.25), 0.0, 0.8)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("PBR point-light mesh failed: {error:?}")))?;
    scene
        .point_light(
            PointLight::default()
                .with_color(Color::from_linear_rgb(0.0, 1.0, 0.0))
                .with_intensity_candela(180.0)
                .with_range(5.0),
        )
        .transform(Transform::at(Vec3::new(0.0, 0.0, 1.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("PBR point light insert failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-punctual-light",
            "light_kind": "green",
            "light_type": "point",
            "material_kind": "pbr-metallic-roughness",
        }),
    })
}

pub(super) fn spot_light_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.25, 0.25, 0.25), 0.0, 0.8)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("PBR spot-light mesh failed: {error:?}")))?;
    scene
        .spot_light(
            SpotLight::default()
                .with_color(Color::from_linear_rgb(0.0, 0.0, 1.0))
                .with_intensity_candela(200.0)
                .with_range(5.0)
                .with_inner_cone_angle(Angle::from_degrees(20.0))
                .with_outer_cone_angle(Angle::from_degrees(35.0)),
        )
        .transform(Transform::at(Vec3::new(0.0, 0.0, 1.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("PBR spot light insert failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-punctual-light",
            "light_kind": "blue",
            "light_type": "spot",
            "material_kind": "pbr-metallic-roughness",
        }),
    })
}

pub(super) async fn normal_map_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let flat = assets
        .load_texture(
            rgba_png_data_uri([128, 128, 255, 255])?,
            crate::TextureColorSpace::Linear,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("flat normal texture failed: {error:?}")))?;
    let inverted = assets
        .load_texture(
            rgba_png_data_uri([128, 128, 0, 255])?,
            crate::TextureColorSpace::Linear,
        )
        .await
        .map_err(|error| {
            JsValue::from_str(&format!("inverted normal texture failed: {error:?}"))
        })?;
    let white = assets
        .load_texture(
            rgba_png_data_uri([255, 255, 255, 255])?,
            crate::TextureColorSpace::Srgb,
        )
        .await
        .map_err(|error| {
            JsValue::from_str(&format!("normal-map base texture failed: {error:?}"))
        })?;
    let geometry = assets.create_geometry(normal_map_quad_geometry());
    let flat_material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8)
            .with_base_color_texture(white)
            .with_normal_texture(flat)
            .with_double_sided(true),
    );
    let inverted_material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8)
            .with_base_color_texture(white)
            .with_normal_texture(inverted)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, flat_material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("flat normal mesh failed: {error:?}")))?;
    scene
        .mesh(geometry, inverted_material)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("inverted normal mesh failed: {error:?}")))?;
    scene
        .directional_light(crate::DirectionalLight::default().with_illuminance_lux(20_000.0))
        .add()
        .map_err(|error| JsValue::from_str(&format!("normal-map light failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-normal-map",
            "material_kind": "pbr-metallic-roughness",
            "normal_map_pixels": {
                "flat_normal": true,
                "inverted_normal": true,
            },
        }),
    })
}

fn normal_map_quad_geometry() -> GeometryDesc {
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        vec![
            GeometryVertex {
                position: Vec3::new(-0.36, -0.55, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.36, -0.55, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.36, 0.55, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(-0.36, 0.55, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
        vec![Color::WHITE; 4],
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .expect("browser normal-map proof quad geometry is valid")
}

pub(super) fn environment_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.04, 0.04, 0.04), 0.0, 0.7)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("PBR environment mesh failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-environment-light",
            "environment_kind": "inline-radiance-hdr",
            "material_kind": "pbr-metallic-roughness",
            "environment_path": radiance_hdr_data_uri(
                2,
                1,
                &[[16, 32, 255, 132], [16, 32, 255, 132]],
                "studio-blue_2x1.hdr",
            ),
        }),
    })
}

pub(super) fn shadow_visibility_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let receiver = assets.create_geometry(shadow_receiver_geometry());
    let caster = assets.create_geometry(shadow_caster_geometry());
    let receiver_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let caster_material = assets.create_material(MaterialDesc::unlit(Color::BLACK));
    let mut scene = Scene::new();
    scene
        .mesh(receiver, receiver_material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("lit receiver insert failed: {error:?}")))?;
    scene
        .mesh(receiver, receiver_material)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("shadow receiver insert failed: {error:?}")))?;
    scene
        .mesh(caster, caster_material)
        .transform(Transform::at(Vec3::new(0.69, 0.0, 0.50)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("shadow caster insert failed: {error:?}")))?;
    scene
        .directional_light(
            DirectionalLight::default()
                .with_illuminance_lux(10_000.0)
                .with_shadows(true),
        )
        .transform(Transform::IDENTITY.rotate_y_deg(30.0))
        .add()
        .map_err(|error| JsValue::from_str(&format!("shadow light insert failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-directional-shadow-visibility",
            "material_kind": "pbr-metallic-roughness",
            "shadow_source": "prepared-visibility",
            "point_spot_shadows": "v1.x-deferred",
        }),
    })
}

fn rgba_png_data_uri(pixel: [u8; 4]) -> Result<String, JsValue> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| JsValue::from_str(&format!("normal PNG header failed: {error}")))?;
        writer
            .write_image_data(&pixel)
            .map_err(|error| JsValue::from_str(&format!("normal PNG payload failed: {error}")))?;
    }
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn shadow_receiver_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            GeometryVertex {
                position: Vec3::new(-0.15, -0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.15, -0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.15, 0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(-0.15, 0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("browser shadow receiver geometry is valid")
}

fn shadow_caster_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            GeometryVertex {
                position: Vec3::new(-0.23, -0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.23, -0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.23, 0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            GeometryVertex {
                position: Vec3::new(-0.23, 0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("browser shadow caster geometry is valid")
}

fn radiance_hdr_data_uri(width: u32, height: u32, pixels: &[[u8; 4]], name: &str) -> String {
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for pixel in pixels {
        bytes.extend_from_slice(pixel);
    }
    format!(
        "data:application/radiance-hdr;base64,{}#{name}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

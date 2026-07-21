use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::geometry::{GeometryMorphTarget, GeometrySkin};
use crate::material::Color;
use crate::scene::Vec3;

pub(super) struct PrimitiveStreams<'a> {
    pub(super) path: &'a AssetPath,
    pub(super) positions: Vec<Vec3>,
    pub(super) normals: Option<Vec<Vec3>>,
    pub(super) indices: Vec<u32>,
    pub(super) vertex_colors: Option<Vec<Color>>,
    pub(super) tex_coords0: Option<Vec<[f32; 2]>>,
    pub(super) tangents: Option<Vec<[f32; 4]>>,
    pub(super) skin: Option<GeometrySkin>,
    pub(super) morph_targets: Vec<GeometryMorphTarget>,
}

pub(super) struct ResolvedPrimitiveStreams {
    pub(super) positions: Vec<Vec3>,
    pub(super) normals: Vec<Vec3>,
    pub(super) indices: Vec<u32>,
    pub(super) vertex_colors: Option<Vec<Color>>,
    pub(super) tex_coords0: Option<Vec<[f32; 2]>>,
    pub(super) tangents: Option<Vec<[f32; 4]>>,
    pub(super) skin: Option<GeometrySkin>,
    pub(super) morph_targets: Vec<GeometryMorphTarget>,
}

pub(super) fn resolve(
    streams: PrimitiveStreams<'_>,
) -> Result<ResolvedPrimitiveStreams, AssetError> {
    let PrimitiveStreams {
        path,
        positions,
        normals,
        indices,
        vertex_colors,
        tex_coords0,
        tangents,
        skin,
        morph_targets,
    } = streams;
    if let Some(normals) = normals {
        return Ok(ResolvedPrimitiveStreams {
            positions,
            normals,
            indices,
            vertex_colors,
            tex_coords0,
            tangents,
            skin,
            morph_targets,
        });
    }
    expand_missing_flat_normals(MissingNormalStreams {
        path,
        positions,
        indices,
        vertex_colors,
        tex_coords0,
        tangents,
        skin,
        morph_targets,
    })
}

struct MissingNormalStreams<'a> {
    path: &'a AssetPath,
    positions: Vec<Vec3>,
    indices: Vec<u32>,
    vertex_colors: Option<Vec<Color>>,
    tex_coords0: Option<Vec<[f32; 2]>>,
    tangents: Option<Vec<[f32; 4]>>,
    skin: Option<GeometrySkin>,
    morph_targets: Vec<GeometryMorphTarget>,
}

fn expand_missing_flat_normals(
    streams: MissingNormalStreams<'_>,
) -> Result<ResolvedPrimitiveStreams, AssetError> {
    let MissingNormalStreams {
        path,
        positions,
        indices,
        vertex_colors,
        tex_coords0,
        tangents,
        skin,
        morph_targets,
    } = streams;
    if !indices.len().is_multiple_of(3) {
        return Err(missing_normal_error(
            path,
            format!(
                "cannot compute flat NORMAL values because the triangle index count {} is not divisible by 3",
                indices.len()
            ),
        ));
    }

    let mut expanded_positions = Vec::with_capacity(indices.len());
    let mut expanded_normals = Vec::with_capacity(indices.len());
    let mut expanded_colors = vertex_colors
        .as_ref()
        .map(|_| Vec::with_capacity(indices.len()));
    let mut expanded_tex_coords0 = tex_coords0
        .as_ref()
        .map(|_| Vec::with_capacity(indices.len()));
    let mut expanded_tangents = tangents.as_ref().map(|_| Vec::with_capacity(indices.len()));
    let mut expanded_joints = skin.as_ref().map(|_| Vec::with_capacity(indices.len()));
    let mut expanded_weights = skin.as_ref().map(|_| Vec::with_capacity(indices.len()));
    let mut expanded_morphs = morph_targets
        .iter()
        .map(|target| ExpandedMorphTarget {
            positions: Vec::with_capacity(indices.len()),
            normals: target
                .normal_deltas()
                .map(|_| Vec::with_capacity(indices.len())),
            tangents: target
                .tangent_deltas()
                .map(|_| Vec::with_capacity(indices.len())),
        })
        .collect::<Vec<_>>();

    for (triangle_index, triangle) in indices.chunks_exact(3).enumerate() {
        let source_indices = [
            source_index(path, triangle[0], positions.len())?,
            source_index(path, triangle[1], positions.len())?,
            source_index(path, triangle[2], positions.len())?,
        ];
        let [a, b, c] = source_indices.map(|index| positions[index]);
        let normal = (b - a).cross(c - a).try_normalize().ok_or_else(|| {
            missing_normal_error(
                path,
                format!("cannot compute flat NORMAL for degenerate triangle {triangle_index}"),
            )
        })?;
        for source_index in source_indices {
            expanded_positions.push(positions[source_index]);
            expanded_normals.push(normal);
            copy_optional(
                path,
                "COLOR_0",
                vertex_colors.as_deref(),
                &mut expanded_colors,
                source_index,
            )?;
            copy_optional(
                path,
                "TEXCOORD_0",
                tex_coords0.as_deref(),
                &mut expanded_tex_coords0,
                source_index,
            )?;
            copy_optional(
                path,
                "TANGENT",
                tangents.as_deref(),
                &mut expanded_tangents,
                source_index,
            )?;
            if let Some(skin) = &skin {
                expanded_joints.as_mut().expect("skin output exists").push(
                    *skin
                        .joints()
                        .get(source_index)
                        .ok_or_else(|| stream_count_error(path, "JOINTS_0", source_index))?,
                );
                expanded_weights.as_mut().expect("skin output exists").push(
                    *skin
                        .weights()
                        .get(source_index)
                        .ok_or_else(|| stream_count_error(path, "WEIGHTS_0", source_index))?,
                );
            }
            for (target_index, (source, expanded)) in
                morph_targets.iter().zip(&mut expanded_morphs).enumerate()
            {
                expanded
                    .positions
                    .push(*source.position_deltas().get(source_index).ok_or_else(|| {
                        stream_count_error(
                            path,
                            &format!("morph target {target_index} POSITION"),
                            source_index,
                        )
                    })?);
                copy_optional(
                    path,
                    &format!("morph target {target_index} NORMAL"),
                    source.normal_deltas(),
                    &mut expanded.normals,
                    source_index,
                )?;
                copy_optional(
                    path,
                    &format!("morph target {target_index} TANGENT"),
                    source.tangent_deltas(),
                    &mut expanded.tangents,
                    source_index,
                )?;
            }
        }
    }

    let skin = match (expanded_joints, expanded_weights) {
        (Some(joints), Some(weights)) => Some(GeometrySkin::new(joints, weights)),
        (None, None) => None,
        _ => unreachable!("skin streams are allocated together"),
    };
    let morph_targets = expanded_morphs
        .into_iter()
        .map(|target| {
            GeometryMorphTarget::new_with_semantics(
                target.positions,
                target.normals,
                target.tangents,
            )
        })
        .collect();
    Ok(ResolvedPrimitiveStreams {
        indices: (0..expanded_positions.len() as u32).collect(),
        positions: expanded_positions,
        normals: expanded_normals,
        vertex_colors: expanded_colors,
        tex_coords0: expanded_tex_coords0,
        tangents: expanded_tangents,
        skin,
        morph_targets,
    })
}

struct ExpandedMorphTarget {
    positions: Vec<Vec3>,
    normals: Option<Vec<Vec3>>,
    tangents: Option<Vec<Vec3>>,
}

fn source_index(path: &AssetPath, index: u32, vertex_count: usize) -> Result<usize, AssetError> {
    let index = index as usize;
    if index < vertex_count {
        Ok(index)
    } else {
        Err(missing_normal_error(
            path,
            format!(
                "cannot compute flat NORMAL because index {index} exceeds vertex count {vertex_count}"
            ),
        ))
    }
}

fn copy_optional<T: Copy>(
    path: &AssetPath,
    semantic: &str,
    source: Option<&[T]>,
    output: &mut Option<Vec<T>>,
    source_index: usize,
) -> Result<(), AssetError> {
    if let (Some(source), Some(output)) = (source, output) {
        output.push(
            *source
                .get(source_index)
                .ok_or_else(|| stream_count_error(path, semantic, source_index))?,
        );
    }
    Ok(())
}

fn stream_count_error(path: &AssetPath, semantic: &str, source_index: usize) -> AssetError {
    missing_normal_error(
        path,
        format!(
            "cannot compute flat NORMAL because {semantic} has no value for vertex {source_index}"
        ),
    )
}

fn missing_normal_error(path: &AssetPath, reason: String) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_owned(),
        reason: format!("glTF primitive missing NORMAL: {reason}"),
    }
}

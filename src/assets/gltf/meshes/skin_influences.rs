use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::geometry::GeometrySkin;

pub(super) struct SkinSet {
    pub(super) joints: Option<Vec<[usize; 4]>>,
    pub(super) weights: Option<Vec<[f32; 4]>>,
}

pub(super) struct SkinResolution {
    pub(super) skin: Option<GeometrySkin>,
    pub(super) truncated_vertices: usize,
}

type SkinVectors = (Vec<[usize; 4]>, Vec<[f32; 4]>);

pub(super) fn resolve(
    path: &AssetPath,
    primary: SkinSet,
    secondary: SkinSet,
) -> Result<SkinResolution, AssetError> {
    let primary = paired_set(path, 0, primary)?;
    let secondary = paired_set(path, 1, secondary)?;
    let Some((primary_joints, primary_weights)) = primary else {
        if secondary.is_some() {
            return Err(skin_error(
                path,
                "JOINTS_1/WEIGHTS_1 require JOINTS_0/WEIGHTS_0".to_owned(),
            ));
        }
        return Ok(SkinResolution {
            skin: None,
            truncated_vertices: 0,
        });
    };
    validate_vertex_counts(path, 0, &primary_joints, &primary_weights)?;

    let Some((secondary_joints, secondary_weights)) = secondary else {
        let weights = primary_weights
            .into_iter()
            .enumerate()
            .map(|(vertex_index, weights)| validate_and_normalize(path, vertex_index, 0, weights))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(SkinResolution {
            skin: Some(GeometrySkin::new(primary_joints, weights)),
            truncated_vertices: 0,
        });
    };
    validate_vertex_counts(path, 1, &secondary_joints, &secondary_weights)?;
    if primary_joints.len() != secondary_joints.len() {
        return Err(skin_error(
            path,
            format!(
                "JOINTS_1/WEIGHTS_1 vertex count {} must match JOINTS_0/WEIGHTS_0 vertex count {}",
                secondary_joints.len(),
                primary_joints.len()
            ),
        ));
    }

    let mut joints = Vec::with_capacity(primary_joints.len());
    let mut weights = Vec::with_capacity(primary_weights.len());
    let mut truncated_vertices = 0;
    for vertex_index in 0..primary_joints.len() {
        let mut influences = Vec::with_capacity(8);
        append_set(
            path,
            vertex_index,
            0,
            primary_joints[vertex_index],
            primary_weights[vertex_index],
            &mut influences,
        )?;
        append_set(
            path,
            vertex_index,
            1,
            secondary_joints[vertex_index],
            secondary_weights[vertex_index],
            &mut influences,
        )?;
        if influences
            .iter()
            .filter(|influence| influence.weight > 0.0)
            .count()
            > 4
        {
            truncated_vertices += 1;
        }
        influences.sort_by(|left, right| {
            right
                .weight
                .total_cmp(&left.weight)
                .then(left.ordinal.cmp(&right.ordinal))
        });
        let selected = &influences[..4];
        let selected_sum = selected
            .iter()
            .map(|influence| influence.weight)
            .sum::<f32>();
        if !selected_sum.is_finite() || selected_sum <= 0.0 {
            return Err(invalid_weights(
                path,
                vertex_index,
                "must have a finite non-zero sum",
            ));
        }
        joints.push([
            selected[0].joint,
            selected[1].joint,
            selected[2].joint,
            selected[3].joint,
        ]);
        weights.push([
            selected[0].weight / selected_sum,
            selected[1].weight / selected_sum,
            selected[2].weight / selected_sum,
            selected[3].weight / selected_sum,
        ]);
    }
    Ok(SkinResolution {
        skin: Some(GeometrySkin::new(joints, weights)),
        truncated_vertices,
    })
}

#[derive(Clone, Copy)]
struct Influence {
    joint: usize,
    weight: f32,
    ordinal: usize,
}

fn paired_set(
    path: &AssetPath,
    set: u32,
    input: SkinSet,
) -> Result<Option<SkinVectors>, AssetError> {
    match (input.joints, input.weights) {
        (Some(joints), Some(weights)) => Ok(Some((joints, weights))),
        (None, None) => Ok(None),
        _ => Err(skin_error(
            path,
            format!("JOINTS_{set} and WEIGHTS_{set} must be provided together"),
        )),
    }
}

fn validate_vertex_counts(
    path: &AssetPath,
    set: u32,
    joints: &[[usize; 4]],
    weights: &[[f32; 4]],
) -> Result<(), AssetError> {
    if joints.len() == weights.len() {
        Ok(())
    } else {
        Err(skin_error(
            path,
            format!(
                "JOINTS_{set} count {} must match WEIGHTS_{set} count {}",
                joints.len(),
                weights.len()
            ),
        ))
    }
}

fn append_set(
    path: &AssetPath,
    vertex_index: usize,
    set: u32,
    joints: [usize; 4],
    weights: [f32; 4],
    output: &mut Vec<Influence>,
) -> Result<(), AssetError> {
    validate_weights(path, vertex_index, set, weights)?;
    let ordinal_base = set as usize * 4;
    for influence in 0..4 {
        output.push(Influence {
            joint: joints[influence],
            weight: weights[influence],
            ordinal: ordinal_base + influence,
        });
    }
    Ok(())
}

fn validate_and_normalize(
    path: &AssetPath,
    vertex_index: usize,
    set: u32,
    mut weights: [f32; 4],
) -> Result<[f32; 4], AssetError> {
    validate_weights(path, vertex_index, set, weights)?;
    let sum = weights.iter().sum::<f32>();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(invalid_weights(
            path,
            vertex_index,
            "must have a finite non-zero sum",
        ));
    }
    for weight in &mut weights {
        *weight /= sum;
    }
    Ok(weights)
}

fn validate_weights(
    path: &AssetPath,
    vertex_index: usize,
    set: u32,
    weights: [f32; 4],
) -> Result<(), AssetError> {
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(invalid_set_weights(
            path,
            set,
            vertex_index,
            "must be finite",
        ));
    }
    if weights.iter().any(|weight| *weight < 0.0) {
        return Err(invalid_set_weights(
            path,
            set,
            vertex_index,
            "must be non-negative",
        ));
    }
    Ok(())
}

fn invalid_set_weights(
    path: &AssetPath,
    set: u32,
    vertex_index: usize,
    reason: &'static str,
) -> AssetError {
    skin_error(
        path,
        format!("WEIGHTS_{set} vertex {vertex_index} {reason}"),
    )
}

fn invalid_weights(path: &AssetPath, vertex_index: usize, reason: &'static str) -> AssetError {
    skin_error(
        path,
        format!("combined skin weights vertex {vertex_index} {reason}"),
    )
}

fn skin_error(path: &AssetPath, reason: String) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_owned(),
        reason: format!("glTF skin attributes: {reason}"),
    }
}

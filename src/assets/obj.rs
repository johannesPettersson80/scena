use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::scene::Vec3;

use super::{AssetFetcher, AssetPath, Assets, GeometryHandle};

impl<F: AssetFetcher> Assets<F> {
    pub async fn load_geometry(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<GeometryHandle, AssetError> {
        let path = path.into();
        let bytes = self.fetcher.fetch(&path).await?;
        let source = std::str::from_utf8(&bytes).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("expected UTF-8 OBJ source: {error}"),
        })?;
        let geometry = parse_obj_geometry(&path, source)?;
        Ok(self.create_geometry(geometry))
    }
}

fn parse_obj_geometry(path: &AssetPath, source: &str) -> Result<GeometryDesc, AssetError> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for line in source.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with("mtllib ") {
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => positions.push(parse_vec3(path, parts, "vertex position")?),
            Some("vn") => normals.push(parse_vec3(path, parts, "vertex normal")?),
            Some("f") => append_face(
                path,
                parts.collect::<Vec<_>>(),
                &positions,
                &normals,
                &mut vertices,
                &mut indices,
            )?,
            Some("vt" | "usemtl" | "o" | "g" | "s") | None => {}
            Some(other) => {
                return Err(parse_error(
                    path,
                    format!("unsupported OBJ record '{other}'"),
                ));
            }
        }
    }

    GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
        .map_err(|error| parse_error(path, format!("invalid OBJ geometry: {error:?}")))
}

fn append_face(
    path: &AssetPath,
    face: Vec<&str>,
    positions: &[Vec3],
    normals: &[Vec3],
    vertices: &mut Vec<GeometryVertex>,
    indices: &mut Vec<u32>,
) -> Result<(), AssetError> {
    if face.len() < 3 {
        return Err(parse_error(path, "OBJ face must have at least 3 vertices"));
    }
    let first = parse_face_vertex(path, face[0], positions, normals)?;
    for edge in face[1..].windows(2) {
        for vertex in [
            first,
            parse_face_vertex(path, edge[0], positions, normals)?,
            parse_face_vertex(path, edge[1], positions, normals)?,
        ] {
            let index = u32::try_from(vertices.len())
                .map_err(|_| parse_error(path, "OBJ geometry has too many vertices"))?;
            vertices.push(vertex);
            indices.push(index);
        }
    }
    Ok(())
}

fn parse_face_vertex(
    path: &AssetPath,
    token: &str,
    positions: &[Vec3],
    normals: &[Vec3],
) -> Result<GeometryVertex, AssetError> {
    let mut fields = token.split('/');
    let position = parse_obj_index(path, fields.next(), positions.len(), "position")?;
    let _texcoord = fields.next();
    let normal = fields
        .next()
        .filter(|field| !field.is_empty())
        .map(|field| parse_obj_index(path, Some(field), normals.len(), "normal"))
        .transpose()?
        .and_then(|index| normals.get(index).copied())
        .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
    let position = positions
        .get(position)
        .copied()
        .ok_or_else(|| parse_error(path, "OBJ face references missing position"))?;
    Ok(GeometryVertex { position, normal })
}

fn parse_obj_index(
    path: &AssetPath,
    field: Option<&str>,
    len: usize,
    label: &str,
) -> Result<usize, AssetError> {
    let value = field
        .filter(|field| !field.is_empty())
        .ok_or_else(|| parse_error(path, format!("OBJ face is missing {label} index")))?
        .parse::<isize>()
        .map_err(|error| parse_error(path, format!("invalid OBJ {label} index: {error}")))?;
    if value == 0 {
        return Err(parse_error(path, "OBJ indices are 1-based"));
    }
    let index = if value > 0 {
        value as usize - 1
    } else {
        len.checked_sub(value.unsigned_abs())
            .ok_or_else(|| parse_error(path, format!("OBJ negative {label} index out of range")))?
    };
    Ok(index)
}

fn parse_vec3<'a>(
    path: &AssetPath,
    mut parts: impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<Vec3, AssetError> {
    let x = parse_f32(path, parts.next(), label)?;
    let y = parse_f32(path, parts.next(), label)?;
    let z = parse_f32(path, parts.next(), label)?;
    Ok(Vec3::new(x, y, z))
}

fn parse_f32(path: &AssetPath, field: Option<&str>, label: &str) -> Result<f32, AssetError> {
    field
        .ok_or_else(|| parse_error(path, format!("missing OBJ {label} component")))?
        .parse::<f32>()
        .map_err(|error| parse_error(path, format!("invalid OBJ {label} component: {error}")))
}

fn parse_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}

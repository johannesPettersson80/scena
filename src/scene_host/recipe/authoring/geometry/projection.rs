use crate::scene::recipe::{
    SceneRecipeDiagnosticV1, SceneRecipeGeometryV1, SceneRecipePrimitiveV1,
};

use super::super::super::error_diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProjectedGeometryCounts {
    pub(super) vertices: u64,
    pub(super) indices: u64,
}

pub(super) fn projected_geometry_counts(
    recipe: &SceneRecipeGeometryV1,
) -> Result<ProjectedGeometryCounts, Box<SceneRecipeDiagnosticV1>> {
    if let Some(mesh) = &recipe.mesh {
        return Ok(ProjectedGeometryCounts {
            vertices: mesh.positions.len() as u64,
            indices: mesh.indices.len() as u64,
        });
    }
    let Some(primitive) = &recipe.primitive else {
        return Ok(ProjectedGeometryCounts {
            vertices: 0,
            indices: 0,
        });
    };
    projected_primitive_counts(primitive)
}

fn projected_primitive_counts(
    primitive: &SceneRecipePrimitiveV1,
) -> Result<ProjectedGeometryCounts, Box<SceneRecipeDiagnosticV1>> {
    let counts = match primitive.kind.as_str() {
        "box" => ProjectedGeometryCounts {
            vertices: 24,
            indices: 36,
        },
        "plane" => ProjectedGeometryCounts {
            vertices: 4,
            indices: 6,
        },
        "sphere" => {
            let segments = u64::from(primitive.segments.unwrap_or(32).max(3));
            let rings = u64::from(primitive.rings.unwrap_or(16).max(2));
            ProjectedGeometryCounts {
                vertices: checked_mul(segments + 1, rings + 1, "sphere vertices")?,
                indices: checked_mul(
                    checked_mul(segments, rings, "sphere faces")?,
                    6,
                    "sphere indices",
                )?,
            }
        }
        "cylinder" => {
            let segments = u64::from(primitive.segments.unwrap_or(32).max(3));
            ProjectedGeometryCounts {
                vertices: checked_add(
                    checked_mul(segments, 4, "cylinder vertices")?,
                    2,
                    "cylinder vertices",
                )?,
                indices: checked_mul(segments, 12, "cylinder indices")?,
            }
        }
        "cone" => {
            let segments = u64::from(primitive.segments.unwrap_or(32).max(3));
            ProjectedGeometryCounts {
                vertices: checked_add(
                    checked_mul(segments, 4, "cone vertices")?,
                    1,
                    "cone vertices",
                )?,
                indices: checked_mul(segments, 6, "cone indices")?,
            }
        }
        "disc" => {
            let segments = u64::from(primitive.segments.unwrap_or(32).max(3));
            ProjectedGeometryCounts {
                vertices: segments + 1,
                indices: checked_mul(segments, 3, "disc indices")?,
            }
        }
        "torus" => {
            let segments = u64::from(primitive.segments.unwrap_or(32).max(3));
            let rings = u64::from(primitive.rings.unwrap_or(12).max(3));
            ProjectedGeometryCounts {
                vertices: checked_mul(segments + 1, rings + 1, "torus vertices")?,
                indices: checked_mul(
                    checked_mul(segments, rings, "torus faces")?,
                    6,
                    "torus indices",
                )?,
            }
        }
        "wedge" => ProjectedGeometryCounts {
            vertices: 18,
            indices: 24,
        },
        "line" => ProjectedGeometryCounts {
            vertices: 2,
            indices: 2,
        },
        "polyline" => ProjectedGeometryCounts {
            vertices: primitive.points.len() as u64,
            indices: primitive.points.len().saturating_sub(1).saturating_mul(2) as u64,
        },
        "arrow" => ProjectedGeometryCounts {
            vertices: 2,
            indices: 2,
        },
        "grid" => {
            let divisions = u64::from(primitive.divisions.unwrap_or(10));
            let lines = checked_mul(divisions + 1, 2, "grid lines")?;
            ProjectedGeometryCounts {
                vertices: checked_mul(lines, 2, "grid vertices")?,
                indices: checked_mul(lines, 2, "grid indices")?,
            }
        }
        "axes" => ProjectedGeometryCounts {
            vertices: 6,
            indices: 6,
        },
        _ => ProjectedGeometryCounts {
            vertices: 0,
            indices: 0,
        },
    };
    Ok(counts)
}

fn checked_mul(left: u64, right: u64, what: &str) -> Result<u64, Box<SceneRecipeDiagnosticV1>> {
    left.checked_mul(right).ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "policy_violation",
            format!("{what} overflowed RecipeBuildPolicy projection"),
            "reduce primitive tessellation before building the recipe",
        ))
    })
}

fn checked_add(left: u64, right: u64, what: &str) -> Result<u64, Box<SceneRecipeDiagnosticV1>> {
    left.checked_add(right).ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "policy_violation",
            format!("{what} overflowed RecipeBuildPolicy projection"),
            "reduce primitive tessellation before building the recipe",
        ))
    })
}

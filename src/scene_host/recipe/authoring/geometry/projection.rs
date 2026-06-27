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
        "box" if primitive.bevel.or(primitive.fillet).is_some() => ProjectedGeometryCounts {
            vertices: 96,
            indices: 132,
        },
        "box" => ProjectedGeometryCounts {
            vertices: 24,
            indices: 36,
        },
        "plane" => ProjectedGeometryCounts {
            vertices: 4,
            indices: 6,
        },
        "sphere" => {
            let segments = u64::from(primitive.segments.unwrap_or(64).max(3));
            let rings = u64::from(primitive.rings.unwrap_or(48).max(2));
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
            if primitive.bevel.or(primitive.fillet).is_some() {
                ProjectedGeometryCounts {
                    vertices: checked_mul(segments, 18, "beveled cylinder vertices")?,
                    indices: checked_mul(segments, 24, "beveled cylinder indices")?,
                }
            } else {
                ProjectedGeometryCounts {
                    vertices: checked_add(
                        checked_mul(segments, 4, "cylinder vertices")?,
                        2,
                        "cylinder vertices",
                    )?,
                    indices: checked_mul(segments, 12, "cylinder indices")?,
                }
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
            vertices: 6,
            indices: 6,
        },
        "grid" => {
            let divisions = u64::from(primitive.divisions.unwrap_or(10).max(1));
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::scene::recipe::{SceneRecipeGeometryV1, SceneRecipePrimitiveV1};
    use crate::scene_host::recipe::authoring::geometry::authored_geometry;

    use super::projected_geometry_counts;

    #[test]
    fn projected_geometry_counts_match_authored_primitive_builders() {
        let cases = primitive_count_cases();
        for (name, primitive) in cases {
            let recipe = SceneRecipeGeometryV1 {
                id: name.to_owned(),
                mesh: None,
                primitive: Some(primitive),
            };
            let projected = projected_geometry_counts(&recipe)
                .unwrap_or_else(|diagnostic| panic!("{name} projection failed: {diagnostic:#?}"));
            let (_kind, geometry) = authored_geometry(&recipe, &BTreeMap::new())
                .unwrap_or_else(|diagnostic| panic!("{name} build failed: {diagnostic:#?}"));
            if name == "arrow-degenerate" {
                assert!(
                    projected.vertices >= geometry.vertices().len() as u64,
                    "{name} projected vertex count must fail closed"
                );
                assert!(
                    projected.indices >= geometry.indices().len() as u64,
                    "{name} projected index count must fail closed"
                );
            } else {
                assert_eq!(
                    projected.vertices,
                    geometry.vertices().len() as u64,
                    "{name} projected vertex count must match the builder"
                );
                assert_eq!(
                    projected.indices,
                    geometry.indices().len() as u64,
                    "{name} projected index count must match the builder"
                );
            }
        }
    }

    fn primitive_count_cases() -> Vec<(&'static str, SceneRecipePrimitiveV1)> {
        vec![
            (
                "box-default",
                primitive("box").with_size(vec![0.2, 0.1, 0.3]).build(),
            ),
            (
                "plane-default",
                primitive("plane").with_size(vec![0.2, 0.3]).build(),
            ),
            (
                "sphere-default",
                primitive("sphere").with_radius(0.2).build(),
            ),
            (
                "sphere-min-clamp",
                primitive("sphere")
                    .with_radius(0.2)
                    .with_segments(0)
                    .with_rings(0)
                    .build(),
            ),
            (
                "sphere-large",
                primitive("sphere")
                    .with_radius(0.2)
                    .with_segments(40)
                    .with_rings(18)
                    .build(),
            ),
            (
                "cylinder-default",
                primitive("cylinder")
                    .with_radius(0.2)
                    .with_height(0.4)
                    .build(),
            ),
            (
                "cylinder-min-clamp",
                primitive("cylinder")
                    .with_radius(0.2)
                    .with_height(0.4)
                    .with_segments(0)
                    .build(),
            ),
            (
                "cylinder-large",
                primitive("cylinder")
                    .with_radius(0.2)
                    .with_height(0.4)
                    .with_segments(48)
                    .build(),
            ),
            (
                "cone-default",
                primitive("cone").with_radius(0.2).with_height(0.4).build(),
            ),
            (
                "cone-min-clamp",
                primitive("cone")
                    .with_radius(0.2)
                    .with_height(0.4)
                    .with_segments(3)
                    .build(),
            ),
            (
                "cone-large",
                primitive("cone")
                    .with_radius(0.2)
                    .with_height(0.4)
                    .with_segments(48)
                    .build(),
            ),
            ("disc-default", primitive("disc").with_radius(0.2).build()),
            (
                "disc-min-clamp",
                primitive("disc").with_radius(0.2).with_segments(3).build(),
            ),
            (
                "disc-large",
                primitive("disc").with_radius(0.2).with_segments(48).build(),
            ),
            (
                "torus-default",
                primitive("torus")
                    .with_major_radius(0.3)
                    .with_minor_radius(0.08)
                    .build(),
            ),
            (
                "torus-min-clamp",
                primitive("torus")
                    .with_major_radius(0.3)
                    .with_minor_radius(0.08)
                    .with_segments(3)
                    .with_rings(3)
                    .build(),
            ),
            (
                "torus-large",
                primitive("torus")
                    .with_major_radius(0.3)
                    .with_minor_radius(0.08)
                    .with_segments(48)
                    .with_rings(20)
                    .build(),
            ),
            (
                "wedge-default",
                primitive("wedge").with_size(vec![0.2, 0.1, 0.3]).build(),
            ),
            (
                "line-default",
                primitive("line")
                    .with_start([0.0, 0.0, 0.0])
                    .with_end([1.0, 0.0, 0.0])
                    .build(),
            ),
            (
                "polyline-default",
                primitive("polyline")
                    .with_points(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]])
                    .build(),
            ),
            (
                "arrow-default",
                primitive("arrow")
                    .with_start([0.0, 0.0, 0.0])
                    .with_end([1.0, 0.0, 0.0])
                    .build(),
            ),
            (
                "arrow-degenerate",
                primitive("arrow")
                    .with_start([0.0, 0.0, 0.0])
                    .with_end([0.0, 0.0, 0.0])
                    .build(),
            ),
            (
                "grid-default",
                primitive("grid").with_size(vec![1.0]).build(),
            ),
            (
                "grid-min-clamp",
                primitive("grid")
                    .with_size(vec![1.0])
                    .with_divisions(0)
                    .build(),
            ),
            (
                "grid-large",
                primitive("grid")
                    .with_size(vec![1.0])
                    .with_divisions(64)
                    .build(),
            ),
            ("axes-default", primitive("axes").with_length(1.0).build()),
        ]
    }

    fn primitive(kind: &'static str) -> PrimitiveCase {
        PrimitiveCase(SceneRecipePrimitiveV1 {
            kind: kind.to_owned(),
            size: None,
            radius: None,
            major_radius: None,
            minor_radius: None,
            height: None,
            bevel: None,
            fillet: None,
            segments: None,
            rings: None,
            divisions: None,
            length: None,
            start: None,
            end: None,
            points: Vec::new(),
        })
    }

    struct PrimitiveCase(SceneRecipePrimitiveV1);

    impl PrimitiveCase {
        fn with_size(mut self, size: Vec<f64>) -> Self {
            self.0.size = Some(size);
            self
        }

        fn with_radius(mut self, radius: f64) -> Self {
            self.0.radius = Some(radius);
            self
        }

        fn with_major_radius(mut self, radius: f64) -> Self {
            self.0.major_radius = Some(radius);
            self
        }

        fn with_minor_radius(mut self, radius: f64) -> Self {
            self.0.minor_radius = Some(radius);
            self
        }

        fn with_height(mut self, height: f64) -> Self {
            self.0.height = Some(height);
            self
        }

        fn with_segments(mut self, segments: u32) -> Self {
            self.0.segments = Some(segments);
            self
        }

        fn with_rings(mut self, rings: u32) -> Self {
            self.0.rings = Some(rings);
            self
        }

        fn with_divisions(mut self, divisions: u32) -> Self {
            self.0.divisions = Some(divisions);
            self
        }

        fn with_length(mut self, length: f64) -> Self {
            self.0.length = Some(length);
            self
        }

        fn with_start(mut self, start: [f64; 3]) -> Self {
            self.0.start = Some(start);
            self
        }

        fn with_end(mut self, end: [f64; 3]) -> Self {
            self.0.end = Some(end);
            self
        }

        fn with_points(mut self, points: Vec<[f64; 3]>) -> Self {
            self.0.points = points;
            self
        }

        fn build(self) -> SceneRecipePrimitiveV1 {
            self.0
        }
    }
}

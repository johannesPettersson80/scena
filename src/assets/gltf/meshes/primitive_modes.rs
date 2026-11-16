//! glTF primitive modes. The renderer draws triangle lists and line lists, so
//! strips, fans, and loops are rewritten as lists while they are imported.

use ::gltf::mesh::Mode;

use crate::diagnostics::AssetError;
use crate::geometry::GeometryTopology;

use super::accessors::PrimitiveSource;

/// The list topology a primitive of `mode` is drawn as.
pub(super) fn topology(
    source: PrimitiveSource<'_>,
    mode: Mode,
) -> Result<GeometryTopology, AssetError> {
    match mode {
        Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan => {
            Ok(GeometryTopology::Triangles)
        }
        Mode::Lines | Mode::LineStrip | Mode::LineLoop => Ok(GeometryTopology::Lines),
        Mode::Points => Err(source.error(
            "mode",
            "POINTS is not supported; scena draws triangle and line primitives".to_owned(),
        )),
    }
}

/// Rewrites the vertex order of a strip, fan, or loop as list indices.
/// `TRIANGLES` and `LINES` are lists already and pass through unchanged.
///
/// `indices` is the primitive's vertex order: its index accessor, or
/// `0..vertex_count` for a primitive without one.
pub(super) fn list_indices(
    source: PrimitiveSource<'_>,
    mode: Mode,
    indices: Vec<u32>,
) -> Result<Vec<u32>, AssetError> {
    let (name, minimum, per_element) = match mode {
        Mode::TriangleStrip => ("TRIANGLE_STRIP", 3, 3),
        Mode::TriangleFan => ("TRIANGLE_FAN", 3, 3),
        Mode::LineStrip => ("LINE_STRIP", 2, 2),
        Mode::LineLoop => ("LINE_LOOP", 2, 2),
        Mode::Triangles | Mode::Lines | Mode::Points => return Ok(indices),
    };
    let count = indices.len();
    if count < minimum {
        return Err(source.error(
            "mode",
            format!("{name} needs at least {minimum} vertices, found {count}"),
        ));
    }
    let elements = match mode {
        Mode::LineLoop => count,
        Mode::LineStrip => count - 1,
        _ => count - 2,
    };
    // A strip of n vertices is three times n - 2 list indices, so the list is
    // reserved from the accessor byte budget before it is built.
    let stream = format!("{name} list indices");
    source.charge(&stream, elements * per_element, size_of::<u32>())?;
    Ok(match mode {
        Mode::TriangleStrip => triangles(elements, |triangle| {
            // Every second triangle swaps its last two vertices, which keeps
            // the winding of the whole strip the same as its first triangle.
            let [first, second] = if triangle % 2 == 0 { [1, 2] } else { [2, 1] };
            [
                indices[triangle],
                indices[triangle + first],
                indices[triangle + second],
            ]
        }),
        Mode::TriangleFan => triangles(elements, |triangle| {
            [indices[triangle + 1], indices[triangle + 2], indices[0]]
        }),
        _ => (0..elements)
            .flat_map(|segment| [indices[segment], indices[(segment + 1) % count]])
            .collect(),
    })
}

/// Builds a triangle list from `count` strip or fan triangles.
///
/// Exporters stitch strips together with triangles that repeat an index.
/// Those have no area and are dropped. A primitive made only of them keeps
/// them all, so it still imports with its vertices.
fn triangles(count: usize, corners: impl Fn(usize) -> [u32; 3]) -> Vec<u32> {
    let has_area = |[a, b, c]: [u32; 3]| a != b && b != c && a != c;
    let keep_all = !(0..count).any(|triangle| has_area(corners(triangle)));
    (0..count)
        .map(corners)
        .filter(|triangle| keep_all || has_area(*triangle))
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::triangles;

    #[test]
    fn stitching_triangles_are_dropped_unless_nothing_else_remains() {
        let strip = [[0, 1, 2], [1, 2, 2], [2, 2, 3], [2, 4, 3]];
        assert_eq!(
            triangles(strip.len(), |triangle| strip[triangle]),
            [0, 1, 2, 2, 4, 3],
        );

        let stitching = [[5, 5, 6], [5, 6, 6]];
        assert_eq!(
            triangles(stitching.len(), |triangle| stitching[triangle]),
            [5, 5, 6, 5, 6, 6],
        );
    }
}

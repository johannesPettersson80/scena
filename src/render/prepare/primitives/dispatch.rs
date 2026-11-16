use crate::diagnostics::PrepareError;
use crate::geometry::GeometryTopology;

use super::super::strokes;
use super::super::types::{
    DeformationInputs, GeometryPrimitiveSource, PrimitiveBakeParams, PrimitiveSinks,
};

pub(in crate::render) fn append_geometry_primitives(
    source: GeometryPrimitiveSource<'_>,
    deformation: DeformationInputs<'_>,
    params: PrimitiveBakeParams<'_>,
    sinks: PrimitiveSinks<'_>,
) -> Result<(), PrepareError> {
    let baked_per_instance = source.instance.is_some();
    let PrimitiveSinks {
        primitives,
        strokes: stroke_sink,
        transparent_primitives,
    } = sinks;
    let first_stroke = stroke_sink.len();
    let sinks = PrimitiveSinks {
        primitives,
        strokes: &mut *stroke_sink,
        transparent_primitives,
    };
    let appended = match source.geometry.topology() {
        GeometryTopology::Triangles => {
            super::append_triangle_primitives(source, deformation, params, sinks)
        }
        GeometryTopology::Lines => strokes::append_line_primitives(
            source.node,
            source.geometry,
            source.material,
            strokes::StrokeBakeInputs {
                tint: source.tint,
                clip_with_scene: source.clip_with_scene,
                params,
                sinks,
            },
        ),
    };
    if baked_per_instance {
        // These strokes carry `node x instance` placement and tint. Only the
        // node is recorded as their source, so they must not be re-placed
        // from it on the retained path.
        for stroke in &mut stroke_sink[first_stroke..] {
            stroke.mark_baked_per_instance();
        }
    }
    appended
}

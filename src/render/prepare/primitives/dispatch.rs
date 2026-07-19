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
    match source.geometry.topology() {
        GeometryTopology::Triangles => {
            super::append_triangle_primitives(source, deformation, params, sinks)
        }
        GeometryTopology::Lines => strokes::append_line_primitives(
            source.node,
            source.geometry,
            source.material,
            strokes::StrokeBakeInputs {
                tint: source.tint,
                params,
                sinks,
            },
        ),
    }
}

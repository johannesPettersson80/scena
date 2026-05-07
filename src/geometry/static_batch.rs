use super::GeometryDesc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticBatchReport {
    source_vertices: usize,
    source_indices: usize,
    instance_count: usize,
    output_vertices: usize,
    output_indices: usize,
}

impl GeometryDesc {
    pub fn static_batch_report(source: &GeometryDesc, instance_count: usize) -> StaticBatchReport {
        let instance_count = instance_count.max(1);
        StaticBatchReport {
            source_vertices: source.vertices.len(),
            source_indices: source.indices.len(),
            instance_count,
            output_vertices: source.vertices.len() * instance_count,
            output_indices: source.indices.len() * instance_count,
        }
    }
}

impl StaticBatchReport {
    pub const fn source_vertices(self) -> usize {
        self.source_vertices
    }

    pub const fn source_indices(self) -> usize {
        self.source_indices
    }

    pub const fn instance_count(self) -> usize {
        self.instance_count
    }

    pub const fn output_vertices(self) -> usize {
        self.output_vertices
    }

    pub const fn output_indices(self) -> usize {
        self.output_indices
    }

    pub const fn requires_prepare_after_rebuild(self) -> bool {
        true
    }

    pub const fn picking_debug_instances(self) -> usize {
        self.instance_count
    }
}

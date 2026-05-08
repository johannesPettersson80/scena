use super::{GeometryDesc, GeometryError};

impl GeometryDesc {
    pub fn with_tangents(mut self, tangents: Vec<[f32; 4]>) -> Result<Self, GeometryError> {
        if tangents.len() != self.vertices.len() {
            return Err(GeometryError::InvalidTangentCount {
                vertex_count: self.vertices.len(),
                tangent_count: tangents.len(),
            });
        }
        self.tangents = Some(tangents);
        Ok(self)
    }

    pub fn tangents(&self) -> Option<&[[f32; 4]]> {
        self.tangents.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenaViewerAnnotationAnchor {
    id: String,
    position: [f32; 3],
    normal: Option<[f32; 3]>,
    surface: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenaViewerAnnotationError {
    EmptyId,
    MissingPosition,
    InvalidVector { field: &'static str, value: String },
}

impl ScenaViewerAnnotationAnchor {
    pub fn from_attributes<I, K, V>(
        fallback_id: impl AsRef<str>,
        attributes: I,
    ) -> Result<Self, ScenaViewerAnnotationError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut id = non_empty_string(fallback_id.as_ref());
        let mut position = None;
        let mut normal = None;
        let mut surface = None;

        for (name, value) in attributes {
            let name = name.as_ref();
            let value = value.as_ref();
            match name {
                "id" | "data-id" | "data-annotation-id" => {
                    if let Some(value) = non_empty_string(value) {
                        id = Some(value);
                    }
                }
                "data-position" | "position" => {
                    position = Some(parse_vec3("data-position", value)?);
                }
                "data-normal" | "normal" => {
                    normal = Some(parse_vec3("data-normal", value)?);
                }
                "data-surface" | "surface" => {
                    surface = non_empty_string(value);
                }
                _ => {}
            }
        }

        Ok(Self {
            id: id.ok_or(ScenaViewerAnnotationError::EmptyId)?,
            position: position.ok_or(ScenaViewerAnnotationError::MissingPosition)?,
            normal,
            surface,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    pub const fn normal(&self) -> Option<[f32; 3]> {
        self.normal
    }

    pub fn surface(&self) -> Option<&str> {
        self.surface.as_deref()
    }

    pub fn is_surface_bound(&self) -> bool {
        self.surface.is_some()
    }
}

fn parse_vec3(field: &'static str, value: &str) -> Result<[f32; 3], ScenaViewerAnnotationError> {
    let normalized = value.replace(',', " ");
    let values = normalized
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ScenaViewerAnnotationError::InvalidVector {
            field,
            value: value.to_string(),
        })?;

    if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(ScenaViewerAnnotationError::InvalidVector {
            field,
            value: value.to_string(),
        });
    }

    Ok([values[0], values[1], values[2]])
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

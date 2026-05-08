use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectorMetadata {
    value: JsonValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorRollPolicy {
    Preserve,
    ChooseNearest,
    ExplicitAngle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorPolarity {
    Plug,
    Socket,
    Neutral,
}

impl ConnectorMetadata {
    pub const fn new(value: JsonValue) -> Self {
        Self { value }
    }

    pub const fn value(&self) -> &JsonValue {
        &self.value
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.value.get(key)
    }
}

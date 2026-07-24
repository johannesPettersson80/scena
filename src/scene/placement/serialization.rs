use super::*;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum StableTransformCompatibilityV1 {
    Canonical(SceneRecipeTransformV1),
    Legacy(LegacyStableTransformV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyStableTransformV1 {
    translation: [f64; 3],
    rotation: [f64; 4],
    scale: [f64; 3],
}

impl From<LegacyStableTransformV1> for SceneRecipeTransformV1 {
    fn from(transform: LegacyStableTransformV1) -> Self {
        Self::Raw {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
}

pub(super) fn stable_transform(transform: Transform) -> SceneRecipeTransformV1 {
    SceneRecipeTransformV1::Raw {
        translation: [
            round3(transform.translation.x),
            round3(transform.translation.y),
            round3(transform.translation.z),
        ],
        rotation: [
            round3(transform.rotation.x),
            round3(transform.rotation.y),
            round3(transform.rotation.z),
            round3(transform.rotation.w),
        ],
        scale: [
            round3(transform.scale.x),
            round3(transform.scale.y),
            round3(transform.scale.z),
        ],
    }
}

pub(super) fn serialize_transform_option<S>(
    transform: &Option<Transform>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    transform.map(stable_transform).serialize(serializer)
}

pub(super) fn deserialize_transform_option<'de, D>(
    deserializer: D,
) -> Result<Option<Transform>, D::Error>
where
    D: Deserializer<'de>,
{
    let transform = Option::<StableTransformCompatibilityV1>::deserialize(deserializer)?;
    transform
        .map(|transform| match transform {
            StableTransformCompatibilityV1::Canonical(transform) => transform,
            StableTransformCompatibilityV1::Legacy(transform) => transform.into(),
        })
        .map(|transform| Transform::try_from(&transform).map_err(D::Error::custom))
        .transpose()
}

use super::*;

pub(super) fn selected_scene_roots(
    path: &AssetPath,
    document: &::gltf::Document,
    nodes: &[scene_asset::SceneAssetNode],
    selection: &GltfSceneSelection,
) -> Result<(Vec<usize>, Option<SelectedGltfScene>), AssetError> {
    let scenes = document.scenes().collect::<Vec<_>>();
    let selected = match selection {
        GltfSceneSelection::Default => document.default_scene().or_else(|| scenes.first().cloned()),
        GltfSceneSelection::Index { index } => scenes.get(*index).cloned(),
        GltfSceneSelection::Name { name } => scenes
            .iter()
            .find(|scene| scene.name() == Some(name.as_str()))
            .cloned(),
    };
    if let Some(scene) = selected {
        let selection = match selection {
            GltfSceneSelection::Default => "default_or_first",
            GltfSceneSelection::Index { .. } => "explicit_index",
            GltfSceneSelection::Name { .. } => "explicit_name",
        };
        let info = SelectedGltfScene {
            index: scene.index(),
            name: scene.name().map(str::to_owned),
            selection: selection.to_owned(),
        };
        return Ok((scene.nodes().map(|node| node.index()).collect(), Some(info)));
    }
    // An explicit request is never satisfied by the root-node fallback, so it
    // fails closed whether the scene table is missing entries or absent
    // entirely. Only the default selection may fall back.
    if !matches!(selection, GltfSceneSelection::Default) {
        let requested = match selection {
            GltfSceneSelection::Index { index } => format!("index {index}"),
            GltfSceneSelection::Name { name } => format!("name {name:?}"),
            GltfSceneSelection::Default => unreachable!(),
        };
        let candidates = if scenes.is_empty() {
            "the document declares no scenes".to_owned()
        } else {
            scenes
                .iter()
                .map(|scene| {
                    format!(
                        "{}:{:?}",
                        scene.index(),
                        scene.name().unwrap_or("<unnamed>")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Err(AssetError::Parse {
            path: path.as_str().to_owned(),
            reason: format!(
                "requested glTF scene {requested} was not found; available scenes: {candidates}"
            ),
        });
    }

    let mut child_indices = std::collections::BTreeSet::new();
    for node in nodes {
        child_indices.extend(node.children.iter().copied());
    }
    Ok((
        (0..nodes.len())
            .filter(|index| !child_indices.contains(index))
            .collect(),
        None,
    ))
}

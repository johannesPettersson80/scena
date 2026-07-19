pub(super) fn resolve_clip(
    host: &mut scena::SceneHostCore,
    manifest: &scena::SceneRecipeBuildV1,
    clip_name: &str,
) -> Result<(u64, f32), String> {
    if let Some(animation) = manifest
        .animations
        .iter()
        .find(|animation| animation.id == clip_name)
    {
        return Ok((animation.handle, animation.duration_seconds));
    }

    for import in &manifest.imports {
        let Ok(inventory) = host.animation_inventory_json(import.import_handle) else {
            continue;
        };
        let inventory: scena::SceneHostAnimationInventoryV1 = serde_json::from_str(&inventory)
            .map_err(|error| format!("failed to decode animation inventory: {error}"))?;
        let Some(clip) = inventory.clips.iter().find(|clip| clip.name == clip_name) else {
            continue;
        };
        let handle = host
            .play_animation(
                import.import_handle,
                clip_name,
                scena::SceneHostAnimationPlayOptions::default(),
            )
            .map_err(|error| format!("failed to play imported clip '{clip_name}': {error}"))?;
        return Ok((handle, clip.duration_seconds));
    }

    let mut available = manifest
        .animations
        .iter()
        .map(|animation| animation.id.clone())
        .collect::<Vec<_>>();
    for import in &manifest.imports {
        let Ok(inventory) = host.animation_inventory_json(import.import_handle) else {
            continue;
        };
        if let Ok(inventory) =
            serde_json::from_str::<scena::SceneHostAnimationInventoryV1>(&inventory)
        {
            available.extend(inventory.clips.into_iter().map(|clip| clip.name));
        }
    }
    available.sort();
    available.dedup();
    Err(format!(
        "animation clip '{clip_name}' was not found; available clips: {}",
        if available.is_empty() {
            "<none>".to_owned()
        } else {
            available.join(", ")
        }
    ))
}

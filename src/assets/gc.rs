use std::collections::BTreeSet;

use super::{
    AssetEvictionStats, Assets, EnvironmentHandle, GeometryHandle, MaterialHandle, SceneAsset,
    TextureHandle,
};

impl<F> Assets<F> {
    /// Frees `GeometryDesc` / `MaterialDesc` / `TextureDesc` /
    /// `EnvironmentDesc` slotmap entries that no cached `SceneAsset`,
    /// material descriptor, or environment lookup still references AND
    /// that were not minted directly by [`Assets::create_geometry`] /
    /// [`Assets::create_material`] / [`Assets::load_texture`].
    ///
    /// This helper is hot-reload-scoped GC, not a generic eviction sweep:
    /// it is intended for long-running [`Assets::reload_scene`] sessions
    /// where the replacement scene's `geometry/material/texture`
    /// descriptors accumulate in the slotmaps because only the latest
    /// `SceneAsset` per path is retained in `scene_lookup`. User-created
    /// descriptors (every `Assets::create_*` call) are tracked and
    /// always retained so a procedural-scene caller cannot lose handles
    /// they still hold; this is the contract a beginner expects after
    /// reading the typed `*HandleNotFound` error documentation.
    ///
    /// Reachability is rooted at:
    /// - every `SceneAsset` in `scene_lookup` (its `nodes()`'s `meshes()`
    ///   contribute their `GeometryHandle` and `MaterialHandle`);
    /// - every cached `EnvironmentHandle` in `environment_lookup`;
    /// - the texture slots of every reachable material descriptor.
    ///
    /// A `SceneAsset` returned by [`Assets::load_scene`] but later
    /// overwritten in `scene_lookup` (for example by a follow-up
    /// `load_scene` for the same path) is no longer reachable here. If a
    /// caller still holds an older `SceneAsset` or an instantiated scene
    /// backed by it, call [`Assets::release_unreferenced_with_scene_roots`]
    /// and pass those live scene roots instead of using this cache-rooted
    /// convenience method.
    /// Returns a per-store eviction count.
    ///
    /// Closes scena-gltf-animation-reviewer Phase 6 finding F4 and
    /// scena-api-ergonomics-reviewer 4b0e621 finding N2.
    pub fn release_unreferenced(&self) -> AssetEvictionStats {
        self.release_unreferenced_with_scene_roots(std::iter::empty::<&SceneAsset>())
    }

    /// Frees unreferenced asset descriptors while treating both cached
    /// `SceneAsset`s and caller-provided live scene roots as reachability
    /// roots.
    ///
    /// Use this variant after hot reload when application state may still
    /// hold an older `SceneAsset` or an instantiated scene that was built
    /// from it. Passing the old `SceneAsset` keeps its glTF-derived
    /// geometry/material/texture descriptors alive until the host has
    /// detached all scene nodes that rely on it.
    pub fn release_unreferenced_with_scene_roots<'a>(
        &self,
        scene_roots: impl IntoIterator<Item = &'a SceneAsset>,
    ) -> AssetEvictionStats {
        let mut storage = self.storage();
        let mut referenced_geometries: BTreeSet<GeometryHandle> = BTreeSet::new();
        let mut referenced_materials: BTreeSet<MaterialHandle> = BTreeSet::new();
        let mut referenced_textures: BTreeSet<TextureHandle> = BTreeSet::new();
        let mut referenced_environments: BTreeSet<EnvironmentHandle> = BTreeSet::new();

        for scene in storage.scene_lookup.values() {
            for node in scene.nodes() {
                for mesh in node.meshes() {
                    referenced_geometries.insert(mesh.geometry());
                    referenced_materials.insert(mesh.material());
                }
            }
        }
        for scene in scene_roots {
            for node in scene.nodes() {
                for mesh in node.meshes() {
                    referenced_geometries.insert(mesh.geometry());
                    referenced_materials.insert(mesh.material());
                }
            }
        }
        for environment in storage.environment_lookup.values().copied() {
            referenced_environments.insert(environment);
        }
        for material_handle in referenced_materials.iter().copied() {
            if let Some(material) = storage.materials.get(material_handle) {
                for handle in [
                    material.base_color_texture(),
                    material.normal_texture(),
                    material.metallic_roughness_texture(),
                    material.occlusion_texture(),
                    material.emissive_texture(),
                ]
                .into_iter()
                .flatten()
                {
                    referenced_textures.insert(handle);
                }
            }
        }

        let mut stats = AssetEvictionStats::default();

        let geometry_keys: Vec<GeometryHandle> = storage.geometries.keys().collect();
        for handle in geometry_keys {
            // User-created descriptors (minted via `Assets::create_<kind>`)
            // are ALWAYS retained - release_unreferenced is hot-reload-scoped
            // GC, not a generic eviction sweep. Closes
            // scena-api-ergonomics-reviewer 4b0e621 finding N2.
            if !referenced_geometries.contains(&handle)
                && !storage.user_created_geometries.contains(&handle)
            {
                storage.geometries.remove(handle);
                storage.user_created_geometries.remove(&handle);
                stats.geometries_evicted += 1;
            }
        }
        let material_keys: Vec<MaterialHandle> = storage.materials.keys().collect();
        for handle in material_keys {
            if !referenced_materials.contains(&handle)
                && !storage.user_created_materials.contains(&handle)
            {
                storage.materials.remove(handle);
                storage.user_created_materials.remove(&handle);
                stats.materials_evicted += 1;
            }
        }
        let texture_keys: Vec<TextureHandle> = storage.textures.keys().collect();
        for handle in texture_keys {
            if !referenced_textures.contains(&handle)
                && !storage.user_created_textures.contains(&handle)
            {
                storage.textures.remove(handle);
                storage.user_created_textures.remove(&handle);
                stats.textures_evicted += 1;
            }
        }
        // Drop texture_lookup entries that pointed at evicted textures so
        // a stable retained-reload identity does not resurrect a dead handle.
        let live_textures: BTreeSet<TextureHandle> = storage.textures.keys().collect();
        storage
            .texture_lookup
            .retain(|_, handle| live_textures.contains(handle));
        let environment_keys: Vec<EnvironmentHandle> = storage.environments.keys().collect();
        for handle in environment_keys {
            if !referenced_environments.contains(&handle)
                && !storage.user_created_environments.contains(&handle)
            {
                storage.environments.remove(handle);
                storage.user_created_environments.remove(&handle);
                stats.environments_evicted += 1;
            }
        }
        stats
    }
}

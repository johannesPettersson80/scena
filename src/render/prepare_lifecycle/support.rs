use super::*;

impl Renderer {
    pub(super) fn prepare_target(&self) -> Result<super::super::RasterTarget, ()> {
        let scale = if self.gpu.is_some()
            && self.target.backend == crate::diagnostics::Backend::HeadlessGpu
        {
            self.supersample_factor
        } else {
            1
        };
        let target = super::super::target::validate_supersample_target(self.target, scale)
            .map_err(|_| ())?;
        let workaround_required = self
            .gpu
            .as_ref()
            .is_some_and(super::super::gpu::GpuDeviceState::requires_v3d_headless_target_alignment);
        Ok(super::super::target::v3d_headless_render_target(
            target,
            workaround_required,
        ))
    }

    pub(super) fn screen_space_prepare_scale(&self) -> f32 {
        if self.gpu.is_none() {
            self.supersample_factor.max(1) as f32
        } else if self.target.backend == crate::diagnostics::Backend::HeadlessGpu {
            self.prepare_target()
                .map_or(1.0, |target| target.width as f32 / self.target.width as f32)
        } else {
            1.0
        }
    }

    pub(in crate::render) fn dynamic_gpu_prepare_rejection_reason(
        &self,
        scene: &Scene,
        backend_material_handles: &[crate::assets::MaterialHandle],
    ) -> Option<&'static str> {
        let Some(prepared) = self.prepared.as_ref() else {
            return Some("no prepared template");
        };
        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Some("scene identity changed");
        }
        if prepared.structure_revision != scene.structure_revision() {
            return Some("structure revision changed");
        }
        if prepared.camera_revision != scene.camera_revision() {
            return Some("camera descriptor revision changed");
        }
        if prepared.environment_revision != self.environment_revision {
            return Some("environment revision changed");
        }
        if prepared.target_revision != self.target_revision {
            return Some("target revision changed");
        }
        if prepared.output_resources_revision != self.output_resources_revision {
            return Some("output resources revision changed");
        }
        if prepared.transform_revision == scene.transform_revision() {
            return None;
        }
        if scene.section_box().is_some() {
            return Some("section caps require a full prepare after transforms change");
        }
        if scene.reflection_probes().next().is_some() {
            return Some("reflection probe selection requires full prepare");
        }
        if scene.model_nodes().next().is_some() {
            return Some("model nodes present");
        }
        if scene.has_mesh_lods() {
            return Some("mesh LOD selection is view-dependent");
        }
        if scene.label_nodes().next().is_some() {
            return Some("label nodes present");
        }
        if scene
            .mesh_nodes()
            .any(|(node, _mesh, _transform)| scene.skin_binding(node).is_some())
        {
            return Some("skinned joints may have moved");
        }
        if prepare::gpu_tiled_light_assignment_required(scene) {
            return Some("tiled light assignment may have moved");
        }
        if scene
            .mesh_nodes()
            .any(|(_node, mesh, _transform)| !backend_material_handles.contains(&mesh.material()))
        {
            return Some("moving mesh missing GPU material slot");
        }
        if !prepared
            .primitives
            .iter()
            .all(|primitive| !primitive.gpu_triangle_path() || primitive.depth_prepass_eligible())
        {
            return Some("non-opaque primitive present");
        }
        None
    }

    pub(super) fn reencode_retained_draws(
        &self,
        scene: &Scene,
    ) -> Option<(
        Vec<prepare::PreparedPrimitive>,
        Vec<prepare::PreparedStrokeSegment>,
        Vec<prepare::PreparedInstanceSet>,
    )> {
        let prepared = self.prepared.as_ref()?;
        if !retained_template_covers_visible_sources(
            scene,
            &prepared.retained_primitives,
            &prepared.retained_strokes,
            &prepared.retained_labels,
            &prepared.retained_instances,
        ) {
            return None;
        }
        let primitives =
            filter_retained_primitives_for_scene(scene, &prepared.retained_primitives)?
                .into_iter()
                .filter(prepare::PreparedPrimitive::gpu_triangle_path)
                .collect();
        let strokes = filter_retained_strokes_for_scene(scene, &prepared.retained_strokes)?;
        let instances = filter_retained_instances_for_scene(scene, &prepared.retained_instances)?;
        let labels = filter_retained_labels_for_scene(scene, &prepared.retained_labels)?;
        if !labels.is_empty() {
            return None;
        }
        Some((primitives, strokes, instances))
    }

    pub(super) fn apply_prepare_stats(
        &mut self,
        logical_stats: prepare::PreparedLogicalResourceStats,
        environment_prepare_stats: prepare::PreparedEnvironmentStats,
        lighting_stats: prepare::PreparedLightingStats,
        depth_stats: prepare::PreparedDepthStats,
        culled_objects: u64,
        backend_material_slots: &[prepare::PreparedMaterialSlot],
    ) {
        self.stats.materials = logical_stats.materials;
        self.stats.material_bindings = logical_stats.material_bindings;
        self.stats.material_texture_bindings = logical_stats.material_texture_bindings;
        self.stats.material_sampler_bindings = logical_stats.material_sampler_bindings;
        self.stats.material_textures_missing_decoded_pixels =
            logical_stats.material_textures_missing_decoded_pixels;
        self.stats.material_batch_layers =
            prepare::compute_material_batch_plan(backend_material_slots).layer_count;
        self.stats.environments = logical_stats.environments;
        self.stats.environment_cubemaps = environment_prepare_stats.cubemaps;
        self.stats.environment_prefilter_passes = environment_prepare_stats.prefilter_passes;
        self.stats.environment_brdf_luts = environment_prepare_stats.brdf_luts;
        self.stats.live_logical_handles = logical_stats.live_logical_handles;
        self.stats.shadow_maps = lighting_stats.shadow_maps;
        self.stats.depth_prepass_passes = depth_stats.passes;
        self.stats.depth_prepass_draws = depth_stats.draws;
        self.stats.directional_shadow_map_resolution =
            lighting_stats.directional_shadow_map_resolution;
        self.stats.directional_shadow_pcf_kernel = lighting_stats.directional_shadow_pcf_kernel;
        self.stats.culled_objects = culled_objects;
    }

    #[cfg(test)]
    pub(super) fn retained_draw_source_counts_for_test(
        &self,
        scene: &Scene,
        mesh_node: crate::scene::NodeKey,
        stroke_node: crate::scene::NodeKey,
    ) -> Option<(usize, usize)> {
        let (primitives, strokes, _instances) = self.reencode_retained_draws(scene)?;
        Some((
            primitives
                .iter()
                .filter(|primitive| primitive.source_node() == Some(mesh_node))
                .count(),
            strokes
                .iter()
                .filter(|stroke| stroke.source_node() == Some(stroke_node))
                .count(),
        ))
    }

    pub(super) fn apply_gpu_resource_stats(
        &mut self,
        stats: gpu::GpuResourceStats,
        pending_destructions: u64,
        logical_texture_count: u64,
    ) {
        self.stats.buffers = stats.buffers;
        self.stats.gpu_textures = stats.textures;
        self.stats.textures = logical_texture_count;
        self.stats.render_targets = stats.render_targets;
        self.stats.pipelines = stats.pipelines;
        self.stats.bind_groups = stats.bind_groups;
        self.stats.shader_modules = stats.shader_modules;
        self.stats.pending_destructions = pending_destructions;
        self.stats.material_bind_groups = stats.material_bind_groups;
        self.stats.approximate_gpu_memory_bytes =
            (stats.approximate_gpu_memory_bytes > 0).then_some(stats.approximate_gpu_memory_bytes);
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::Assets;
    use crate::geometry::GeometryDesc;
    use crate::material::{Color, MaterialDesc};
    use crate::scene::{Scene, Transform, Vec3};

    use super::Renderer;

    /// Multi-list retained re-entry.
    ///
    /// A scene holding both a triangle mesh and a line mesh must drop **and
    /// restore** the primitive draw list and the stroke draw list together.
    /// `reencode_retained_draws` filters the two through separate functions, so
    /// a regression can restore one list and leave the other empty; neither the
    /// mesh-only nor the stroke-only test can observe that.
    #[test]
    fn retained_re_entry_restores_mesh_and_stroke_draw_lists_together() {
        let mut renderer = Renderer::headless(48, 48).expect("CPU renderer builds");
        let assets = Assets::new();
        let mesh_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
        let mesh_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let line_geometry = assets.create_geometry(GeometryDesc::line(
            Vec3::new(0.0, -0.5, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
        ));
        let line_material = assets.create_material(MaterialDesc::line(Color::WHITE, 3.0));

        let mut scene = Scene::new();
        scene.add_default_camera().expect("camera inserts");
        let mesh_node = scene
            .mesh(mesh_geometry, mesh_material)
            .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
            .add()
            .expect("triangle mesh inserts");
        let stroke_node = scene
            .mesh(line_geometry, line_material)
            .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
            .add()
            .expect("line mesh inserts");

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("initial prepare succeeds");

        let visible = renderer
            .retained_draw_source_counts_for_test(&scene, mesh_node, stroke_node)
            .expect("the retained template covers both sources");
        assert!(
            visible.0 > 0,
            "the retained template must produce the triangle mesh's primitives",
        );
        assert!(
            visible.1 > 0,
            "the retained template must produce the line mesh's stroke segments",
        );

        scene.set_visible(mesh_node, false).expect("mesh hides");
        scene.set_visible(stroke_node, false).expect("line hides");
        assert_eq!(
            renderer
                .retained_draw_source_counts_for_test(&scene, mesh_node, stroke_node)
                .expect("hidden sources keep the template valid"),
            (0, 0),
            "leaving the active camera must drop the primitive list and the stroke list together",
        );

        scene.set_visible(mesh_node, true).expect("mesh re-enters");
        scene
            .set_visible(stroke_node, true)
            .expect("line re-enters");
        assert_eq!(
            renderer
                .retained_draw_source_counts_for_test(&scene, mesh_node, stroke_node)
                .expect("re-entry keeps the template valid"),
            visible,
            "re-entry must restore both draw lists from the retained template, not only one",
        );
    }
}

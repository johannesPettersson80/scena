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
        super::super::target::validate_supersample_target(self.target, scale).map_err(|_| ())
    }

    pub(super) fn screen_space_prepare_scale(&self) -> f32 {
        if self.gpu.is_none() || self.target.backend == crate::diagnostics::Backend::HeadlessGpu {
            self.supersample_factor.max(1) as f32
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

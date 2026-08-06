use super::*;

mod accessors;

mod prepared_state;

mod surface;

impl Renderer {
    pub fn render(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RenderOutcome, RenderError> {
        self.render_with_readback_mode(scene, camera, RenderReadbackMode::Automatic)
    }

    pub fn render_with_readback_mode(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
        readback_mode: RenderReadbackMode,
    ) -> Result<RenderOutcome, RenderError> {
        self.last_render_work_metrics = RenderWorkMetrics::default();
        self.loss_error()?;
        self.prepared_state(scene)?;
        if scene.camera(camera).is_none() {
            return Err(RenderError::CameraNotFound(camera));
        }

        let surface_auto_exposure = self.auto_exposure.is_some()
            && matches!(
                auto_exposure_frame_policy(
                    self.gpu.is_some(),
                    self.gpu
                        .as_ref()
                        .is_some_and(gpu::GpuDeviceState::has_surface),
                ),
                AutoExposureFramePolicy::PriorAsyncMeterSample,
            );
        if surface_auto_exposure {
            self.apply_pending_surface_auto_exposure()?;
        }

        let dirty_state = scene.dirty_state();
        if readback_mode != RenderReadbackMode::Synchronous
            && self.render_mode == RenderMode::OnChange
            && self.last_rendered_generation == Some(self.render_generation)
            && self
                .last_rendered_frame
                .is_some_and(|state| state.matches(dirty_state, camera))
        {
            self.stats.skipped_frames = self.stats.skipped_frames.saturating_add(1);
            return Ok(RenderOutcome {
                width: self.target.width,
                height: self.target.height,
                draw_calls: 0,
                primitives: 0,
                skipped: true,
            });
        }

        let gpu_target = if self.gpu.is_some() {
            Some(self.gpu_render_target()?)
        } else {
            None
        };
        let camera_projection =
            camera::CameraProjection::from_scene(scene, camera, gpu_target.unwrap_or(self.target))?;
        let primitive_count = prepared_triangle_alias_count(self.prepared_state(scene)?);
        let mut auto_exposure_attempted = false;
        let mut gpu_draw_submissions = 0;
        loop {
            if self.gpu.is_some() {
                let format_probes_before = self
                    .gpu
                    .as_ref()
                    .map_or(0, gpu::GpuDeviceState::sample_count_capability_probe_count);
                let (clipping_planes, section_box) = {
                    let prepared = self.prepared_state(scene)?;
                    (prepared.clipping_planes.clone(), prepared.section_box)
                };
                let gpu_result = match self.draw_gpu(
                    gpu_target.expect("GPU render target exists when GPU is active"),
                    &camera_projection,
                    clipping_planes.as_ref(),
                    section_box,
                    readback_mode,
                    surface_auto_exposure,
                ) {
                    Ok(result) => result,
                    Err(RenderError::SurfaceLost { recoverable }) => {
                        self.surface_lost = Some(recoverable);
                        return Err(RenderError::SurfaceLost { recoverable });
                    }
                    Err(error) => return Err(error),
                };
                self.last_render_work_metrics.add_gpu_result(gpu_result);
                let format_probes_after = self
                    .gpu
                    .as_ref()
                    .map_or(0, gpu::GpuDeviceState::sample_count_capability_probe_count);
                self.last_render_work_metrics.gpu_format_feature_probes = self
                    .last_render_work_metrics
                    .gpu_format_feature_probes
                    .saturating_add(format_probes_after.saturating_sub(format_probes_before));
                if let Some(outcome) =
                    surface::record_surface_result(&mut self.stats, self.target, gpu_result)
                {
                    return Ok(outcome);
                }
                gpu_draw_submissions = gpu_result.draw_submissions;
                self.stats.order_independent_transparency_passes = 0;
                self.stats.ambient_occlusion_passes = gpu_result.post_counts.ambient_occlusion;
                self.stats.screen_space_reflection_passes =
                    gpu_result.post_counts.screen_space_reflections;
                self.stats.bloom_passes = gpu_result.post_counts.bloom;
                self.stats.depth_of_field_passes = gpu_result.post_counts.depth_of_field;
                self.stats.fxaa_passes = gpu_result.post_counts.fxaa;
            } else {
                let cpu_projection =
                    camera::CameraProjection::from_scene(scene, camera, self.target)?;
                self.draw_cpu(scene, camera, &cpu_projection)?;
            }
            let auto_exposure_source_available = self.gpu.is_none()
                || self.last_render_work_metrics.readback_copies > 0
                || cfg!(target_arch = "wasm32");
            if surface_auto_exposure {
                break;
            }
            if auto_exposure_attempted
                || !auto_exposure_source_available
                || !self.apply_managed_auto_exposure_after_render()
            {
                break;
            }
            auto_exposure_attempted = true;
        }
        self.stats.frames_rendered = self.stats.frames_rendered.saturating_add(1);
        self.stats.draw_calls = primitive_count;
        self.stats.triangles = primitive_count;
        self.stats.primitives = primitive_count;
        self.stats.instances = self
            .prepared_state(scene)
            .map(|prepared| {
                prepared
                    .instances
                    .iter()
                    .map(|set| set.instances().len() as u64)
                    .sum()
            })
            .unwrap_or(0);
        self.stats.gpu_draw_submissions = gpu_draw_submissions;
        self.last_rendered_generation = Some(self.render_generation);
        let rendered_frame = RenderedFrameState {
            dirty_state,
            camera,
            width: self.target.width,
            height: self.target.height,
            capabilities: self.capabilities,
            render_generation: self.render_generation,
            target_revision: self.target_revision,
            output_resources_revision: self.output_resources_revision,
            output_color_space: self.output_color_space,
            exposure_ev: self.output.exposure_ev(),
            tonemapper: match self.output.tonemapper() {
                Tonemapper::Aces => "aces",
                Tonemapper::Standard => "standard",
                Tonemapper::PbrNeutral => "pbr_neutral",
            },
            anti_aliasing: match self.anti_aliasing {
                AntiAliasing::None => "none",
                AntiAliasing::Fxaa => "fxaa",
                AntiAliasing::Msaa4 => "msaa4",
                AntiAliasing::Msaa8 => "msaa8",
            },
            supersample_factor: self.supersample_factor,
            bloom: self.bloom.is_some(),
            screen_space_ambient_occlusion: self.screen_space_ambient_occlusion.is_some(),
            screen_space_reflections: self.screen_space_reflections.is_some(),
            depth_of_field: self.depth_of_field.is_some(),
            readback_completed_unix_ms: None,
        };
        let composition_frame_key =
            super::state::CompositionFrameKey::from_rendered_frame(rendered_frame);
        debug_assert_eq!(
            composition_frame_key.staleness_against_rendered_frame(rendered_frame),
            None
        );
        self.last_rendered_frame = Some(rendered_frame);
        self.last_readback_frame = (self.gpu.is_none()
            || self.last_render_work_metrics.readback_copies > 0)
            .then(|| rendered_frame.with_readback_completed_now());

        Ok(RenderOutcome {
            width: self.target.width,
            height: self.target.height,
            draw_calls: primitive_count,
            primitives: primitive_count,
            skipped: false,
        })
    }

    /// Renders a camera sequence and reads the frames back in input order
    /// through two alternating prepared GPU buffers.
    ///
    /// Rendering and map submission stay nonblocking until both slots are in
    /// flight; the oldest slot is then resolved before it is reused. The
    /// returned frames always follow `cameras`, and the final frame also
    /// becomes the renderer's current `frame_rgba8()` value.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn render_batch_with_async_readback(
        &mut self,
        scene: &Scene,
        cameras: &[CameraKey],
    ) -> Result<Vec<PixelReadback>, RenderError> {
        self.prepared_state(scene)?;
        if self.gpu.is_none() {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: self.target.backend,
            });
        }
        if cameras.is_empty() {
            self.last_render_work_metrics = RenderWorkMetrics::default();
            return Ok(Vec::new());
        }
        let target = self.gpu_render_target()?;
        let mut pending = VecDeque::with_capacity(2);
        let mut completed = Vec::with_capacity(cameras.len());
        let mut peak_in_flight = 0_u64;
        for (order, camera) in cameras.iter().copied().enumerate() {
            self.render_with_readback_mode(scene, camera, RenderReadbackMode::PresentOnly)?;
            if pending.len() == 2 {
                let oldest = pending.pop_front().expect("two-slot queue is nonempty");
                completed.push(
                    self.gpu
                        .as_mut()
                        .expect("GPU existence checked above")
                        .finish_async_readback(oldest)?,
                );
            }
            let readback = self
                .gpu
                .as_mut()
                .expect("GPU existence checked above")
                .begin_async_readback(target, order % 2, order)?;
            pending.push_back(readback);
            peak_in_flight = peak_in_flight.max(pending.len() as u64);
        }
        while let Some(readback) = pending.pop_front() {
            completed.push(
                self.gpu
                    .as_mut()
                    .expect("GPU existence checked above")
                    .finish_async_readback(readback)?,
            );
        }
        completed.sort_by_key(|(order, _)| *order);
        let mut frames = Vec::with_capacity(completed.len());
        for (_order, raw) in completed {
            let rgba8 = if target == self.target {
                raw
            } else {
                let mut resolved = Vec::new();
                cpu_resolve::resolve_rgba8_reconstruction_filter(
                    target,
                    self.supersample_factor,
                    &raw,
                    self.target,
                    &mut resolved,
                    self.reconstruction_filter,
                );
                resolved
            };
            frames.push(PixelReadback::from_rgba8(
                self.target.width,
                self.target.height,
                rgba8,
            ));
        }
        if let Some(last) = frames.last() {
            self.frame.clear();
            self.frame.extend_from_slice(last.rgba8());
            self.last_readback_frame = self
                .last_rendered_frame
                .map(RenderedFrameState::with_readback_completed_now);
        }
        let count = cameras.len() as u64;
        let raw_bytes = target.byte_len() as u64;
        self.last_render_work_metrics = RenderWorkMetrics {
            readback_copies: count,
            readback_bytes_copied: count.saturating_mul(raw_bytes),
            map_requests: count,
            blocking_polls: count,
            blocking_waits: count,
            cpu_frame_copy_bytes: count.saturating_mul(raw_bytes),
            async_readback_submissions: count,
            peak_readbacks_in_flight: peak_in_flight,
            ..RenderWorkMetrics::default()
        };
        Ok(frames)
    }

    fn draw_gpu(
        &mut self,
        target: RasterTarget,
        camera_projection: &camera::CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
        readback_mode: RenderReadbackMode,
        auto_exposure_meter: bool,
    ) -> Result<gpu::GpuRenderResult, RenderError> {
        let post_settings = gpu::GpuPostSettings::new(
            self.anti_aliasing,
            self.bloom,
            self.screen_space_ambient_occlusion,
            self.screen_space_reflections,
            depth_of_field_post_config(self.depth_of_field, camera_projection),
            self.auto_exposure.is_some(),
            self.scene_linear_capture_enabled,
        );
        #[cfg(not(target_arch = "wasm32"))]
        {
            let resolved_readback_mode = match readback_mode {
                RenderReadbackMode::Automatic => {
                    if self.gpu.as_ref().is_some_and(|gpu| !gpu.has_surface()) {
                        RenderReadbackMode::Synchronous
                    } else {
                        RenderReadbackMode::PresentOnly
                    }
                }
                explicit => explicit,
            };
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            let resolve_to_logical_target = target != self.target;
            let frame = if resolve_to_logical_target {
                self.gpu_supersample_frame.clear();
                &mut self.gpu_supersample_frame
            } else {
                &mut self.frame
            };
            let result = gpu.render_to_frame(
                target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.output.white_balance_uniform(),
                self.background_color,
                camera_projection,
                clipping_planes,
                section_box,
                frame,
                post_settings,
                resolved_readback_mode == RenderReadbackMode::Synchronous,
                auto_exposure_meter,
            )?;
            if resolve_to_logical_target
                && resolved_readback_mode == RenderReadbackMode::Synchronous
            {
                cpu_resolve::resolve_rgba8_reconstruction_filter(
                    target,
                    self.supersample_factor,
                    self.gpu_supersample_frame.as_slice(),
                    self.target,
                    &mut self.frame,
                    self.reconstruction_filter,
                );
            }
            if result.submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            // self.stats.gpu_culling_dispatches stays at 0 — the empty culling
            // kernel was deleted in commit a311fcd. The public counter is kept
            // for API stability and will be repurposed when a real culling
            // kernel lands in a future v1.x.
            Ok(result)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = target;
            let _ = readback_mode;
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            let result = gpu.render_to_surface(
                self.target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.output.white_balance_uniform(),
                self.background_color,
                camera_projection,
                clipping_planes,
                section_box,
                post_settings,
                auto_exposure_meter,
            )?;
            if result.submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            Ok(result)
        }
    }

    fn gpu_render_target(&self) -> Result<RasterTarget, RenderError> {
        let scale = if self.target.backend == crate::diagnostics::Backend::HeadlessGpu {
            self.supersample_factor
        } else {
            1
        };
        let target = self::target::validate_supersample_target(self.target, scale)?;
        let workaround_required = self
            .gpu
            .as_ref()
            .is_some_and(gpu::GpuDeviceState::requires_v3d_headless_target_alignment);
        Ok(self::target::v3d_headless_render_target(
            target,
            workaround_required,
        ))
    }
}

pub(super) fn depth_of_field_post_config(
    config: Option<DepthOfFieldConfig>,
    camera_projection: &camera::CameraProjection,
) -> Option<output::DepthOfFieldPostConfig> {
    let config = config?;
    let focus_depth =
        camera_projection.depth_buffer_for_camera_distance(config.focus_distance())?;
    Some(output::DepthOfFieldPostConfig::new(
        focus_depth,
        config,
        camera_projection.near_far(),
        camera_projection.uses_reversed_z(),
    ))
}

fn prepared_triangle_alias_count(prepared: &PreparedSceneState) -> u64 {
    let primitive_triangles = prepared.primitives.len() as u64;
    let instance_triangles = prepared
        .instances
        .iter()
        .map(|set| (set.primitives().len() as u64).saturating_mul(set.instances().len() as u64))
        .sum::<u64>();
    primitive_triangles.saturating_add(instance_triangles)
}

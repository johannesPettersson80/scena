use super::*;

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
                let (clipping_planes, section_box) = {
                    let prepared = self.prepared_state(scene)?;
                    (prepared.clipping_planes.clone(), prepared.section_box)
                };
                let gpu_result = self.draw_gpu(
                    gpu_target.expect("GPU render target exists when GPU is active"),
                    &camera_projection,
                    &clipping_planes,
                    section_box,
                    readback_mode,
                )?;
                self.last_render_work_metrics.add_gpu_result(gpu_result);
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
        };
        self.last_rendered_frame = Some(rendered_frame);
        self.last_readback_frame = (self.gpu.is_none()
            || self.last_render_work_metrics.readback_copies > 0)
            .then_some(rendered_frame);

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
                cpu_resolve::downsample_rgba8_reconstruction_filter(
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
            self.last_readback_frame = self.last_rendered_frame;
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

    pub fn gpu_adapter_report(&self) -> Option<GpuAdapterReport> {
        self.gpu.as_ref().map(GpuDeviceState::adapter_report)
    }

    pub const fn last_render_work_metrics(&self) -> RenderWorkMetrics {
        self.last_render_work_metrics
    }

    pub fn render_active(&mut self, scene: &Scene) -> Result<RenderOutcome, RenderError> {
        self.prepared_state(scene)?;
        let camera = scene.active_camera().ok_or(RenderError::NoActiveCamera)?;
        self.render(scene, camera)
    }

    pub fn frame_rgba8(&self) -> &[u8] {
        &self.frame
    }

    pub fn poll_device(&mut self) -> DevicePoll {
        let before = self.stats.pending_destructions;
        let (destroyed_resources, status) = self
            .gpu
            .as_mut()
            .map(|gpu| gpu.poll_device())
            .unwrap_or((0, DevicePollStatus::Unsupported));
        let after = self
            .gpu
            .as_ref()
            .map(|gpu| gpu.pending_destructions())
            .unwrap_or(0);
        self.stats.pending_destructions = after;
        DevicePoll {
            pending_destructions_before: before,
            pending_destructions_after: after,
            destroyed_resources,
            status,
            gpu_polled: status == DevicePollStatus::Confirmed,
        }
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub(crate) fn rendered_frame_state(&self) -> Option<RenderedFrameState> {
        self.last_rendered_frame
    }

    pub(crate) fn readback_frame_state(&self) -> Option<RenderedFrameState> {
        self.last_readback_frame
    }

    pub(crate) fn clear_rendered_frame(&mut self) {
        self.last_rendered_generation = None;
        self.last_rendered_frame = None;
        self.last_readback_frame = None;
    }

    pub fn has_gpu_device(&self) -> bool {
        self.gpu.is_some()
    }

    fn draw_gpu(
        &mut self,
        target: RasterTarget,
        camera_projection: &camera::CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
        readback_mode: RenderReadbackMode,
    ) -> Result<gpu::GpuRenderResult, RenderError> {
        let post_settings = gpu::GpuPostSettings::new(
            self.anti_aliasing,
            self.bloom,
            self.screen_space_ambient_occlusion,
            self.screen_space_reflections,
            depth_of_field_post_config(self.depth_of_field, camera_projection),
        );
        #[cfg(not(target_arch = "wasm32"))]
        {
            let resolved_readback_mode = match readback_mode {
                RenderReadbackMode::Automatic => {
                    if self.auto_exposure.is_some()
                        || self.gpu.as_ref().is_some_and(|gpu| !gpu.has_surface())
                    {
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
            let scale = if target == self.target {
                1
            } else {
                self.supersample_factor
            };
            let frame = if scale > 1 {
                self.gpu_supersample_frame.clear();
                &mut self.gpu_supersample_frame
            } else {
                &mut self.frame
            };
            let result = gpu.render_to_frame(
                target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.background_color,
                camera_projection,
                clipping_planes,
                section_box,
                frame,
                post_settings,
                resolved_readback_mode == RenderReadbackMode::Synchronous,
            )?;
            if scale > 1 && resolved_readback_mode == RenderReadbackMode::Synchronous {
                cpu_resolve::downsample_rgba8_reconstruction_filter(
                    target,
                    scale,
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
                self.background_color,
                camera_projection,
                clipping_planes,
                section_box,
                post_settings,
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
        self::target::validate_supersample_target(self.target, scale)
    }

    pub(in crate::render) fn prepared_state(
        &self,
        scene: &Scene,
    ) -> Result<&PreparedSceneState, RenderError> {
        let prepared = self.prepared.as_ref().ok_or(RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared,
        })?;

        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::DifferentScene,
            });
        }

        let current_revision = scene.structure_revision();
        if prepared.structure_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.structure_revision,
                    current_revision,
                    change: ChangeKind::SceneStructure,
                },
            });
        }

        let current_revision = scene.transform_revision();
        if prepared.transform_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.transform_revision,
                    current_revision,
                    change: ChangeKind::Transform,
                },
            });
        }

        let current_revision = scene.appearance_revision();
        if prepared.appearance_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.appearance_revision,
                    current_revision,
                    change: ChangeKind::Appearance,
                },
            });
        }

        let current_revision = scene.visibility_revision();
        if prepared.visibility_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.visibility_revision,
                    current_revision,
                    change: ChangeKind::Visibility,
                },
            });
        }

        if prepared.environment_revision != self.environment_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::EnvironmentChanged {
                    prepared_revision: prepared.environment_revision,
                    current_revision: self.environment_revision,
                    change: ChangeKind::Environment,
                },
            });
        }

        if prepared.target_revision != self.target_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::TargetChanged {
                    prepared_revision: prepared.target_revision,
                    current_revision: self.target_revision,
                    change: ChangeKind::RenderTarget,
                },
            });
        }

        if prepared.output_resources_revision != self.output_resources_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::OutputSettingsChanged {
                    prepared_revision: prepared.output_resources_revision,
                    current_revision: self.output_resources_revision,
                    change: ChangeKind::OutputSettings,
                },
            });
        }

        Ok(prepared)
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
        config.aperture_f_stop(),
        config.radius_px(),
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

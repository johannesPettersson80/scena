use super::*;

#[derive(Clone)]
pub(super) struct Surface {
    material: MaterialDesc,
    tint: Color,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Guide {
    pub(super) target: Option<NodeKey>,
    pub(super) depth: f32,
    pub(super) normal: Vec3,
    pub(super) hit: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Ray {
    pub(super) origin: Vec3,
    pub(super) direction: Vec3,
}
#[derive(Clone, Copy)]
pub(super) struct SampledLight {
    direction: Vec3,
    distance: f32,
    radiance: Vec3,
    pdf: f32,
}

#[derive(Default)]
pub(super) struct TraceCounters {
    primary_rays: u64,
    secondary_rays: u64,
    shadow_rays: u64,
    intersections: u64,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn render_photographic_final(
        &self,
        raster: &CaptureRgba8,
        subject: Option<u64>,
        quality: PhotographicTransportQuality,
    ) -> Result<(CaptureRgba8, PhotographicTransportReportV1), SceneHostError> {
        let width = raster.descriptor.width;
        let height = raster.descriptor.height;
        let camera = self
            .scene
            .active_camera()
            .ok_or(crate::LookupError::NoActiveCamera)?;
        let mut surfaces = BTreeMap::new();
        let inspection = self.scene.inspect_with_assets(&self.assets);
        for node in inspection.nodes() {
            let Some(material) = node
                .mesh_material()
                .and_then(|handle| self.assets.material(handle))
            else {
                continue;
            };
            surfaces.insert(
                node.node(),
                Surface {
                    material,
                    tint: node.tint().unwrap_or(Color::WHITE),
                },
            );
        }
        let emissive = emissive_geometry_samples(&inspection, &surfaces);
        let light_kinds = self
            .scene
            .light_nodes()
            .map(|(_, _, light, _)| light_kind(light).to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut counters = TraceCounters::default();
        let subject_nodes = subject
            .map(|handle| self.resolve_node(handle))
            .transpose()?
            .map(|root| self.scene.subtree_nodes(root))
            .transpose()?;
        let mut linear = vec![Vec3::ZERO; (width * height) as usize];
        let mut guides = vec![Guide::default(); linear.len()];
        let background = raster_background_linear(raster);
        let spp = quality.samples();
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) as usize;
                let mut accumulated = Vec3::ZERO;
                for sample in 0..spp {
                    let jitter = sample_jitter(x, y, sample);
                    let Some(ray) = camera_ray(
                        &self.scene,
                        camera,
                        CameraRaySample {
                            viewport: [width, height],
                            pixel: [x, y],
                            jitter,
                            depth_of_field: self.renderer.depth_of_field(),
                            sample,
                        },
                    ) else {
                        accumulated += background;
                        continue;
                    };
                    counters.primary_rays = counters.primary_rays.saturating_add(1);
                    let (radiance, guide) = trace_path(
                        &self.scene,
                        &self.assets,
                        &surfaces,
                        &emissive,
                        ray,
                        background,
                        quality.bounces(),
                        seed(x, y, sample),
                        &mut counters,
                    )?;
                    accumulated += radiance;
                    if sample == 0 {
                        guides[index] = guide;
                    }
                }
                linear[index] = accumulated / spp as f32;
            }
        }
        suppress_isolated_fireflies(&mut linear, &guides, width, height);
        let linear = edge_aware_denoise(&linear, &guides, width, height);
        let exposure = 2.0_f32.powf(self.renderer.exposure_ev());
        let exposure_match = match_final_subject_exposure(
            &linear,
            &guides,
            raster,
            exposure,
            subject_nodes.as_deref(),
        );
        let rgba8 = linear
            .into_iter()
            .flat_map(|color| display_rgba8(color * exposure * exposure_match.scale))
            .collect();
        let mesh_acceleration_structures = inspection
            .draw_list()
            .iter()
            .map(|draw| draw.geometry())
            .collect::<BTreeSet<_>>()
            .len();
        let instance_count = inspection
            .draw_list()
            .iter()
            .filter(|draw| draw.instance().is_some())
            .count();
        Ok((
            CaptureRgba8 {
                descriptor: raster.descriptor.clone(),
                rgba8,
            },
            PhotographicTransportReportV1 {
                schema: PHOTOGRAPHIC_TRANSPORT_REPORT_SCHEMA_V1.to_owned(),
                path: "cpu_progressive_path_tracer".to_owned(),
                samples_per_pixel: spp,
                maximum_bounces: quality.bounces(),
                primary_rays: counters.primary_rays,
                secondary_rays: counters.secondary_rays,
                shadow_rays: counters.shadow_rays,
                intersections: counters.intersections,
                mesh_acceleration_structures,
                instance_count,
                light_kinds,
                emissive_geometry_lights: emissive.len(),
                multiple_importance_sampling: true,
                edge_aware_denoising: true,
                firefly_suppression: "isolated_outliers_only".to_owned(),
                raster_preview_preserved: true,
                final_exposure_scale: exposure_match.scale,
                exposure_target_luminance_srgb8: exposure_match.target,
                exposure_measured_luminance_srgb8: exposure_match.measured,
                exposure_sample_count: exposure_match.sample_count,
            },
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn trace_path<F: AssetFetcher>(
    scene: &crate::Scene,
    assets: &crate::Assets<F>,
    surfaces: &BTreeMap<NodeKey, Surface>,
    emissive: &[(Vec3, Vec3)],
    mut ray: Ray,
    background: Vec3,
    max_bounces: u32,
    mut rng: u64,
    counters: &mut TraceCounters,
) -> Result<(Vec3, Guide), SceneHostError> {
    let mut throughput = Vec3::ONE;
    let mut radiance = Vec3::ZERO;
    let mut first_guide = Guide::default();
    for bounce in 0..max_bounces {
        let hit = scene.raycast_with_assets(ray.origin, ray.direction, assets)?;
        let Some(hit) = hit else {
            radiance += throughput * background;
            break;
        };
        counters.intersections = counters.intersections.saturating_add(1);
        let node = hit_node(hit);
        let normal = oriented_normal(hit, ray.direction);
        if bounce == 0 {
            first_guide = Guide {
                target: Some(node),
                depth: hit.distance,
                normal,
                hit: true,
            };
        }
        let Some(surface) = surfaces.get(&node) else {
            radiance += throughput * Vec3::splat(0.18);
            break;
        };
        let material = &surface.material;
        let base = color_vec(material.base_color()) * color_vec(surface.tint);
        let alpha = (material.base_color().a * surface.tint.a).clamp(0.0, 1.0);
        match material.alpha_mode() {
            AlphaMode::Mask { cutoff } if alpha < cutoff => {
                ray.origin = hit.world_position + ray.direction * scene_epsilon(hit.distance);
                continue;
            }
            AlphaMode::Blend if random01(&mut rng) > alpha => {
                throughput *= Vec3::splat(1.0 - alpha * 0.15);
                ray.origin = hit.world_position + ray.direction * scene_epsilon(hit.distance);
                continue;
            }
            AlphaMode::Opaque | AlphaMode::Mask { .. } | AlphaMode::Blend => {}
        }
        let emission = color_vec(material.emissive()) * material.emissive_strength();
        radiance += throughput * emission;

        let direct = estimate_direct(
            scene,
            assets,
            hit.world_position,
            normal,
            -ray.direction,
            material,
            emissive,
            &mut rng,
            counters,
        )?;
        radiance += throughput * direct;

        let metallic = material.metallic_factor();
        let roughness = material.roughness_factor().clamp(0.02, 1.0);
        let transmission = material.transmission_factor().clamp(0.0, 1.0);
        let fresnel = schlick((-ray.direction).dot(normal).max(0.0), material.ior());
        let choice = random01(&mut rng);
        let next_direction;
        if choice < transmission {
            next_direction = refract_or_reflect(
                ray.direction,
                normal,
                material.ior(),
                random_unit_vector(&mut rng) * roughness * 0.08,
            );
            let attenuation = color_vec(material.attenuation_color());
            let distance = material.attenuation_distance();
            if distance.is_finite() && distance > 0.0 {
                throughput *= attenuation.powf(hit.distance / distance);
            }
            throughput *= base.lerp(Vec3::ONE, transmission);
        } else if choice < transmission + metallic.max(fresnel) * (1.0 - transmission) {
            let reflected = reflect(ray.direction, normal);
            next_direction = (reflected
                + random_in_hemisphere(normal, &mut rng) * roughness.powi(2))
            .normalize_or_zero();
            throughput *= base.lerp(Vec3::ONE, fresnel) * (1.0 - roughness * 0.12);
        } else {
            next_direction = cosine_hemisphere(normal, &mut rng);
            throughput *= base;
        }
        if next_direction.length_squared() <= 0.5 {
            break;
        }
        if bounce >= 2 {
            let survive = throughput.max_element().clamp(0.08, 0.95);
            if random01(&mut rng) > survive {
                break;
            }
            throughput /= survive;
        }
        ray = Ray {
            origin: hit.world_position + next_direction * scene_epsilon(hit.distance),
            direction: next_direction,
        };
        counters.secondary_rays = counters.secondary_rays.saturating_add(1);
    }
    Ok((radiance, first_guide))
}

#[allow(clippy::too_many_arguments)]
fn estimate_direct<F: AssetFetcher>(
    scene: &crate::Scene,
    assets: &crate::Assets<F>,
    position: Vec3,
    normal: Vec3,
    view: Vec3,
    material: &MaterialDesc,
    emissive: &[(Vec3, Vec3)],
    rng: &mut u64,
    counters: &mut TraceCounters,
) -> Result<Vec3, SceneHostError> {
    let mut candidates = Vec::new();
    for (_, _, light, transform) in scene.light_nodes() {
        if let Some(sample) = sample_light(light, transform, position, rng) {
            candidates.push(sample);
        }
    }
    if !emissive.is_empty() {
        let index = (random01(rng) * emissive.len() as f32) as usize % emissive.len();
        let (sample_position, emission) = emissive[index];
        let offset = sample_position - position;
        let distance = offset.length();
        if distance > 1.0e-5 {
            candidates.push(SampledLight {
                direction: offset / distance,
                distance,
                radiance: emission / distance.mul_add(distance, 1.0),
                pdf: 1.0 / emissive.len() as f32,
            });
        }
    }
    if candidates.is_empty() {
        return Ok(Vec3::splat(0.08));
    }
    let index = (random01(rng) * candidates.len() as f32) as usize % candidates.len();
    let sample = candidates[index];
    let n_dot_l = normal.dot(sample.direction).max(0.0);
    if n_dot_l <= 0.0 {
        return Ok(Vec3::ZERO);
    }
    counters.shadow_rays = counters.shadow_rays.saturating_add(1);
    let origin = position + normal * scene_epsilon(sample.distance);
    let blocked = scene
        .raycast_with_assets(origin, sample.direction, assets)?
        .is_some_and(|hit| hit.distance < sample.distance - scene_epsilon(sample.distance));
    if blocked {
        return Ok(Vec3::ZERO);
    }
    let roughness = material.roughness_factor().clamp(0.02, 1.0);
    let brdf_pdf = n_dot_l / std::f32::consts::PI;
    let light_pdf = sample.pdf.max(1.0e-5);
    let mis = light_pdf.powi(2) / (light_pdf.powi(2) + brdf_pdf.powi(2)).max(1.0e-6);
    let base = color_vec(material.base_color());
    let metallic = material.metallic_factor().clamp(0.0, 1.0);
    let half = (view + sample.direction).normalize_or_zero();
    let n_dot_v = normal.dot(view).max(1.0e-4);
    let n_dot_h = normal.dot(half).max(0.0);
    let v_dot_h = view.dot(half).max(0.0);
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let denominator = n_dot_h.mul_add(n_dot_h * (alpha2 - 1.0), 1.0).max(1.0e-4);
    let distribution = alpha2 / (std::f32::consts::PI * denominator * denominator);
    let geometry = smith_ggx(n_dot_v, alpha) * smith_ggx(n_dot_l, alpha);
    let dielectric_f0 = ((material.ior() - 1.0) / (material.ior() + 1.0)).powi(2);
    let f0 = Vec3::splat(dielectric_f0).lerp(base, metallic);
    let fresnel = f0 + (Vec3::ONE - f0) * (1.0 - v_dot_h).powi(5);
    let specular = fresnel * (distribution * geometry / (4.0 * n_dot_v * n_dot_l).max(1.0e-4));
    let diffuse = base * (Vec3::ONE - fresnel) * (1.0 - metallic) / std::f32::consts::PI;
    Ok(sample.radiance * (diffuse + specular) * n_dot_l * mis * candidates.len() as f32)
}

fn smith_ggx(n_dot_direction: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let denominator = n_dot_direction + (alpha2 + (1.0 - alpha2) * n_dot_direction.powi(2)).sqrt();
    2.0 * n_dot_direction / denominator.max(1.0e-4)
}

fn sample_light(
    light: Light,
    transform: Transform,
    position: Vec3,
    rng: &mut u64,
) -> Option<SampledLight> {
    match light {
        Light::Directional(light) => {
            let direction = -(transform.rotation * Vec3::NEG_Z).normalize_or_zero();
            Some(SampledLight {
                direction,
                distance: 1.0e6,
                radiance: color_vec(light.color()) * (light.illuminance_lux() / 10_000.0),
                pdf: 1.0,
            })
        }
        Light::Point(light) => point_sample(
            transform.translation,
            light.color(),
            light.intensity_candela(),
            position,
        ),
        Light::Spot(light) => {
            let sample = point_sample(
                transform.translation,
                light.color(),
                light.intensity_candela(),
                position,
            )?;
            let forward = transform.rotation * Vec3::NEG_Z;
            let cone = (-sample.direction)
                .dot(forward.normalize_or_zero())
                .max(0.0);
            let outer = light.outer_cone_angle().radians().cos();
            (cone > outer).then_some(SampledLight {
                radiance: sample.radiance * ((cone - outer) / (1.0 - outer).max(1.0e-5)),
                ..sample
            })
        }
        Light::Area(light) => {
            let local = match light.shape() {
                crate::AreaLightShape::Rect { width, height } => Vec3::new(
                    (random01(rng) - 0.5) * width,
                    (random01(rng) - 0.5) * height,
                    0.0,
                ),
                crate::AreaLightShape::Disc { radius } => {
                    let r = random01(rng).sqrt() * radius;
                    let theta = random01(rng) * std::f32::consts::TAU;
                    Vec3::new(theta.cos() * r, theta.sin() * r, 0.0)
                }
                crate::AreaLightShape::Sphere { radius } => random_unit_vector(rng) * radius,
            };
            let emitter = transform.translation + transform.rotation * local;
            point_sample(
                emitter,
                light.color(),
                light.luminous_flux_lumens() / (4.0 * std::f32::consts::PI),
                position,
            )
            .map(|sample| SampledLight {
                pdf: 0.25,
                ..sample
            })
        }
    }
}

fn point_sample(
    position: Vec3,
    color: Color,
    intensity: f32,
    surface: Vec3,
) -> Option<SampledLight> {
    let offset = position - surface;
    let distance = offset.length();
    (distance > 1.0e-5).then_some(SampledLight {
        direction: offset / distance,
        distance,
        radiance: color_vec(color) * (intensity / distance.mul_add(distance, 1.0)),
        pdf: 1.0,
    })
}

pub(super) fn emissive_geometry_samples(
    inspection: &crate::SceneInspectionReport,
    surfaces: &BTreeMap<NodeKey, Surface>,
) -> Vec<(Vec3, Vec3)> {
    inspection
        .draw_list()
        .iter()
        .filter_map(|draw| {
            let surface = surfaces.get(&draw.node())?;
            let emission =
                color_vec(surface.material.emissive()) * surface.material.emissive_strength();
            (emission.max_element() > 1.0e-5).then(|| {
                (
                    transform_point(draw.local_bounds().center(), draw.world_transform()),
                    emission,
                )
            })
        })
        .collect()
}

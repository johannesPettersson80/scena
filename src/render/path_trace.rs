//! Offline path tracing: a still image computed by following light through
//! the scene instead of rasterizing it.
//!
//! The rasterizer approximates soft shadows, reflections, refraction and
//! bounce light with separate techniques, each tuned to be fast. The path
//! tracer follows many random light paths through every pixel and averages
//! them, so those effects come out of one calculation: area lights cast
//! shadows that soften with distance, surfaces light each other, mirrors
//! and glass show the rest of the scene. The price is time, seconds to
//! minutes per image, so this is a mode for stills rather than for
//! interaction.
//!
//! It reads the same `Scene`, `Assets`, lights, environment, exposure and
//! tone mapping as the rasterizer and needs no GPU.

mod bsdf;
mod bvh;
mod camera;
mod denoise;
mod detail;
mod emitters;
mod environment;
mod error;
mod film;
mod finish;
mod image;
mod integrator;
mod light;
mod material;
mod noise;
mod options;
mod rounding;
mod sampling;
mod scene;
mod shading;
#[cfg(test)]
mod tests;

pub use self::error::PathTraceError;
pub use self::image::PathTracedImage;
pub use self::options::{PathTraceOptions, PathTraceStats};

use crate::assets::Assets;
use crate::scene::{CameraKey, NodeKey, Scene};

use self::camera::TraceCamera;
use self::detail::DetailSettings;
use self::emitters::Emitters;
use self::environment::TraceEnvironment;
use self::integrator::Tracer;
use self::light::LightSampler;
use self::scene::TraceScene;
use super::Renderer;

impl Renderer {
    /// Renders a still image of `scene` by path tracing.
    ///
    /// The image has the renderer's target size and uses its environment,
    /// background, exposure, white balance, tone mapping and depth of field.
    /// Nothing needs to be prepared first and the renderer is not changed:
    /// the scene is read as it stands.
    ///
    /// Meshes and instance sets with unlit or metallic-roughness materials
    /// are traced, with their textures, normal maps, cut-outs, clearcoat,
    /// sheen and glass, and with the [`crate::SurfaceFinish`] of materials
    /// that have one. Anisotropy and iridescence are not traced and
    /// render as the plain material. Lines, wireframes, labels and particles
    /// are overlays rather than lit surfaces and are left out; the returned
    /// [`PathTraceStats`] say how many.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Assets, GeometryDesc, MaterialDesc, PathTraceOptions, Renderer, Scene};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let assets = Assets::new();
    /// let mut scene = Scene::new();
    /// let geometry = assets.create_geometry(GeometryDesc::sphere(0.5, 32, 16));
    /// let material = assets.create_material(MaterialDesc::default());
    /// scene.mesh(geometry, material).add()?;
    /// scene.add_studio_lighting()?;
    /// let camera = scene.add_default_camera()?;
    ///
    /// let renderer = Renderer::headless(64, 48)?;
    /// let image = renderer.path_trace(&scene, &assets, camera, &PathTraceOptions::preview())?;
    /// assert_eq!(image.rgba8().len(), 64 * 48 * 4);
    /// # Ok(())
    /// # }
    /// ```
    pub fn path_trace<F>(
        &self,
        scene: &Scene,
        assets: &Assets<F>,
        camera: CameraKey,
        options: &PathTraceOptions,
    ) -> Result<PathTracedImage, PathTraceError> {
        self.path_trace_with_progress(scene, assets, camera, options, &|_, _| {})
    }

    /// [`Self::path_trace`], reporting progress.
    ///
    /// `progress` is called after each finished image row with the number of
    /// rows done and the total. It may be called from several threads.
    pub fn path_trace_with_progress<F>(
        &self,
        scene: &Scene,
        assets: &Assets<F>,
        camera: CameraKey,
        options: &PathTraceOptions,
        progress: &(dyn Fn(u32, u32) + Sync),
    ) -> Result<PathTracedImage, PathTraceError> {
        self.path_trace_without(scene, assets, camera, options, &[], progress)
    }

    /// Re-develops a traced image with the renderer's current exposure,
    /// white balance, tone mapping and background, without tracing again.
    ///
    /// The linear radiance is kept; only the 8-bit image changes. This makes
    /// exposure a decision that can follow the trace: meter the image,
    /// change `set_exposure_ev`, and develop again in milliseconds.
    pub fn develop_path_traced(&self, image: &mut PathTracedImage) {
        image.rgba8 = film::encode(
            &image.radiance,
            image.width as usize,
            self.output,
            self.background_color,
        );
    }

    /// [`Self::path_trace_with_progress`], leaving out the nodes in `without`.
    ///
    /// Use it for stand-ins that only a rasterized frame needs, such as a
    /// blended decal that fakes a contact shadow the tracer casts for real.
    /// Only the listed nodes are left out, not their descendants.
    pub fn path_trace_without<F>(
        &self,
        scene: &Scene,
        assets: &Assets<F>,
        camera: CameraKey,
        options: &PathTraceOptions,
        without: &[NodeKey],
        progress: &(dyn Fn(u32, u32) + Sync),
    ) -> Result<PathTracedImage, PathTraceError> {
        let (width, height) = (self.target.width, self.target.height);
        if width == 0 || height == 0 {
            return Err(PathTraceError::InvalidTargetSize { width, height });
        }
        let (camera, origin) =
            TraceCamera::from_scene(scene, camera, (width, height), self.depth_of_field)?;
        let detail = DetailSettings {
            enabled: options.surface_detail(),
            edge_radius: options.edge_radius_m(),
        };
        let traced = TraceScene::build(scene, assets, origin, without, detail)?;
        let lights = light::collect(scene, origin, options.directional_light_angular_radius());
        let emitters = Emitters::build(&traced);
        let environment = match self.environment {
            Some(environment) => {
                let desc = assets
                    .environment_snapshot(environment)
                    .ok_or(PathTraceError::EnvironmentNotFound { environment })?;
                TraceEnvironment::from_desc(
                    &desc,
                    options.environment_resolution(),
                    self.environment_intensity,
                    self.environment_rotation_y_radians,
                )
            }
            None => None,
        };
        let tracer = Tracer {
            scene: &traced,
            lights: LightSampler::new(&lights),
            emitters: &emitters,
            environment: environment.as_ref(),
            max_bounces: options.max_bounces(),
            footprint: camera.pixel_footprint(height),
        };
        // The clamp is set against the exposed image, so the same setting
        // suits a dim scene and a bright one.
        let clamp = options.radiance_clamp() * (-self.output.exposure_ev()).exp2();
        let film = film::expose(&tracer, &camera, (width, height), options, clamp, progress);
        let (radiance, rgba8) = film.develop(options.denoise(), self.output, self.background_color);
        let count = |wanted: &dyn Fn(&scene::TraceObject) -> bool| {
            traced
                .objects
                .iter()
                .filter(|object| wanted(object))
                .count() as u32
        };
        Ok(PathTracedImage {
            width,
            height,
            rgba8,
            radiance,
            stats: PathTraceStats {
                triangles: traced.positions.len() as u64,
                emissive_triangles: emitters.len() as u64,
                lights: lights.len() as u32,
                materials: traced.materials.len() as u32,
                textures: traced.textures.len() as u32,
                environment_resolution: environment.as_ref().map(TraceEnvironment::resolution),
                samples_per_pixel: options.samples_per_pixel(),
                max_bounces: options.max_bounces(),
                denoised: options.denoise(),
                camera_paths: u64::from(width)
                    * u64::from(height)
                    * u64::from(options.samples_per_pixel()),
                skipped_stroke_nodes: traced.skipped.stroke_nodes,
                skipped_label_nodes: traced.skipped.label_nodes,
                skipped_particle_nodes: traced.skipped.particle_nodes,
                skipped_degenerate_triangles: traced.skipped.degenerate_triangles,
                finished_surfaces: count(&|object| object.finish.is_some()),
                rounded_surfaces: count(&|object| object.edge_radius > 0.0),
            },
        })
    }
}

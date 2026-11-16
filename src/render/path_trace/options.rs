//! Settings and statistics of a path-traced render.

/// Settings for [`Renderer::path_trace`](crate::Renderer::path_trace).
///
/// # Examples
///
/// ```
/// use scena::PathTraceOptions;
///
/// let options = PathTraceOptions::new().with_samples_per_pixel(128).with_max_bounces(10);
/// assert_eq!(options.samples_per_pixel(), 128);
/// assert!(options.denoise());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathTraceOptions {
    samples_per_pixel: u32,
    max_bounces: u32,
    denoise: bool,
    seed: u64,
    radiance_clamp: f32,
    directional_light_angular_radius: f32,
    environment_resolution: u32,
}

impl Default for PathTraceOptions {
    fn default() -> Self {
        Self {
            samples_per_pixel: 64,
            max_bounces: 8,
            denoise: true,
            seed: 0,
            radiance_clamp: 32.0,
            directional_light_angular_radius: 0.5_f32.to_radians(),
            environment_resolution: 1024,
        }
    }
}

impl PathTraceOptions {
    /// Largest accepted number of camera paths per pixel.
    pub const MAX_SAMPLES_PER_PIXEL: u32 = 65_536;
    /// Largest accepted number of bounces per path.
    pub const MAX_BOUNCES: u32 = 64;
    /// Largest accepted environment cube edge, in texels.
    pub const MAX_ENVIRONMENT_RESOLUTION: u32 = 2048;

    /// Balanced defaults: 64 paths per pixel, 8 bounces, denoised.
    pub fn new() -> Self {
        Self::default()
    }

    /// A quick look at the lighting: 16 paths per pixel and 4 bounces.
    pub fn preview() -> Self {
        Self::default()
            .with_samples_per_pixel(16)
            .with_max_bounces(4)
    }

    /// A finished still: 256 paths per pixel and 12 bounces.
    pub fn production() -> Self {
        Self::default()
            .with_samples_per_pixel(256)
            .with_max_bounces(12)
    }

    /// Camera paths traced through every pixel. Noise falls with the square
    /// root of this number; render time grows in proportion to it.
    pub const fn samples_per_pixel(self) -> u32 {
        self.samples_per_pixel
    }

    /// Times a path may change direction before it ends.
    pub const fn max_bounces(self) -> u32 {
        self.max_bounces
    }

    /// Whether the finished image is filtered to remove residual noise.
    pub const fn denoise(self) -> bool {
        self.denoise
    }

    /// Seed of the random sequence. The same seed, scene and settings give
    /// the same image whatever the number of threads.
    pub const fn seed(self) -> u64 {
        self.seed
    }

    /// Brightest scattered light one path may contribute, relative to the
    /// renderer's exposure. Infinite when clamping is off.
    pub const fn radiance_clamp(self) -> f32 {
        self.radiance_clamp
    }

    /// Angular radius of directional lights in radians. Zero gives a
    /// perfectly parallel beam and hard-edged shadows.
    pub const fn directional_light_angular_radius(self) -> f32 {
        self.directional_light_angular_radius
    }

    /// Texels along each edge of the cube an HDR environment is sampled into.
    pub const fn environment_resolution(self) -> u32 {
        self.environment_resolution
    }

    /// Sets the paths per pixel, clamped to `1..=MAX_SAMPLES_PER_PIXEL`.
    pub const fn with_samples_per_pixel(mut self, samples: u32) -> Self {
        self.samples_per_pixel = if samples < 1 {
            1
        } else if samples > Self::MAX_SAMPLES_PER_PIXEL {
            Self::MAX_SAMPLES_PER_PIXEL
        } else {
            samples
        };
        self
    }

    /// Sets the bounce limit, clamped to `MAX_BOUNCES`. Zero shows only what
    /// the camera sees directly: emitted light and nothing lit.
    pub const fn with_max_bounces(mut self, bounces: u32) -> Self {
        self.max_bounces = if bounces > Self::MAX_BOUNCES {
            Self::MAX_BOUNCES
        } else {
            bounces
        };
        self
    }

    /// Turns the noise filter on or off.
    pub const fn with_denoise(mut self, denoise: bool) -> Self {
        self.denoise = denoise;
        self
    }

    /// Sets the seed of the random sequence.
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Limits the scattered light one path may contribute, which trades a
    /// little brightness in rare, very bright light paths for the absence of
    /// isolated bright pixels. Values that are not positive turn clamping off.
    pub const fn with_radiance_clamp(mut self, clamp: f32) -> Self {
        self.radiance_clamp = if clamp > 0.0 { clamp } else { f32::INFINITY };
        self
    }

    /// Sets the angular radius of directional lights, in radians, clamped to
    /// `0..=0.5`. Larger discs give softer shadow edges.
    pub const fn with_directional_light_angular_radius(mut self, radians: f32) -> Self {
        self.directional_light_angular_radius = if radians > 0.0 {
            if radians > 0.5 { 0.5 } else { radians }
        } else {
            0.0
        };
        self
    }

    /// Sets the environment cube edge, clamped to
    /// `16..=MAX_ENVIRONMENT_RESOLUTION`.
    pub const fn with_environment_resolution(mut self, resolution: u32) -> Self {
        self.environment_resolution = if resolution < 16 {
            16
        } else if resolution > Self::MAX_ENVIRONMENT_RESOLUTION {
            Self::MAX_ENVIRONMENT_RESOLUTION
        } else {
            resolution
        };
        self
    }
}

/// What a path-traced render contained and how it was computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathTraceStats {
    /// Triangles traced, with instanced geometry counted once per instance.
    pub triangles: u64,
    /// Triangles that glow and were sampled as light sources.
    pub emissive_triangles: u64,
    /// Directional, point, spot and area lights.
    pub lights: u32,
    pub materials: u32,
    pub textures: u32,
    /// Edge of the environment cube in texels, when an environment lit the scene.
    pub environment_resolution: Option<u32>,
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub denoised: bool,
    /// Camera paths traced: pixels times samples per pixel.
    pub camera_paths: u64,
    /// Nodes left out because they draw lines, wireframes or edges.
    pub skipped_stroke_nodes: u32,
    /// Label nodes left out; labels are an overlay, not a lit surface.
    pub skipped_label_nodes: u32,
    /// Particle sets left out.
    pub skipped_particle_nodes: u32,
    /// Triangles left out because they have no area or are not finite.
    pub skipped_degenerate_triangles: u64,
}

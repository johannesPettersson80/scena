//! Surface finish: what a surface looks like between the shape of a part and
//! the roughness of its material.

use glam::Vec3;

use super::MaterialDesc;
use crate::assets::PhotographicSurfaceKind;

/// How a surface was made, at the scale a close photograph resolves.
///
/// A material's roughness says how sharp its reflections are on average. A
/// finish says what the surface does between that average and the shape of
/// the part: the grain of a casting, the orange peel of paint, the marks a
/// lathe leaves, the slightly rounded edge every real part has, paint worn
/// off its corners and dirt settled into its recesses. These are what make a
/// rendered part read as a manufactured object instead of a diagram.
///
/// A finish is generated in three dimensions from the surface position, so
/// it needs no texture coordinates and shows no seams or tiling. Sizes are
/// physical: a scene measured in metres gets grain of the size the real
/// finish has.
///
/// The path tracer resolves a finish ([`crate::Renderer::path_trace`]). The
/// rasterizer draws the material without it.
///
/// # Examples
///
/// ```
/// use scena::{Color, MaterialDesc, PhotographicSurfaceKind, SurfaceFinish};
///
/// let cast_iron = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(70, 72, 76), 1.0, 0.6)
///     .with_surface_finish(SurfaceFinish::new(PhotographicSurfaceKind::CastMetal).with_grime(0.4));
/// assert!(cast_iron.surface_finish().is_some());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceFinish {
    kind: PhotographicSurfaceKind,
    scale: f32,
    relief: f32,
    wear: f32,
    grime: f32,
    edge_radius_m: Option<f32>,
    axis: Vec3,
    seed: u32,
}

impl SurfaceFinish {
    /// Largest accepted [`Self::with_scale`].
    pub const MAX_SCALE: f32 = 50.0;
    /// Largest accepted [`Self::with_relief`].
    pub const MAX_RELIEF: f32 = 4.0;
    /// Largest accepted [`Self::with_edge_radius_m`], in metres.
    pub const MAX_EDGE_RADIUS_M: f32 = 1.0;

    /// The finish `kind` at its real-world size, clean and unworn.
    pub const fn new(kind: PhotographicSurfaceKind) -> Self {
        Self {
            kind,
            scale: 1.0,
            relief: 1.0,
            wear: 0.0,
            grime: 0.0,
            edge_radius_m: None,
            axis: Vec3::Y,
            seed: 0,
        }
    }

    pub const fn kind(self) -> PhotographicSurfaceKind {
        self.kind
    }

    /// Factor on every feature size; see [`Self::with_scale`].
    pub const fn scale(self) -> f32 {
        self.scale
    }

    /// Factor on the depth of the relief; see [`Self::with_relief`].
    pub const fn relief(self) -> f32 {
        self.relief
    }

    pub const fn wear(self) -> f32 {
        self.wear
    }

    pub const fn grime(self) -> f32 {
        self.grime
    }

    /// Edge rounding radius in metres, or `None` for the finish's own.
    pub const fn edge_radius_m(self) -> Option<f32> {
        self.edge_radius_m
    }

    /// Direction of the finish's marks in the mesh's own space.
    pub const fn axis(self) -> Vec3 {
        self.axis
    }

    pub const fn seed(self) -> u32 {
        self.seed
    }

    /// Multiplies the size of every feature of the finish.
    ///
    /// `1.0` is the real size for a scene measured in metres. A model built
    /// at another scale, or a subject shown much larger than life, wants the
    /// features scaled with it. Values outside `0.02..=MAX_SCALE` are
    /// clamped; a value that is not a number leaves the scale unchanged.
    pub fn with_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.scale = scale.clamp(0.02, Self::MAX_SCALE);
        }
        self
    }

    /// Multiplies the depth of the relief: `0.0` keeps the finish's gloss
    /// and tone variation but flattens its bumps, `2.0` doubles them.
    pub fn with_relief(mut self, relief: f32) -> Self {
        if relief.is_finite() {
            self.relief = relief.clamp(0.0, Self::MAX_RELIEF);
        }
        self
    }

    /// How far use has worn the finish off exposed edges, `0.0..=1.0`.
    ///
    /// Paint and coatings wear through to bare metal, bare metal is
    /// burnished smoother, plastic and rubber scuff paler.
    pub fn with_wear(mut self, wear: f32) -> Self {
        if wear.is_finite() {
            self.wear = wear.clamp(0.0, 1.0);
        }
        self
    }

    /// How much dirt has settled into recesses, `0.0..=1.0`.
    pub fn with_grime(mut self, grime: f32) -> Self {
        if grime.is_finite() {
            self.grime = grime.clamp(0.0, 1.0);
        }
        self
    }

    /// Radius, in metres, to which the part's edges are rounded.
    ///
    /// No real edge is perfectly sharp, and the thin highlight a rounded
    /// edge catches is much of what separates a photograph from a diagram.
    /// The rounding is a shading effect: the silhouette keeps its corners.
    /// `0.0` leaves the edges sharp. Without this call the finish uses the
    /// radius typical of how such a part is made, times [`Self::scale`].
    pub fn with_edge_radius_m(mut self, radius_m: f32) -> Self {
        if radius_m.is_finite() {
            self.edge_radius_m = Some(radius_m.clamp(0.0, Self::MAX_EDGE_RADIUS_M));
        }
        self
    }

    /// Direction of the finish's marks, in the mesh's own space.
    ///
    /// Brushed metal is brushed along this axis. Machined metal was turned
    /// about it, through the mesh's origin, which leaves rings on a face and
    /// hoops on a cylinder. Other finishes ignore it. A zero or non-finite
    /// axis leaves the current one, which starts as `+Y`.
    pub fn with_axis(mut self, axis: Vec3) -> Self {
        let length = axis.length();
        if length.is_finite() && length > 0.0 {
            self.axis = axis / length;
        }
        self
    }

    /// Picks another instance of the same random pattern.
    pub const fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }
}

impl MaterialDesc {
    /// The surface finish, if one was given.
    pub const fn surface_finish(&self) -> Option<SurfaceFinish> {
        self.surface_finish
    }

    /// Gives the material a [`SurfaceFinish`].
    pub fn with_surface_finish(mut self, finish: SurfaceFinish) -> Self {
        self.surface_finish = Some(finish);
        self
    }

    /// Removes the surface finish.
    pub fn without_surface_finish(mut self) -> Self {
        self.surface_finish = None;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_clamped_and_bad_numbers_are_ignored() {
        let finish = SurfaceFinish::new(PhotographicSurfaceKind::CastMetal)
            .with_scale(1.0e9)
            .with_relief(-3.0)
            .with_wear(7.0)
            .with_grime(f32::NAN)
            .with_edge_radius_m(-1.0)
            .with_axis(Vec3::new(0.0, 0.0, 4.0));
        assert_eq!(finish.scale(), SurfaceFinish::MAX_SCALE);
        assert_eq!(finish.relief(), 0.0);
        assert_eq!(finish.wear(), 1.0);
        assert_eq!(finish.grime(), 0.0);
        assert_eq!(finish.edge_radius_m(), Some(0.0));
        assert_eq!(finish.axis(), Vec3::Z);
        let unchanged = finish
            .with_scale(f32::INFINITY)
            .with_axis(Vec3::ZERO)
            .with_edge_radius_m(f32::NAN);
        assert_eq!(unchanged, finish);
    }

    #[test]
    fn a_material_carries_its_finish() {
        let finish = SurfaceFinish::new(PhotographicSurfaceKind::BrushedMetal).with_seed(3);
        let material = MaterialDesc::default().with_surface_finish(finish);
        assert_eq!(material.surface_finish(), Some(finish));
        assert_eq!(material.without_surface_finish().surface_finish(), None);
        assert_eq!(MaterialDesc::default().surface_finish(), None);
    }
}

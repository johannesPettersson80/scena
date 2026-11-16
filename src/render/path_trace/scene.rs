//! The traced scene: triangles in camera-relative world space with their
//! shading attributes, built once from a `Scene` and the `Assets` it uses.

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::assets::{Assets, MaterialHandle};
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex, Primitive};
use crate::material::{Color, MaterialKind};
use crate::scene::{NodeKey, Scene, WorldTransform};

use super::super::prepare::tangents::generate_model_tangents;
use super::PathTraceError;
use super::bvh::Bvh;
use super::detail::DetailSettings;
use super::finish::TraceFinish;
use super::material::{Coverage, TextureStore, TraceMaterial, TraceTexture};

/// Marks a triangle without tangents or vertex colours.
pub(super) const NONE: u32 = u32::MAX;
/// Triangles the tracer accepts. Every triangle is stored in world space, so
/// instanced geometry counts once per instance.
pub(super) const MAX_TRIANGLES: usize = 16_000_000;

/// Triangles that share a material and a tint.
pub(super) struct TraceObject {
    pub(super) material: u32,
    pub(super) tint: Vec3,
    pub(super) tint_alpha: f32,
    /// Rays pass through back faces, as back-face culling does when
    /// rasterizing a single-sided material.
    pub(super) culls_back_faces: bool,
    /// Hits are tested against the surface's opacity.
    pub(super) tests_coverage: bool,
    /// Radius to which the object's edges are rounded when shading, or zero.
    pub(super) edge_radius: f32,
    pub(super) finish: Option<TraceFinish>,
}

pub(super) struct TriangleAttributes {
    pub(super) normals: [Vec3; 3],
    pub(super) uvs: [[f32; 2]; 3],
    pub(super) object: u32,
    /// Index into [`TraceScene::tangents`], or [`NONE`].
    pub(super) tangents: u32,
    /// Index into [`TraceScene::colors`], or [`NONE`].
    pub(super) colors: u32,
}

/// Scene content the tracer leaves out because it is not a lit surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Skipped {
    pub(super) stroke_nodes: u32,
    pub(super) label_nodes: u32,
    pub(super) particle_nodes: u32,
    pub(super) degenerate_triangles: u64,
}

pub(super) struct TraceScene {
    pub(super) positions: Vec<[Vec3; 3]>,
    pub(super) attributes: Vec<TriangleAttributes>,
    pub(super) tangents: Vec<[[f32; 4]; 3]>,
    pub(super) colors: Vec<[[f32; 4]; 3]>,
    pub(super) objects: Vec<TraceObject>,
    pub(super) materials: Vec<TraceMaterial>,
    pub(super) textures: Vec<TraceTexture>,
    pub(super) bvh: Bvh,
    pub(super) skipped: Skipped,
}

struct Builder<'a, F> {
    assets: &'a Assets<F>,
    /// World position every vertex is measured from, which keeps coordinates
    /// small and precise however far the scene lies from the world origin.
    origin: Vec3,
    detail: DetailSettings,
    store: TextureStore,
    material_slots: HashMap<MaterialHandle, Option<u32>>,
    vertex_color_material: Option<u32>,
    positions: Vec<[Vec3; 3]>,
    attributes: Vec<TriangleAttributes>,
    tangents: Vec<[[f32; 4]; 3]>,
    colors: Vec<[[f32; 4]; 3]>,
    objects: Vec<TraceObject>,
    materials: Vec<TraceMaterial>,
    skipped: Skipped,
}

/// One mesh to add: geometry in its deformed pose, placed in the world.
struct MeshSource<'a> {
    node: NodeKey,
    geometry: &'a GeometryDesc,
    vertices: &'a [GeometryVertex],
    /// The pose differs from the authored one, so cached tangents are stale.
    deformed: bool,
    morph_weights: Option<&'a [f32]>,
    world: WorldTransform,
    material: u32,
    tint: Option<Color>,
}

impl TraceScene {
    /// Builds the traced scene measured from `origin`, leaving out the
    /// nodes in `without`.
    pub(super) fn build<F>(
        scene: &Scene,
        assets: &Assets<F>,
        origin: Vec3,
        without: &[NodeKey],
        detail: DetailSettings,
    ) -> Result<Self, PathTraceError> {
        if let Some(node) = scene.model_nodes().next() {
            return Err(PathTraceError::UnsupportedModelNode { node });
        }
        let without: HashSet<NodeKey> = without.iter().copied().collect();
        let traced = |node: &NodeKey| !without.contains(node);
        let mut builder = Builder {
            assets,
            origin,
            detail,
            store: TextureStore::default(),
            material_slots: HashMap::new(),
            vertex_color_material: None,
            positions: Vec::new(),
            attributes: Vec::new(),
            tangents: Vec::new(),
            colors: Vec::new(),
            objects: Vec::new(),
            materials: Vec::new(),
            skipped: Skipped {
                label_nodes: scene.label_nodes().count() as u32,
                particle_nodes: scene.particle_set_nodes().count() as u32,
                ..Skipped::default()
            },
        };
        for (node, renderable, world) in scene.renderable_nodes().filter(|(node, ..)| traced(node))
        {
            let tint = scene.node_tint(node).unwrap_or(None);
            builder.add_primitives(renderable.primitives(), world, tint);
        }
        for (node, mesh, world) in scene.mesh_nodes().filter(|(node, ..)| traced(node)) {
            let geometry = assets
                .geometry_snapshot(mesh.geometry())
                .ok_or(PathTraceError::GeometryNotFound { node })?;
            let Some(material) = builder.material(node, mesh.material(), &geometry)? else {
                continue;
            };
            let skin = scene.skin_matrices(node);
            let vertices = geometry
                .deformed_vertices(scene.morph_weights(node), skin.as_deref())
                .map_err(|error| PathTraceError::InvalidGeometry {
                    node,
                    reason: format!("{error:?}"),
                })?;
            builder.add_mesh(MeshSource {
                node,
                geometry: &geometry,
                vertices: &vertices,
                deformed: matches!(vertices, std::borrow::Cow::Owned(_)),
                morph_weights: scene.morph_weights(node),
                world,
                material,
                tint: scene.node_tint(node).unwrap_or(None),
            })?;
        }
        for (node, set, world) in scene.instance_set_nodes().filter(|(node, ..)| traced(node)) {
            let geometry = assets
                .geometry_snapshot(set.geometry())
                .ok_or(PathTraceError::GeometryNotFound { node })?;
            let Some(material) = builder.material(node, set.material(), &geometry)? else {
                continue;
            };
            let node_tint = scene.node_tint(node).unwrap_or(None);
            for instance in set.instances().filter(|instance| instance.visible()) {
                builder.add_mesh(MeshSource {
                    node,
                    geometry: &geometry,
                    vertices: geometry.vertices(),
                    deformed: false,
                    morph_weights: None,
                    world: world.then(set.placement(instance)),
                    material,
                    tint: multiply_tint(node_tint, instance.tint()),
                })?;
            }
        }
        let bvh = Bvh::build(&builder.positions);
        Ok(Self {
            positions: builder.positions,
            attributes: builder.attributes,
            tangents: builder.tangents,
            colors: builder.colors,
            objects: builder.objects,
            materials: builder.materials,
            textures: builder.store.textures,
            bvh,
            skipped: builder.skipped,
        })
    }
}

fn multiply_tint(node: Option<Color>, instance: Option<Color>) -> Option<Color> {
    match (node, instance) {
        (Some(a), Some(b)) => Some(Color::from_linear_rgba(
            a.r * b.r,
            a.g * b.g,
            a.b * b.b,
            a.a * b.a,
        )),
        (tint, None) | (None, tint) => tint,
    }
}

impl<F> Builder<'_, F> {
    /// Resolves a material to its slot, or `None` when the node draws strokes
    /// rather than surfaces.
    fn material(
        &mut self,
        node: NodeKey,
        handle: MaterialHandle,
        geometry: &GeometryDesc,
    ) -> Result<Option<u32>, PathTraceError> {
        if geometry.topology() != GeometryTopology::Triangles {
            self.skipped.stroke_nodes += 1;
            return Ok(None);
        }
        if let Some(slot) = self.material_slots.get(&handle) {
            self.skipped.stroke_nodes += u32::from(slot.is_none());
            return Ok(*slot);
        }
        let desc = self
            .assets
            .material_snapshot(handle)
            .ok_or(PathTraceError::MaterialNotFound { node })?;
        let slot = match desc.kind() {
            MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {
                self.materials.push(TraceMaterial::from_desc(
                    &desc,
                    self.assets,
                    &mut self.store,
                ));
                Some(self.materials.len() as u32 - 1)
            }
            MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge => None,
        };
        self.skipped.stroke_nodes += u32::from(slot.is_none());
        self.material_slots.insert(handle, slot);
        Ok(slot)
    }

    fn object(&mut self, material: u32, tint: Option<Color>) -> u32 {
        let traced = &self.materials[material as usize];
        let tint_alpha = tint.map_or(1.0, |tint| tint.a.clamp(0.0, 1.0));
        self.objects.push(TraceObject {
            material,
            tint: tint.map_or(Vec3::ONE, |tint| Vec3::new(tint.r, tint.g, tint.b)),
            tint_alpha,
            culls_back_faces: !traced.double_sided && !traced.transmits(),
            tests_coverage: traced.coverage != Coverage::Opaque || tint_alpha < 1.0,
            edge_radius: 0.0,
            finish: None,
        });
        self.objects.len() as u32 - 1
    }

    fn add_mesh(&mut self, source: MeshSource<'_>) -> Result<(), PathTraceError> {
        let MeshSource {
            node,
            geometry,
            vertices,
            ..
        } = source;
        let world = relative_to(source.world, self.origin);
        let indices = geometry.indices();
        let triangle_count = if indices.is_empty() {
            vertices.len()
        } else {
            indices.len()
        } / 3;
        if self.positions.len() + triangle_count > MAX_TRIANGLES {
            return Err(PathTraceError::SceneTooLarge {
                triangles: (self.positions.len() + triangle_count) as u64,
                limit: MAX_TRIANGLES as u64,
            });
        }
        let object = self.object(source.material, source.tint);
        if self.detail.enabled && !self.materials[source.material as usize].unlit {
            let finish = self.materials[source.material as usize]
                .finish
                .map(|finish| TraceFinish::new(finish, world.matrix()));
            let object = &mut self.objects[object as usize];
            object.edge_radius = finish
                .as_ref()
                .map_or(self.detail.edge_radius, |finish| finish.edge_radius);
            object.finish = finish;
        }
        let model_tangents = self.materials[source.material as usize]
            .uses_tangents()
            .then(|| model_tangents(&source))
            .flatten();
        let colors = geometry.authored_vertex_colors();
        // Generated photographic textures repeat at a physical size, as the
        // rasterizer maps them.
        let uv_scale = self.materials[source.material as usize]
            .tile_size_m
            .and_then(|tile| geometry.photographic_texture_uv_scale(world.trs().scale, tile))
            .unwrap_or([1.0, 1.0]);
        let mirrored = world.is_mirrored();
        let corner = |triangle: usize, corner: usize| -> Result<usize, PathTraceError> {
            let slot = triangle * 3 + corner;
            let index = if indices.is_empty() {
                slot
            } else {
                indices[slot] as usize
            };
            (index < vertices.len()).then_some(index).ok_or_else(|| {
                PathTraceError::InvalidGeometry {
                    node,
                    reason: format!("index {index} is outside the {} vertices", vertices.len()),
                }
            })
        };
        for triangle in 0..triangle_count {
            // A mirrored placement reverses the winding; swapping two corners
            // keeps counter-clockwise the front face in world space.
            let order = if mirrored { [0, 2, 1] } else { [0, 1, 2] };
            let mut corners = [0_usize; 3];
            for (slot, corner_index) in order.into_iter().enumerate() {
                corners[slot] = corner(triangle, corner_index)?;
            }
            let positions = corners.map(|index| world.transform_point(vertices[index].position));
            let Some(face_normal) = face_normal(&positions) else {
                self.skipped.degenerate_triangles += 1;
                continue;
            };
            let tangents = model_tangents.as_deref().map_or(NONE, |tangents| {
                self.tangents.push(corners.map(|index| {
                    let tangent = tangents.get(index).copied().unwrap_or([1.0, 0.0, 0.0, 1.0]);
                    let direction = world
                        .transform_vector(Vec3::new(tangent[0], tangent[1], tangent[2]))
                        .normalize_or_zero();
                    let handedness = if mirrored { -tangent[3] } else { tangent[3] };
                    [direction.x, direction.y, direction.z, handedness]
                }));
                self.tangents.len() as u32 - 1
            });
            let colors = colors.map_or(NONE, |colors| {
                self.colors.push(corners.map(|index| {
                    let color = colors.get(index).copied().unwrap_or(Color::WHITE);
                    [color.r, color.g, color.b, color.a]
                }));
                self.colors.len() as u32 - 1
            });
            self.positions.push(positions);
            self.attributes.push(TriangleAttributes {
                normals: corners.map(|index| {
                    let normal = world
                        .transform_normal(vertices[index].normal)
                        .normalize_or_zero();
                    if normal == Vec3::ZERO {
                        face_normal
                    } else {
                        normal
                    }
                }),
                uvs: corners.map(|index| {
                    let uv = geometry.tex_coord0_or_default(index);
                    [uv[0] * uv_scale[0], uv[1] * uv_scale[1]]
                }),
                object,
                tangents,
                colors,
            });
        }
        Ok(())
    }

    /// Raw vertex-coloured triangles, which the rasterizer draws unlit.
    fn add_primitives(
        &mut self,
        primitives: &[Primitive],
        world: WorldTransform,
        tint: Option<Color>,
    ) {
        if primitives.is_empty() {
            return;
        }
        let material = *self.vertex_color_material.get_or_insert_with(|| {
            self.materials.push(TraceMaterial::vertex_colored());
            self.materials.len() as u32 - 1
        });
        let object = self.object(material, tint);
        let world = relative_to(world, self.origin);
        for primitive in primitives
            .iter()
            .take(MAX_TRIANGLES.saturating_sub(self.positions.len()))
        {
            let positions = primitive
                .vertices()
                .map(|vertex| world.transform_point(vertex.position));
            let Some(face_normal) = face_normal(&positions) else {
                self.skipped.degenerate_triangles += 1;
                continue;
            };
            self.colors.push(primitive.vertices().map(|vertex| {
                let color = vertex.color;
                [color.r, color.g, color.b, color.a]
            }));
            self.positions.push(positions);
            self.attributes.push(TriangleAttributes {
                normals: [face_normal; 3],
                uvs: primitive
                    .vertex_attributes()
                    .map(|attributes| attributes.tex_coord0),
                object,
                tangents: NONE,
                colors: self.colors.len() as u32 - 1,
            });
        }
    }
}

/// `world` measured from `origin`. The subtraction happens on the placement,
/// before any vertex is transformed, so vertices keep their full precision
/// however far the scene lies from the world origin.
fn relative_to(world: WorldTransform, origin: Vec3) -> WorldTransform {
    let mut matrix = world.matrix();
    matrix.translation -= glam::Vec3A::from(origin);
    WorldTransform::from_matrix(matrix)
}

/// Unit normal of a triangle, or `None` when it has no area or is not finite.
fn face_normal(positions: &[Vec3; 3]) -> Option<Vec3> {
    let normal = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
    let length = normal.length();
    (length.is_finite() && length > 0.0 && positions.iter().all(|position| position.is_finite()))
        .then(|| normal / length)
}

/// Model-space tangents for a mesh: authored ones when present, otherwise the
/// same generated basis the rasterizer uses.
fn model_tangents(source: &MeshSource<'_>) -> Option<std::sync::Arc<[[f32; 4]]>> {
    let geometry = source.geometry;
    if let Some(morphed) = source
        .morph_weights
        .and_then(|weights| geometry.morphed_tangents(weights))
    {
        return Some(morphed.into());
    }
    if let Some(authored) = geometry.tangents() {
        return Some(authored.into());
    }
    let tex_coords = geometry.authored_tex_coords0()?;
    let generate =
        || generate_model_tangents(source.vertices, geometry.indices(), Some(tex_coords));
    if source.deformed {
        return Some(generate().into());
    }
    Some(geometry.cached_generated_tangents(generate).0)
}

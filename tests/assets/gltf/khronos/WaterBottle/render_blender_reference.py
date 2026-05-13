"""Render WaterBottle through Blender Cycles → reference_blender_cycles_512.png

Run with:
  blender --background --python tests/assets/gltf/khronos/WaterBottle/render_blender_reference.py

Produces a third-party PBR reference render (Blender Cycles, 128 spp,
neutral studio lighting). The output is a loose material-family oracle.
The separate reference_512.png remains the scena-gold regression
baseline.
"""
import bpy
import math
import os
import mathutils
import sys

ASSET_DIR = os.path.dirname(os.path.abspath(__file__))
GLTF_PATH = os.path.join(ASSET_DIR, "WaterBottle.gltf")
OUTPUT_PATH = os.path.join(ASSET_DIR, "reference_blender_cycles_512.png")

# Wipe default scene
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete()

bpy.ops.import_scene.gltf(filepath=GLTF_PATH)

# Compute mesh bounds in world space
xs, ys, zs = [], [], []
for obj in bpy.context.scene.objects:
    if obj.type != "MESH":
        continue
    mw = obj.matrix_world
    for corner in obj.bound_box:
        v = mw @ mathutils.Vector(corner)
        xs.append(v.x)
        ys.append(v.y)
        zs.append(v.z)
extent = max(max(xs) - min(xs), max(ys) - min(ys), max(zs) - min(zs))
cx, cy, cz = ((max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2, (max(zs) + min(zs)) / 2)
print(f"WaterBottle extent={extent:.3f} centre=({cx:.3f},{cy:.3f},{cz:.3f})")

# Camera: 3/4 front view at distance ≈ extent * 1.6. Blender imports glTF
# into its Z-up world, so the view must orbit in X/Y and use Z only as a
# small height offset. The prior script orbited in X/Z and produced a
# top-down cap view, which made the third-party reference unusable as a
# visual oracle for the scena/Khronos front-view proof.
cam_data = bpy.data.cameras.new("Camera")
cam_data.lens = 50
cam = bpy.data.objects.new("Camera", cam_data)
bpy.context.scene.collection.objects.link(cam)

distance = extent * 1.6
yaw = math.radians(25.0)
cam.location = mathutils.Vector((
    cx + math.sin(yaw) * distance,
    cy - math.cos(yaw) * distance,
    cz + extent * 0.08,
))
target = mathutils.Vector((cx, cy, cz + extent * 0.03))
direction = target - cam.location
cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
bpy.context.scene.camera = cam

# Lighting: NEUTRAL studio. Matches what produces the canonical Khronos
# thumbnail look. NO directional lights — only ambient world light.
world = bpy.context.scene.world
if world is None:
    world = bpy.data.worlds.new("World")
    bpy.context.scene.world = world
world.use_nodes = True
bg = world.node_tree.nodes.get("Background")
if bg is None:
    bg = world.node_tree.nodes.new("ShaderNodeBackground")
# Warm tan studio background, low intensity
bg.inputs[0].default_value = (0.82, 0.66, 0.50, 1.0)
bg.inputs[1].default_value = 0.8

# Render settings
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.device = "CPU"
scene.cycles.samples = 128
scene.cycles.use_denoising = False
if hasattr(scene, "view_layers"):
    for vl in scene.view_layers:
        vl.cycles.use_denoising = False
scene.render.resolution_x = 512
scene.render.resolution_y = 512
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = "PNG"
scene.render.filepath = OUTPUT_PATH
scene.view_settings.view_transform = "Standard"

bpy.ops.render.render(write_still=True)
print(f"wrote {OUTPUT_PATH}")

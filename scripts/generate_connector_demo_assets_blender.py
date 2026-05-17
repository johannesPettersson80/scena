#!/usr/bin/env blender --background --python

import math
from pathlib import Path

import bpy

REPO = Path(__file__).resolve().parents[1]
TEST_ASSET_DIR = REPO / "tests/assets/gltf"
DEMO_ASSET_DIR = REPO / "demo/samples/connector-snap"

AXIS_Y = 0.07
FLOOR_Y = -0.18
PLATE_THICKNESS = 0.028
ASSEMBLY_DRIVE_OFFSET_X = -0.615

DRIVE_SHAFT_CONNECTOR = (0.66, AXIS_Y, 0.0)
LOAD_HUB_CONNECTOR = (0.045, AXIS_Y, 0.0)

SMOOTH_SMALL = 32
SMOOTH_MEDIUM = 44
SMOOTH_LARGE = 64


def reset_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.context.scene.render.engine = "CYCLES"
    bpy.context.scene.unit_settings.system = "METRIC"


def material(name, color, metallic=0.0, roughness=0.45, specular=0.5):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if "Specular IOR Level" in bsdf.inputs:
        bsdf.inputs["Specular IOR Level"].default_value = specular
    return mat


def materials():
    return {
        "steel": material("polished shaft steel", (0.42, 0.45, 0.48, 1.0), 1.0, 0.18, 0.65),
        "machined": material("machined aluminium", (0.30, 0.33, 0.36, 1.0), 1.0, 0.26, 0.60),
        "navy": material("navy powder coat", (0.018, 0.055, 0.115, 1.0), 0.0, 0.30, 0.55),
        "anodized": material("dark anodized aluminium", (0.020, 0.025, 0.030, 1.0), 1.0, 0.21, 0.65),
        "cast_iron": material("cast iron", (0.08, 0.08, 0.09, 1.0), 0.55, 0.55, 0.40),
        "plate": material("brushed baseplate steel", (0.10, 0.12, 0.14, 1.0), 0.65, 0.42, 0.50),
        "bolt": material("stainless bolt", (0.46, 0.48, 0.50, 1.0), 1.0, 0.18, 0.60),
        "rubber": material("isolator pad", (0.04, 0.04, 0.05, 1.0), 0.0, 0.85, 0.30),
        "bore": material("recessed bore", (0.012, 0.014, 0.018, 1.0), 0.30, 0.62, 0.30),
        "label": material("data plate", (0.78, 0.74, 0.55, 1.0), 0.35, 0.40, 0.45),
    }


def gltf_to_blender_position(value):
    return (value[0], value[2], value[1])


def gltf_to_blender_size(value):
    return (value[0], value[2], value[1])


def parent_to(obj, parent):
    obj.parent = parent
    return obj


def shade_smooth(obj):
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.shade_smooth()
    obj.select_set(False)


def shade_flat(obj):
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.shade_flat()
    obj.select_set(False)


def add_weighted_normals(obj):
    obj.modifiers.new("weighted normals", "WEIGHTED_NORMAL")


def add_bevel(obj, width=0.012, segments=2, only_edges=True):
    if width <= 0:
        return
    mod = obj.modifiers.new("bevel", "BEVEL")
    mod.width = width
    mod.segments = segments
    mod.limit_method = "ANGLE"
    mod.angle_limit = math.radians(35.0)
    if only_edges:
        mod.affect = "EDGES"


def apply_modifiers(obj):
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    for mod in list(obj.modifiers):
        bpy.ops.object.modifier_apply(modifier=mod.name)
    triangulate = obj.modifiers.new("glTF tangent triangulation", "TRIANGULATE")
    triangulate.quad_method = "BEAUTY"
    triangulate.ngon_method = "BEAUTY"
    bpy.ops.object.modifier_apply(modifier=triangulate.name)
    obj.select_set(False)


def merge_parts_by_material(root, parts, name_prefix):
    groups = {}
    for part in parts:
        material_name = part.data.materials[0].name if part.data.materials else "unmaterialed"
        groups.setdefault(material_name, []).append(part)

    merged = []
    for material_name, group in groups.items():
        for part in group:
            apply_modifiers(part)

        if len(group) == 1:
            obj = group[0]
        else:
            bpy.ops.object.select_all(action="DESELECT")
            active = group[0]
            bpy.context.view_layer.objects.active = active
            for part in group:
                part.select_set(True)
            bpy.ops.object.join()
            obj = bpy.context.object

        safe_material_name = material_name.lower().replace(" ", "_")
        obj.name = f"{name_prefix}_{safe_material_name}"
        parent_to(obj, root)
        merged.append(obj)

    return merged


def bevelled_box(name, center, size, mat, bevel=0.012):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=gltf_to_blender_position(center))
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = gltf_to_blender_size(size)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if mat:
        obj.data.materials.append(mat)
    add_bevel(obj, bevel)
    add_weighted_normals(obj)
    shade_flat(obj)
    return obj


def cylinder_x(name, center, length, radius, mat, vertices=SMOOTH_MEDIUM, bevel=0.0):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=length,
        end_fill_type="NGON",
        location=gltf_to_blender_position(center),
        rotation=(0.0, math.radians(90.0), 0.0),
    )
    obj = bpy.context.object
    obj.name = name
    if mat:
        obj.data.materials.append(mat)
    shade_smooth(obj)
    if bevel > 0:
        add_bevel(obj, bevel, segments=2)
    add_weighted_normals(obj)
    return obj


def cylinder_y(name, center, length, radius, mat, vertices=SMOOTH_SMALL, bevel=0.0):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=length,
        end_fill_type="NGON",
        location=gltf_to_blender_position(center),
    )
    obj = bpy.context.object
    obj.name = name
    if mat:
        obj.data.materials.append(mat)
    shade_smooth(obj)
    if bevel > 0:
        add_bevel(obj, bevel, segments=2)
    add_weighted_normals(obj)
    return obj


def torus_x(name, center, major_radius, minor_radius, mat, major_segments=SMOOTH_LARGE, minor_segments=8):
    bpy.ops.mesh.primitive_torus_add(
        major_segments=major_segments,
        minor_segments=minor_segments,
        major_radius=major_radius,
        minor_radius=minor_radius,
        location=gltf_to_blender_position(center),
        rotation=(0.0, math.radians(90.0), 0.0),
    )
    obj = bpy.context.object
    obj.name = name
    if mat:
        obj.data.materials.append(mat)
    shade_smooth(obj)
    add_weighted_normals(obj)
    return obj


def hex_bolt_on_yz_face(name, center, head_height, head_radius, mat):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=6,
        radius=head_radius,
        depth=head_height,
        end_fill_type="NGON",
        location=gltf_to_blender_position((center[0] + head_height * 0.5, center[1], center[2])),
        rotation=(0.0, math.radians(90.0), 0.0),
    )
    obj = bpy.context.object
    obj.name = name
    if mat:
        obj.data.materials.append(mat)
    shade_flat(obj)
    add_bevel(obj, head_height * 0.18, segments=2)
    return obj


def bolt_ring_on_yz_face(name_prefix, center_x, axis_y, axis_z, ring_radius, count, head_radius, head_height, mat):
    bolts = []
    for index in range(count):
        angle = math.tau * index / count
        y = axis_y + math.cos(angle) * ring_radius
        z = axis_z + math.sin(angle) * ring_radius
        bolts.append(
            hex_bolt_on_yz_face(
                f"{name_prefix} bolt {index + 1}",
                (center_x, y, z),
                head_height,
                head_radius,
                mat,
            )
        )
    return bolts


def hex_bolt_on_yz_face_facing_minus_x(name, center, head_height, head_radius, mat):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=6,
        radius=head_radius,
        depth=head_height,
        end_fill_type="NGON",
        location=gltf_to_blender_position((center[0] - head_height * 0.5, center[1], center[2])),
        rotation=(0.0, math.radians(90.0), 0.0),
    )
    obj = bpy.context.object
    obj.name = name
    if mat:
        obj.data.materials.append(mat)
    shade_flat(obj)
    add_bevel(obj, head_height * 0.18, segments=2)
    return obj


def bolt_ring_facing_minus_x(name_prefix, center_x, axis_y, axis_z, ring_radius, count, head_radius, head_height, mat):
    bolts = []
    for index in range(count):
        angle = math.tau * index / count
        y = axis_y + math.cos(angle) * ring_radius
        z = axis_z + math.sin(angle) * ring_radius
        bolts.append(
            hex_bolt_on_yz_face_facing_minus_x(
                f"{name_prefix} bolt {index + 1}",
                (center_x, y, z),
                head_height,
                head_radius,
                mat,
            )
        )
    return bolts


def hex_bolt_vertical(name, top_center, head_height, head_radius, mat):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=6,
        radius=head_radius,
        depth=head_height,
        end_fill_type="NGON",
        location=gltf_to_blender_position(
            (top_center[0], top_center[1] - head_height * 0.5, top_center[2])
        ),
    )
    obj = bpy.context.object
    obj.name = name
    if mat:
        obj.data.materials.append(mat)
    shade_flat(obj)
    add_bevel(obj, head_height * 0.18, segments=2)
    return obj


def cooling_fins(name_prefix, axis_x_center, axis_y, axis_z, length, body_radius, fin_radius, count, mat):
    fins = []
    fin_thickness = 0.006
    span = length - 0.04
    start_x = axis_x_center - span * 0.5
    step = span / max(count - 1, 1)
    for index in range(count):
        x = start_x + step * index
        fin = cylinder_x(
            f"{name_prefix} fin {index + 1}",
            (x, axis_y, axis_z),
            fin_thickness,
            fin_radius,
            mat,
            vertices=SMOOTH_SMALL,
            bevel=0.001,
        )
        fins.append(fin)
    return fins


def root_empty(name, connector):
    obj = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(obj)
    obj["scena"] = {"connectors": [connector]}
    return obj


def connector(name, kind, allowed_mates, polarity, translation):
    return {
        "name": name,
        "kind": kind,
        "translation": translation,
        "forward": [1.0, 0.0, 0.0],
        "up": [0.0, 1.0, 0.0],
        "allowedMates": allowed_mates,
        "tags": ["assembly", "canonical-demo"],
        "snapTolerance": 0.01,
        "clearanceHint": 0.002,
        "rollPolicy": "chooseNearest",
        "polarity": polarity,
        "metadata": {
            "author": "scena canonical connector demo",
            "generatedBy": "scripts/generate_connector_demo_assets_blender.py",
        },
    }


def drive_unit(mats):
    parts = []
    root = root_empty(
        "drive_unit",
        connector("shaft", "shaft", ["hub"], "plug", list(DRIVE_SHAFT_CONNECTOR)),
    )

    plate_center_y = FLOOR_Y - PLATE_THICKNESS * 0.5
    parts.append(
        bevelled_box(
            "drive baseplate",
            (-0.04, plate_center_y, 0.0),
            (1.05, PLATE_THICKNESS, 0.40),
            mats["plate"],
            bevel=0.010,
        )
    )

    for sign_x in (-1.0, 1.0):
        for sign_z in (-1.0, 1.0):
            parts.append(
                hex_bolt_vertical(
                    f"drive plate bolt {('p' if sign_x > 0 else 'n')}{('p' if sign_z > 0 else 'n')}",
                    (-0.04 + sign_x * 0.49, FLOOR_Y + 0.0006, sign_z * 0.175),
                    head_height=0.014,
                    head_radius=0.012,
                    mat=mats["bolt"],
                )
            )

    for sign_z in (-1.0, 1.0):
        parts.append(
            bevelled_box(
                f"drive isolator {('rear' if sign_z > 0 else 'front')}",
                (-0.30, FLOOR_Y + 0.012, sign_z * 0.115),
                (0.30, 0.018, 0.05),
                mats["rubber"],
                bevel=0.004,
            )
        )

    motor_axis = (-0.30, AXIS_Y, 0.0)
    parts.append(
        bevelled_box(
            "drive motor cradle",
            (-0.30, FLOOR_Y + 0.055, 0.0),
            (0.34, 0.064, 0.34),
            mats["navy"],
            bevel=0.012,
        )
    )

    motor_length = 0.34
    motor_radius = 0.115
    parts.append(
        cylinder_x(
            "motor steel body",
            motor_axis,
            motor_length,
            motor_radius,
            mats["steel"],
            vertices=SMOOTH_LARGE,
        )
    )
    for offset, radius in ((-0.13, motor_radius + 0.008), (0.13, motor_radius + 0.008)):
        parts.append(
            torus_x(
                f"motor polished end band {offset:+.2f}",
                (motor_axis[0] + offset, AXIS_Y, 0.0),
                radius,
                0.003,
                mats["bolt"],
                major_segments=SMOOTH_LARGE,
                minor_segments=8,
            )
        )

    parts.extend(
        cooling_fins(
            "motor",
            axis_x_center=motor_axis[0],
            axis_y=motor_axis[1],
            axis_z=motor_axis[2],
            length=motor_length,
            body_radius=motor_radius,
            fin_radius=motor_radius + 0.014,
            count=9,
            mat=mats["steel"],
        )
    )

    end_cap_x = motor_axis[0] - motor_length * 0.5 - 0.011
    parts.append(
        cylinder_x(
            "motor end cap",
            (end_cap_x, AXIS_Y, 0.0),
            0.022,
            motor_radius + 0.005,
            mats["machined"],
            vertices=SMOOTH_LARGE,
            bevel=0.004,
        )
    )

    parts.extend(
        bolt_ring_facing_minus_x(
            "motor end cap",
            end_cap_x - 0.011,
            AXIS_Y,
            0.0,
            ring_radius=motor_radius - 0.018,
            count=6,
            head_radius=0.011,
            head_height=0.009,
            mat=mats["bolt"],
        )
    )

    parts.append(
        bevelled_box(
            "motor data plate",
            (motor_axis[0], AXIS_Y, motor_radius + 0.001),
            (0.13, 0.07, 0.001),
            mats["label"],
            bevel=0.002,
        )
    )

    shroud_x = motor_axis[0] + motor_length * 0.5 + 0.075
    parts.append(
        bevelled_box(
            "motor fan shroud",
            (shroud_x, AXIS_Y, 0.0),
            (0.15, 0.225, 0.225),
            mats["navy"],
            bevel=0.018,
        )
    )

    gearbox_x = shroud_x + 0.205
    parts.append(
        bevelled_box(
            "reduction gearbox",
            (gearbox_x, AXIS_Y, 0.0),
            (0.255, 0.205, 0.205),
            mats["machined"],
            bevel=0.014,
        )
    )

    parts.append(
        bevelled_box(
            "gearbox cover plate",
            (gearbox_x, AXIS_Y + 0.105, 0.0),
            (0.225, 0.012, 0.175),
            mats["machined"],
            bevel=0.004,
        )
    )

    for cover_index, sign_x in enumerate((-1.0, 1.0)):
        for sign_z in (-1.0, 1.0):
            parts.append(
                hex_bolt_vertical(
                    f"gearbox cover bolt {cover_index}{('p' if sign_z > 0 else 'n')}",
                    (gearbox_x + sign_x * 0.090, AXIS_Y + 0.111 + 0.005, sign_z * 0.072),
                    head_height=0.009,
                    head_radius=0.007,
                    mat=mats["bolt"],
                )
            )

    flange_x = gearbox_x + 0.255 * 0.5 + 0.018
    parts.append(
        cylinder_x(
            "gearbox output flange",
            (flange_x, AXIS_Y, 0.0),
            0.030,
            0.090,
            mats["machined"],
            vertices=SMOOTH_MEDIUM,
            bevel=0.003,
        )
    )

    parts.extend(
        bolt_ring_on_yz_face(
            "gearbox flange",
            flange_x + 0.016,
            AXIS_Y,
            0.0,
            ring_radius=0.062,
            count=6,
            head_radius=0.009,
            head_height=0.008,
            mat=mats["bolt"],
        )
    )

    bearing_x = flange_x + 0.030
    parts.append(
        cylinder_x(
            "drive bearing collar",
            (bearing_x, AXIS_Y, 0.0),
            0.024,
            0.038,
            mats["steel"],
            vertices=SMOOTH_MEDIUM,
            bevel=0.002,
        )
    )

    shaft_start_x = bearing_x + 0.012
    shaft_tip_x = 0.685
    shaft_length = shaft_tip_x - shaft_start_x
    shaft_center_x = shaft_start_x + shaft_length * 0.5
    parts.append(
        cylinder_x(
            "drive shaft",
            (shaft_center_x, AXIS_Y, 0.0),
            shaft_length,
            0.022,
            mats["steel"],
            vertices=SMOOTH_MEDIUM,
        )
    )
    parts.append(
        bevelled_box(
            "drive shaft keyway",
            (shaft_center_x + 0.018, AXIS_Y + 0.0225, 0.0),
            (0.170, 0.004, 0.012),
            mats["bore"],
            bevel=0.001,
        )
    )

    parts.append(
        cylinder_x(
            "drive shaft shoulder",
            (shaft_start_x + 0.020, AXIS_Y, 0.0),
            0.022,
            0.026,
            mats["steel"],
            vertices=SMOOTH_MEDIUM,
            bevel=0.002,
        )
    )

    merge_parts_by_material(root, parts, "drive")

    return root


def load_unit(mats):
    parts = []
    root = root_empty(
        "load_unit",
        connector("hub", "hub", ["shaft"], "socket", list(LOAD_HUB_CONNECTOR)),
    )

    plate_center_y = FLOOR_Y - PLATE_THICKNESS * 0.5
    parts.append(
        bevelled_box(
            "load baseplate",
            (0.16, plate_center_y, 0.0),
            (0.56, PLATE_THICKNESS, 0.46),
            mats["plate"],
            bevel=0.010,
        )
    )

    for sign_x in (-1.0, 1.0):
        for sign_z in (-1.0, 1.0):
            parts.append(
                hex_bolt_vertical(
                    f"load plate bolt {('p' if sign_x > 0 else 'n')}{('p' if sign_z > 0 else 'n')}",
                    (0.16 + sign_x * 0.245, FLOOR_Y + 0.0006, sign_z * 0.200),
                    head_height=0.014,
                    head_radius=0.012,
                    mat=mats["bolt"],
                )
            )

    pedestal_center = (0.165, FLOOR_Y + 0.115, 0.0)
    parts.append(
        bevelled_box(
            "load pedestal column",
            pedestal_center,
            (0.18, 0.230, 0.24),
            mats["machined"],
            bevel=0.014,
        )
    )

    parts.append(
        bevelled_box(
            "load pedestal flange",
            (0.165, FLOOR_Y + 0.236, 0.0),
            (0.225, 0.022, 0.30),
            mats["machined"],
            bevel=0.005,
        )
    )

    for sign_z in (-1.0, 1.0):
        for sign_x in (-1.0, 1.0):
            parts.append(
                hex_bolt_vertical(
                    f"pedestal flange bolt {('p' if sign_x > 0 else 'n')}{('p' if sign_z > 0 else 'n')}",
                    (0.165 + sign_x * 0.090, FLOOR_Y + 0.247 + 0.005, sign_z * 0.115),
                    head_height=0.010,
                    head_radius=0.0075,
                    mat=mats["bolt"],
                )
            )

    hub_flange_x = LOAD_HUB_CONNECTOR[0] + 0.018
    parts.append(
        cylinder_x(
            "hub bearing flange",
            (hub_flange_x, AXIS_Y, 0.0),
            0.036,
            0.105,
            mats["machined"],
            vertices=SMOOTH_LARGE,
            bevel=0.004,
        )
    )
    parts.append(
        torus_x(
            "hub polished lip",
            (hub_flange_x - 0.018, AXIS_Y, 0.0),
            0.104,
            0.004,
            mats["bolt"],
            major_segments=SMOOTH_LARGE,
            minor_segments=8,
        )
    )

    parts.extend(
        bolt_ring_facing_minus_x(
            "hub flange",
            LOAD_HUB_CONNECTOR[0] + 0.001,
            AXIS_Y,
            0.0,
            ring_radius=0.078,
            count=6,
            head_radius=0.010,
            head_height=0.010,
            mat=mats["bolt"],
        )
    )

    parts.append(
        cylinder_x(
            "hub socket bore",
            (LOAD_HUB_CONNECTOR[0] + 0.008, AXIS_Y, 0.0),
            0.018,
            0.030,
            mats["bore"],
            vertices=SMOOTH_MEDIUM,
        )
    )

    hub_housing_x = hub_flange_x + 0.018 + 0.058
    parts.append(
        cylinder_x(
            "hub housing",
            (hub_housing_x, AXIS_Y, 0.0),
            0.118,
            0.082,
            mats["machined"],
            vertices=SMOOTH_LARGE,
            bevel=0.004,
        )
    )

    flywheel_face_x = hub_housing_x + 0.118 * 0.5 + 0.010
    flywheel_length = 0.085
    flywheel_center_x = flywheel_face_x + flywheel_length * 0.5
    flywheel_radius = 0.215
    parts.append(
        cylinder_x(
            "flywheel disk",
            (flywheel_center_x, AXIS_Y, 0.0),
            flywheel_length,
            flywheel_radius,
            mats["anodized"],
            vertices=SMOOTH_LARGE,
            bevel=0.005,
        )
    )
    parts.append(
        torus_x(
            "flywheel bright outer bevel",
            (flywheel_face_x - 0.001, AXIS_Y, 0.0),
            flywheel_radius - 0.010,
            0.005,
            mats["bolt"],
            major_segments=SMOOTH_LARGE,
            minor_segments=8,
        )
    )
    parts.append(
        torus_x(
            "flywheel recessed inner groove",
            (flywheel_face_x - 0.003, AXIS_Y, 0.0),
            flywheel_radius - 0.075,
            0.004,
            mats["bore"],
            major_segments=SMOOTH_MEDIUM,
            minor_segments=8,
        )
    )

    parts.append(
        cylinder_x(
            "flywheel inner ring",
            (flywheel_center_x, AXIS_Y, 0.0),
            flywheel_length + 0.002,
            flywheel_radius - 0.075,
            mats["machined"],
            vertices=SMOOTH_LARGE,
        )
    )

    parts.append(
        cylinder_x(
            "flywheel hub cap",
            (flywheel_face_x - 0.012, AXIS_Y, 0.0),
            0.024,
            0.055,
            mats["steel"],
            vertices=SMOOTH_MEDIUM,
            bevel=0.003,
        )
    )

    parts.extend(
        bolt_ring_facing_minus_x(
            "flywheel",
            flywheel_face_x - 0.001,
            AXIS_Y,
            0.0,
            ring_radius=0.110,
            count=8,
            head_radius=0.011,
            head_height=0.010,
            mat=mats["bolt"],
        )
    )

    for index, angle_deg in enumerate((0.0, 90.0, 180.0, 270.0)):
        angle = math.radians(angle_deg)
        parts.append(
            cylinder_x(
                f"flywheel lightening hole {index + 1}",
                (flywheel_center_x, AXIS_Y + math.cos(angle) * 0.160, math.sin(angle) * 0.160),
                flywheel_length + 0.003,
                0.028,
                mats["bore"],
                vertices=SMOOTH_SMALL,
            )
        )

    merge_parts_by_material(root, parts, "load")

    return root


def export_glb(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        export_extras=True,
        export_apply=True,
        export_tangents=True,
        export_yup=True,
    )
    print(f"wrote {path.relative_to(REPO)}")


def write_drive():
    reset_scene()
    root = drive_unit(materials())
    root.select_set(True)
    export_glb(TEST_ASSET_DIR / "drive_unit.glb")
    export_glb(DEMO_ASSET_DIR / "drive_unit.glb")


def write_load():
    reset_scene()
    root = load_unit(materials())
    root.select_set(True)
    export_glb(TEST_ASSET_DIR / "load_unit.glb")
    export_glb(DEMO_ASSET_DIR / "load_unit.glb")


def write_assembly():
    reset_scene()
    mats = materials()
    load_unit(mats)
    drive = drive_unit(mats)
    drive.location.x = ASSEMBLY_DRIVE_OFFSET_X
    export_glb(DEMO_ASSET_DIR / "connector_snap_assembly.glb")


if __name__ == "__main__":
    write_drive()
    write_load()
    write_assembly()

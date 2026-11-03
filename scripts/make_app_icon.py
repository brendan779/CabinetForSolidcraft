#!/usr/bin/env python3
"""Renders the app icon: a base cabinet, its left half as built and its right half as CAD wireframe.

The wood is procedural, so no brand's decor image ends up in the public repo.

    /Applications/Blender.app/Contents/MacOS/Blender -b -P scripts/make_app_icon.py -- out.png [--samples 512]
    /Applications/Blender.app/Contents/MacOS/Blender -b -P scripts/make_app_icon.py -- --install

--install renders apps/macos/CabinetStudio/Assets.xcassets/AppIcon.appiconset, at every size.
"""
import math
import os
import subprocess
import sys

import bpy
import numpy as np
from mathutils import Vector

SIZE = 1024
MM = 0.001
# Theme.swift's colours, as icon's background and wire.
TOP, BOTTOM = (0x4B, 0x42, 0x38), (0x1B, 0x18, 0x14)
ACCENT = (0xE8, 0x84, 0x5E)
# macOS's icon grid: the body sits inside the canvas, with room for its shadow.
BODY = 824
WIRE_RADIUS = 2.8
BEVEL = 1.2  # mm: edges catch the light, as a real panel's do
W, D, H = 900.0, 580.0, 870.0
PLINTH, T, GAP = 120.0, 18.0, 3.0
SPLIT = W / 2


def linear(srgb):
    c = [v / 255 for v in srgb]
    return tuple(x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c)


def parts():
    """(name, material, lo, hi) in mm: X across the front, Y from the front face to the back, Z up."""
    top = H
    drawer = H - 190
    door_mid = W / 2
    out = [
        ("Plinth", "plinth", (T, 40, 0), (W - T, D, PLINTH)),
        ("Left side", "wood", (0, 22, PLINTH), (T, D, top)),
        ("Right side", "carcass", (W - T, 22, PLINTH), (W, D, top)),
        ("Bottom", "carcass", (T, 22, PLINTH), (W - T, D - 16, PLINTH + T)),
        ("Back", "carcass", (T, D - 16, PLINTH), (W - T, D, top)),
        ("Top rail", "carcass", (T, 22, top - T), (W - T, 122, top)),
        ("Divider", "carcass", (T, 22, drawer - T), (W - T, D - 16, drawer)),
        ("Shelf", "carcass", (T, 40, (PLINTH + drawer) / 2 - T / 2), (W - T, D - 16, (PLINTH + drawer) / 2 + T / 2)),
        ("Drawer front", "wood_across", (GAP, 0, drawer + GAP), (W - GAP, 18, top - GAP)),
        ("Left door", "wood", (GAP, 0, PLINTH + GAP), (door_mid - GAP / 2, 18, drawer - GAP)),
        ("Right door", "wood", (door_mid + GAP / 2, 0, PLINTH + GAP), (W - GAP, 18, drawer - GAP)),
        ("Benchtop", "stone", (-14, -24, top), (W + 14, D + 6, top + 33)),
    ]
    # Bar handles: two posts and a bar, on the fronts.
    pull = 150.0
    for name, cx, cz, across in (
        ("Drawer pull", W / 2, (drawer + top) / 2, True),
        ("Left pull", door_mid - 50, drawer - 150, False),
        ("Right pull", door_mid + 50, drawer - 150, False),
    ):
        if across:
            out.append((name, "brass", (cx - pull, -34, cz - 6), (cx + pull, -22, cz + 6)))
            for dx in (-pull + 25, pull - 25):
                out.append((name + " post", "brass", (cx + dx - 5, -22, cz - 5), (cx + dx + 5, 0, cz + 5)))
        else:
            out.append((name, "brass", (cx - 6, -34, cz - pull * 0.65), (cx + 6, -22, cz + pull * 0.65)))
            for dz in (-pull * 0.65 + 20, pull * 0.65 - 20):
                out.append((name + " post", "brass", (cx - 5, -22, cz + dz - 5), (cx + 5, 0, cz + dz + 5)))
    return out


def box_mesh(name, lo, hi, material, into):
    x0, y0, z0 = (v * MM for v in lo)
    x1, y1, z1 = (v * MM for v in hi)
    verts = [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0), (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)]
    faces = [(3, 2, 1, 0), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    me.update()
    me.materials.append(material)
    ob = bpy.data.objects.new(name, me)
    into.objects.link(ob)
    return ob


def wire_box(name, lo, hi, material, into):
    """A box's twelve edges as thin bars, one mesh."""
    x = (lo[0], hi[0])
    y = (lo[1], hi[1])
    z = (lo[2], hi[2])
    bars = []
    for a in y:
        for b in z:
            bars.append(((x[0], a, b), (x[1], a, b)))
    for a in x:
        for b in z:
            bars.append(((a, y[0], b), (a, y[1], b)))
    for a in x:
        for b in y:
            bars.append(((a, b, z[0]), (a, b, z[1])))
    for i, (p, q) in enumerate(bars):
        r = WIRE_RADIUS
        box_mesh(
            f"{name} {i}",
            tuple(min(p[k], q[k]) - r for k in range(3)),
            tuple(max(p[k], q[k]) + r for k in range(3)),
            material,
            into,
        ).visible_shadow = False


def wood(name, across):
    """Crown-cut walnut: long straight streaks over a slow drift of colour, each part from its own
    place on the tree."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    t = m.node_tree
    b = t.nodes["Principled BSDF"]
    b.inputs["Roughness"].default_value = 0.42
    b.inputs["Coat Weight"].default_value = 0.2
    b.inputs["Coat Roughness"].default_value = 0.3
    coord = t.nodes.new("ShaderNodeTexCoord")
    turned = t.nodes.new("ShaderNodeMapping")
    # Grain runs along Z; across a drawer front, along X.
    if across:
        turned.inputs["Rotation"].default_value = (0, math.pi / 2, 0)
    t.links.new(coord.outputs["Object"], turned.inputs["Vector"])
    info = t.nodes.new("ShaderNodeObjectInfo")
    shift = t.nodes.new("ShaderNodeMath")
    shift.operation = "MULTIPLY"
    shift.inputs[1].default_value = 40.0
    t.links.new(info.outputs["Random"], shift.inputs[0])
    offset = t.nodes.new("ShaderNodeVectorMath")
    t.links.new(turned.outputs[0], offset.inputs[0])
    t.links.new(shift.outputs[0], offset.inputs[1])

    def streaks(scale, stretch, detail, distortion):
        along = t.nodes.new("ShaderNodeMapping")
        along.inputs["Scale"].default_value = (1.0, 1.0, stretch)
        t.links.new(offset.outputs[0], along.inputs["Vector"])
        noise = t.nodes.new("ShaderNodeTexNoise")
        noise.inputs["Scale"].default_value = scale
        noise.inputs["Detail"].default_value = detail
        noise.inputs["Distortion"].default_value = distortion
        t.links.new(along.outputs[0], noise.inputs["Vector"])
        return noise.outputs[0]

    def between(value, lo, hi, low=0.0, high=1.0):
        span = t.nodes.new("ShaderNodeMapRange")
        span.inputs["From Min"].default_value = lo
        span.inputs["From Max"].default_value = hi
        span.inputs["To Min"].default_value = low
        span.inputs["To Max"].default_value = high
        t.links.new(value, span.inputs["Value"])
        return span.outputs[0]

    ramp = t.nodes.new("ShaderNodeValToRGB")
    e = ramp.color_ramp.elements
    e[0].position, e[0].color = 0.0, (*linear((0x4E, 0x31, 0x20)), 1)
    e[1].position, e[1].color = 1.0, (*linear((0x8C, 0x60, 0x3F)), 1)
    t.links.new(between(streaks(16.0, 0.03, 5.0, 0.35), 0.36, 0.64), ramp.inputs["Fac"])
    mix = t.nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    mix.blend_type = "MULTIPLY"
    mix.inputs[0].default_value = 1.0
    t.links.new(ramp.outputs[0], mix.inputs[6])
    pores = between(streaks(150.0, 0.015, 3.0, 0.0), 0.38, 0.56, 0.68, 1.0)
    t.links.new(pores, mix.inputs[7])
    t.links.new(mix.outputs[2], b.inputs["Base Color"])
    bump = t.nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = 0.08
    bump.inputs["Distance"].default_value = 0.001
    t.links.new(pores, bump.inputs["Height"])
    t.links.new(bump.outputs[0], b.inputs["Normal"])
    return m


def plain(name, colour, roughness, metallic=0.0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*linear(colour), 1)
    b.inputs["Roughness"].default_value = roughness
    b.inputs["Metallic"].default_value = metallic
    return m


def stone():
    m = plain("Stone", (0xE9, 0xE5, 0xDC), 0.22)
    t = m.node_tree
    b = t.nodes["Principled BSDF"]
    noise = t.nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 26.0
    noise.inputs["Detail"].default_value = 8.0
    ramp = t.nodes.new("ShaderNodeValToRGB")
    e = ramp.color_ramp.elements
    e[0].position, e[0].color = 0.3, (*linear((0xE3, 0xDF, 0xD6)), 1)
    e[1].position, e[1].color = 0.7, (*linear((0xEF, 0xEC, 0xE4)), 1)
    t.links.new(noise.outputs[0], ramp.inputs["Fac"])
    t.links.new(ramp.outputs[0], b.inputs["Base Color"])
    return m


def wire_material():
    m = bpy.data.materials.new("Wire")
    m.use_nodes = True
    t = m.node_tree
    t.nodes.clear()
    out = t.nodes.new("ShaderNodeOutputMaterial")
    e = t.nodes.new("ShaderNodeEmission")
    e.inputs["Color"].default_value = (*linear(ACCENT), 1)
    e.inputs["Strength"].default_value = 1.0
    t.links.new(e.outputs[0], out.inputs["Surface"])
    return m


def area(name, at, aim, size, energy, colour=(1, 1, 1)):
    d = bpy.data.lights.new(name, "AREA")
    d.size = size
    d.energy = energy
    d.color = colour
    ob = bpy.data.objects.new(name, d)
    ob.location = at
    ob.rotation_euler = (Vector(aim) - Vector(at)).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.collection.objects.link(ob)


def build():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    into = sc.collection
    mats = {
        "wood": wood("Walnut", False),
        "wood_across": wood("Walnut across", True),
        "carcass": plain("Carcass", (0xEF, 0xE9, 0xDC), 0.5),
        "plinth": plain("Plinth", (0x2A, 0x25, 0x20), 0.45),
        "stone": stone(),
        "brass": plain("Brass", (0xD8, 0xB2, 0x6A), 0.25, 1.0),
    }
    wire = wire_material()
    for name, kind, lo, hi in parts():
        if lo[0] < SPLIT:
            solid = box_mesh(name, lo, (min(hi[0], SPLIT), hi[1], hi[2]), mats[kind], into)
            bevel = solid.modifiers.new("Bevel", "BEVEL")
            bevel.width = BEVEL * MM
            bevel.segments = 3
        if hi[0] > SPLIT and not name.endswith("post"):
            wire_box(name + " wire", (max(lo[0], SPLIT), lo[1], lo[2]), hi, wire, into)

    floor = box_mesh("Floor", (-3000, -3000, -10), (4000, 4000, 0), plain("Floor", (0x80, 0x80, 0x80), 0.5), into)
    floor.is_shadow_catcher = True

    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.8, 0.77, 0.72, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.28
    sc.world = world

    centre = (W / 2 * MM, D / 2 * MM - 0.05, H / 2 * MM + 0.02)
    area("Key", (-1.0, -2.2, 2.4), centre, 1.6, 130, (1.0, 0.95, 0.88))
    area("Fill", (2.4, -2.0, 1.1), centre, 2.5, 20, (0.95, 0.96, 1.0))

    cam = bpy.data.cameras.new("Camera")
    cam.lens = 80
    cam.sensor_width = 36
    ob = bpy.data.objects.new("Camera", cam)
    ob.location = (centre[0] - 1.75, centre[1] - 3.6, centre[2] + 0.95)
    ob.rotation_euler = (Vector(centre) - ob.location).to_track_quat("-Z", "Y").to_euler()
    into.objects.link(ob)
    sc.camera = ob

    sc.render.engine = "CYCLES"
    prefs = bpy.context.preferences.addons["cycles"].preferences
    prefs.compute_device_type = "METAL"
    prefs.get_devices()
    for device in prefs.devices:
        device.use = True
    sc.cycles.device = "GPU"
    sc.cycles.use_denoising = True
    sc.render.film_transparent = True
    sc.render.resolution_x = sc.render.resolution_y = SIZE
    sc.render.image_settings.file_format = "PNG"
    sc.render.image_settings.color_mode = "RGBA"
    sc.view_settings.view_transform = "Standard"


def box_blur(a, r):
    for axis in (0, 1):
        a = sum(np.roll(a, s, axis) for s in range(-r, r + 1)) / (2 * r + 1)
    return a


def compose(render_path, out):
    """The render on macOS's rounded square, darkest at the foot."""
    img = bpy.data.images.load(render_path)
    px = np.array(img.pixels[:], dtype=np.float32).reshape(SIZE, SIZE, 4)
    bpy.data.images.remove(img)
    # image.pixels runs from the bottom row, as this does.
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(np.float32) + 0.5
    half = BODY / 2
    # A superellipse is the shape's look; the small band softens its edge.
    d = (np.abs(xx - SIZE / 2) / half) ** 5 + (np.abs(yy - SIZE / 2) / half) ** 5
    mask = np.clip((1 - d) * half / 5 + 0.5, 0, 1)
    t = (yy / SIZE)[..., None]
    bg = (np.array(BOTTOM) * (1 - t) + np.array(TOP) * t) / 255
    # A soft pool of light behind the cabinet.
    glow = np.exp(-(((xx - SIZE / 2) / 330) ** 2 + ((yy - SIZE * 0.5) / 300) ** 2))[..., None]
    bg = np.clip(bg + glow * 0.07, 0, 1)
    a = px[..., 3:4]
    rgb = px[..., :3] * a + bg * (1 - a)
    shadow = box_blur(np.roll(mask, -12, 0), 16) * 0.42
    alpha = mask + shadow * (1 - mask)
    colour = rgb * mask[..., None] / np.maximum(alpha[..., None], 1e-4)
    done = np.dstack([np.clip(colour, 0, 1), alpha])
    image = bpy.data.images.new("icon", SIZE, SIZE, alpha=True)
    image.pixels.foreach_set(done.astype(np.float32).ravel())
    image.filepath_raw = out
    image.file_format = "PNG"
    image.save()


def main():
    args = sys.argv[sys.argv.index("--") + 1:]
    samples = int(args[args.index("--samples") + 1]) if "--samples" in args else 512
    install = "--install" in args
    out = "apps/macos/CabinetStudio/Assets.xcassets/AppIcon.appiconset/icon_512x512@2x.png" if install else args[0]
    build()
    sc = bpy.context.scene
    sc.cycles.samples = samples
    raw = os.path.abspath(out) + ".render.png"
    sc.render.filepath = raw
    bpy.ops.render.render(write_still=True)
    compose(raw, os.path.abspath(out))
    os.remove(raw)
    if install:
        folder = os.path.dirname(out)
        for size in (16, 32, 128, 256, 512):
            for scale, suffix in ((1, ""), (2, "@2x")):
                if size * scale != SIZE:
                    target = os.path.join(folder, f"icon_{size}x{size}{suffix}.png")
                    subprocess.run(["/usr/bin/sips", "-Z", str(size * scale), out, "--out", target], capture_output=True, check=True)


main()

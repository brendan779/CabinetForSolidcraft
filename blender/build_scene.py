"""Makes a job's scene in Blender: the cabinets solidcraft-cli built, in Polytec's decors, in a
lit kitchen. Run by Cabinet Studio, or by hand:

  Blender -b -P blender/build_scene.py -- JOB.json JOB.obj OUT.blend CACHE_DIR [--render OUT.png]
  Blender -b -P blender/build_scene.py -- --prepare CACHE_DIR

Bodies are matched to the manifest by name, so each part takes the material of its board. A decor's
image comes from Polytec the first time it's needed and its height map is made from it; both stay in
CACHE_DIR.
"""
import json
import math
import os
import subprocess
import sys

import bpy
import numpy as np
from mathutils import Vector

MM = 0.001
HEIGHT_VERSION = "h1"
WARM = (1.0, 0.76, 0.54)  # about 3000 K: kitchen downlights
DAYLIGHT = (0.86, 0.92, 1.0)


# ---- Decor images and their height maps --------------------------------------------------------

def fetch(url, path):
    # curl for the system's certificates, whichever Python this Blender has.
    part = path + ".part"
    agent = "CabinetStudio/1.0 (decor images for a cabinet tool)"
    done = subprocess.run(["/usr/bin/curl", "-sSfL", "--max-time", "60", "-A", agent, "-o", part, url])
    if done.returncode != 0:
        return False
    os.replace(part, path)
    return True


def blur(a, sigma):
    h, w = a.shape
    fy = np.fft.fftfreq(h)[:, None]
    fx = np.fft.rfftfreq(w)[None, :]
    return np.fft.irfft2(np.fft.rfft2(a) * np.exp(-2 * (math.pi * sigma) ** 2 * (fx * fx + fy * fy)), s=a.shape)


def make_height(src, dst):
    """Fine relief from the decor's own picture: the panel's colour drift is not relief, and dark
    grain reads as low. A plain colour keeps its small contrast rather than having noise raised."""
    im = bpy.data.images.load(src)
    w, h = im.size
    px = np.empty(w * h * 4, dtype=np.float32)
    im.pixels.foreach_get(px)
    bpy.data.images.remove(im)
    px = px.reshape(h, w, 4)
    lum = px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722
    detail = lum - blur(lum, 24)
    scale = max(float(np.percentile(np.abs(detail), 99.5)), 0.04)
    height = np.clip(detail / scale, -1, 1) * 0.5 + 0.5
    out = bpy.data.images.new("height", w, h, alpha=False)
    rgba = np.ones((h, w, 4), dtype=np.float32)
    rgba[..., 0] = rgba[..., 1] = rgba[..., 2] = height
    out.pixels.foreach_set(rgba.ravel())
    out.filepath_raw = dst
    out.file_format = "PNG"
    out.save()
    bpy.data.images.remove(out)


def height_path(cache, name):
    return os.path.join(cache, f"{name}-{HEIGHT_VERSION}.png")


def ensure_maps(cache, board):
    """The image and height map of a board's decor, or None: it then shows its plain colour."""
    name = board.get("image")
    if not name:
        return None
    os.makedirs(cache, exist_ok=True)
    src = os.path.join(cache, name + ".jpg")
    if not os.path.exists(src) and not fetch(board["url"], src):
        print(f"no image for {name}: plain colour")
        return None
    height = height_path(cache, name)
    if not os.path.exists(height):
        make_height(src, height)
    return src, height


# ---- Materials ---------------------------------------------------------------------------------

def hex_colour(text, fallback=(0.8, 0.78, 0.72)):
    if not text:
        return fallback
    text = text.lstrip("#")
    srgb = [int(text[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in srgb)


def principled(name, colour, roughness, metallic=0.0, coat=0.0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*colour, 1)
    b.inputs["Roughness"].default_value = roughness
    b.inputs["Metallic"].default_value = metallic
    b.inputs["Coat Weight"].default_value = coat
    b.inputs["IOR"].default_value = 1.5
    return m, b


def soften_edges(m, b, into=None):
    # Boards aren't knife-edged: edge tape catches the light.
    bevel = m.node_tree.nodes.new("ShaderNodeBevel")
    bevel.samples = 4
    bevel.inputs["Radius"].default_value = 0.0008
    if into is None:
        m.node_tree.links.new(bevel.outputs[0], b.inputs["Normal"])
    else:
        m.node_tree.links.new(bevel.outputs[0], into)


def board_material(board_id, board, cache):
    label = f'{board.get("decor") or board_id} {board.get("finish") or ""}'.strip()
    look = board["look"]
    m, b = principled(label, hex_colour(board.get("colour")), look["roughness"], coat=look["coat"])
    if look["coat"] > 0:
        b.inputs["Coat Roughness"].default_value = 0.08
    soft = True
    maps = ensure_maps(cache, board)
    landscape = False
    if maps:
        n = m.node_tree
        uv = n.nodes.new("ShaderNodeUVMap")
        uv.uv_map = "panel"

        def texture(path, data):
            t = n.nodes.new("ShaderNodeTexImage")
            t.image = bpy.data.images.load(path, check_existing=True)
            t.extension = "REPEAT"
            t.interpolation = "Cubic"
            if data:
                t.image.colorspace_settings.name = "Non-Color"
            n.links.new(uv.outputs[0], t.inputs[0])
            return t

        base = texture(maps[0], False)
        landscape = base.image.size[0] > base.image.size[1]
        n.links.new(base.outputs[0], b.inputs["Base Color"])
        height = texture(maps[1], True)
        bump = n.nodes.new("ShaderNodeBump")
        bump.inputs["Strength"].default_value = look["bump"]
        bump.inputs["Distance"].default_value = 0.0006
        n.links.new(height.outputs[0], bump.inputs["Height"])
        n.links.new(bump.outputs[0], b.inputs["Normal"])
        # Grain is duller than the surface around it.
        rough = n.nodes.new("ShaderNodeMapRange")
        rough.inputs["From Min"].default_value = 0.0
        rough.inputs["From Max"].default_value = 1.0
        rough.inputs["To Min"].default_value = min(look["roughness"] + look["bump"] * 0.18, 1.0)
        rough.inputs["To Max"].default_value = max(look["roughness"] - look["bump"] * 0.06, 0.05)
        n.links.new(height.outputs[0], rough.inputs["Value"])
        n.links.new(rough.outputs[0], b.inputs["Roughness"])
        soften_edges(m, b, bump.inputs["Normal"])
        soft = False
    if soft:
        soften_edges(m, b)
    return m, landscape


def add_uvs(obj, part, sheet, landscape, seed):
    """Each face mapped by its plane, the sheet's length along the part's grain, from its own place
    on the sheet, at the sheet's true size."""
    me = obj.data
    layer = me.uv_layers.new(name="panel")
    axis_of = {"x": 0, "y": 1, "z": 2}
    grain = axis_of.get(part.get("grain"))
    lo, hi = part["min"], part["max"]
    size = [hi[i] - lo[i] for i in range(3)]
    length, width = sheet
    offset = ((seed * 0.6180339) % 1, (seed * 0.7548776) % 1)
    world = obj.matrix_world
    for poly in me.polygons:
        normal = poly.normal
        facing = max(range(3), key=lambda k: abs(normal[k]))
        plane = [k for k in range(3) if k != facing]
        along = grain if grain in plane else max(plane, key=lambda k: size[k])
        across = plane[0] if plane[1] == along else plane[1]
        for li in poly.loop_indices:
            co = world @ me.vertices[me.loops[li].vertex_index].co
            u = co[across] / MM / width + offset[0]
            v = co[along] / MM / length + offset[1]
            layer.data[li].uv = (v, u) if landscape else (u, v)


# ---- The room ----------------------------------------------------------------------------------

def collection(name):
    c = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(c)
    return c


def box(name, lo, hi, material, into):
    """A box between two corners, in metres, with its faces out."""
    v = [(lo[0] if i & 1 == 0 else hi[0], lo[1] if i & 2 == 0 else hi[1], lo[2] if i & 4 == 0 else hi[2]) for i in range(8)]
    f = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    me = bpy.data.meshes.new(name)
    me.from_pydata(v, [], f)
    me.update()
    me.materials.append(material)
    ob = bpy.data.objects.new(name, me)
    into.objects.link(ob)
    return ob


def disc(name, x, y, z, r, material, into):
    """A flat circle under the ceiling, facing down."""
    n = 32
    v = [(x + r * math.cos(-2 * math.pi * i / n), y + r * math.sin(-2 * math.pi * i / n), z) for i in range(n)]
    me = bpy.data.meshes.new(name)
    me.from_pydata(v, [], [list(range(n))])
    me.update()
    me.materials.append(material)
    ob = bpy.data.objects.new(name, me)
    into.objects.link(ob)
    return ob


def floor_material():
    m, b = principled("Floor oak boards", (0.2, 0.14, 0.09), 0.42)
    n = m.node_tree
    coord = n.nodes.new("ShaderNodeTexCoord")
    bricks = n.nodes.new("ShaderNodeTexBrick")
    bricks.offset = 0.5
    bricks.inputs["Color1"].default_value = (0.2, 0.145, 0.095, 1)
    bricks.inputs["Color2"].default_value = (0.155, 0.108, 0.07, 1)
    bricks.inputs["Mortar"].default_value = (0.06, 0.04, 0.025, 1)
    bricks.inputs["Scale"].default_value = 1.0
    bricks.inputs["Mortar Size"].default_value = 0.0012
    bricks.inputs["Brick Width"].default_value = 1.8
    bricks.inputs["Row Height"].default_value = 0.14
    n.links.new(coord.outputs["Object"], bricks.inputs["Vector"])
    noise = n.nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 40
    noise.inputs["Detail"].default_value = 8
    mapping = n.nodes.new("ShaderNodeMapping")
    mapping.inputs["Scale"].default_value = (0.08, 1.0, 1.0)
    n.links.new(coord.outputs["Object"], mapping.inputs["Vector"])
    n.links.new(mapping.outputs[0], noise.inputs["Vector"])
    mix = n.nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    mix.blend_type = "MULTIPLY"
    mix.inputs[0].default_value = 0.55
    n.links.new(bricks.outputs[0], mix.inputs[6])
    n.links.new(noise.outputs[0], mix.inputs[7])
    n.links.new(mix.outputs[2], b.inputs["Base Color"])
    return m


def emitter(name, colour, strength):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    n = m.node_tree
    for node in list(n.nodes):
        if node.type != "OUTPUT_MATERIAL":
            n.nodes.remove(node)
    e = n.nodes.new("ShaderNodeEmission")
    e.inputs[0].default_value = (*colour, 1)
    e.inputs[1].default_value = strength
    n.links.new(e.outputs[0], n.nodes["Material Output"].inputs[0])
    return m


def light(name, kind, at, into, colour, energy, aim=None, **settings):
    data = bpy.data.lights.new(name, kind)
    data.color = colour
    data.energy = energy
    for key, value in settings.items():
        setattr(data, key, value)
    ob = bpy.data.objects.new(name, data)
    ob.location = at
    if aim is not None:
        ob.rotation_euler = (Vector(aim) - Vector(at)).to_track_quat("-Z", "Y").to_euler()
    into.objects.link(ob)
    return ob


BENCH_KINDS = {"base", "drawer_bank", "sink", "blind_corner", "corner_base"}
WALL_KINDS = {"wall", "open_shelf", "corner_wall"}


def build_room(job, lo, hi):
    """Walls, floor and ceiling round the run, with its lights. Numbers are metres, in the cabinets'
    frame: X across the front, Y to the back wall, Z up."""
    room = collection("Room")
    lamps = collection("Lights")
    walls = job.get("walls") or {}
    x0, x1 = lo.x - 1.2, hi.x + 1.2
    if walls.get("left"):
        x0 = lo.x
    if walls.get("right"):
        x1 = hi.x
    if walls.get("length"):
        x1 = x0 + walls["length"] * MM
    y1 = hi.y
    z1 = max((walls.get("height") or 2700) * MM, hi.z + 0.1)
    # Far enough back to see the whole run through a 28 mm lens.
    span = x1 - x0
    fov = 2 * math.atan(18 / 28)
    dist = max((hi.x - lo.x + 0.8) / 2 / math.tan(fov / 2), 2.6)
    y0 = y1 - dist - hi.y - 0.8
    t = 0.1
    paint, _ = principled("Wall paint", (0.82, 0.8, 0.75), 0.75)
    ceiling, _ = principled("Ceiling paint", (0.86, 0.86, 0.84), 0.85)
    trim, _ = principled("Skirting", (0.84, 0.83, 0.8), 0.4)
    floor = floor_material()
    box("Floor", (x0 - t, y0 - t, -t), (x1 + t, y1 + t, 0), floor, room)
    box("Ceiling", (x0 - t, y0 - t, z1), (x1 + t, y1 + t, z1 + t), ceiling, room)
    box("Back wall", (x0 - t, y1, 0), (x1 + t, y1 + t, z1), paint, room)
    box("Left wall", (x0 - t, y0, 0), (x0, y1, z1), paint, room)
    box("Right wall", (x1, y0, 0), (x1 + t, y1, z1), paint, room)
    box("Wall behind the camera", (x0, y0 - t, 0), (x1, y0, z1), paint, room)
    box("Skirting left", (x0, y0, 0), (x0 + 0.018, y1, 0.09), trim, room)
    box("Skirting right", (x1 - 0.018, y0, 0), (x1, y1, 0.09), trim, room)

    # Daylight from a window in the wall the room has more of, in front of the cabinets.
    side_left = (lo.x - x0) >= (x1 - hi.x)
    wx = x0 if side_left else x1
    sign = 1 if side_left else -1
    wy0, wy1 = max(y0 + 0.5, -2.8), -0.7
    wz0, wz1 = 0.95, min(2.3, z1 - 0.25)
    glow = emitter("Window daylight", DAYLIGHT, 6)
    box("Window", (wx - 0.002, wy0, wz0), (wx + 0.002, wy1, wz1), glow, room)
    frame, _ = principled("Window frame", (0.05, 0.05, 0.05), 0.4)
    bar = 0.05
    for name, a, b in [
        ("head", (wy0 - bar, wz1), (wy1 + bar, wz1 + bar)),
        ("sill", (wy0 - bar, wz0 - bar), (wy1 + bar, wz0)),
        ("jamb near", (wy0 - bar, wz0), (wy0, wz1)),
        ("jamb far", (wy1, wz0), (wy1 + bar, wz1)),
        ("mullion", ((wy0 + wy1) / 2 - bar / 2, wz0), ((wy0 + wy1) / 2 + bar / 2, wz1)),
    ]:
        xa, xb = sorted((wx, wx + sign * 0.05))
        box(f"Window {name}", (xa, a[0], a[1]), (xb, b[0], b[1]), frame, room)
    light(
        "Window light", "AREA", (wx + sign * 0.06, (wy0 + wy1) / 2, (wz0 + wz1) / 2), lamps, DAYLIGHT,
        220, aim=((lo.x + hi.x) / 2, 0.0, 0.9), shape="RECTANGLE", size=wy1 - wy0, size_y=wz1 - wz0,
    )

    # Recessed downlights: a row over the bench, a row over the floor.
    rim, _ = principled("Downlight rim", (0.8, 0.8, 0.8), 0.4)
    glow = emitter("Downlight", WARM, 40)
    n = max(2, round(span / 1.2))
    for row, y in enumerate((-0.35, -1.7)):
        if y <= y0 + 0.3:
            continue
        for i in range(n):
            x = x0 + span * (i + 0.5) / n
            disc(f"Downlight rim {row}.{i}", x, y, z1 - 0.001, 0.05, rim, room)
            disc(f"Downlight {row}.{i}", x, y, z1 - 0.002, 0.038, glow, room)
            light(
                f"Downlight {row}.{i}", "SPOT", (x, y, z1 - 0.01), lamps, WARM, 26,
                aim=(x, y, 0), spot_size=math.radians(70), spot_blend=0.9, shadow_soft_size=0.03,
            )

    # Stone on the base run for the look of a kitchen, and strips under the wall cabinets.
    stone, stone_b = principled("Benchtop stone", (0.78, 0.77, 0.74), 0.22, coat=0.2)
    has_top = any(p["role"] == "Benchtop" for p in job["parts"])
    runs = []
    for c in sorted(job["cabinets"], key=lambda c: c["min"][0]):
        if c["kind"] in BENCH_KINDS:
            if runs and abs(runs[-1]["max"][0] - c["min"][0]) < 1:
                runs[-1]["min"][1] = min(runs[-1]["min"][1], c["min"][1])
                runs[-1]["max"] = [c["max"][0], max(runs[-1]["max"][1], c["max"][1]), max(runs[-1]["max"][2], c["max"][2])]
            else:
                runs.append({"min": list(c["min"]), "max": list(c["max"])})
    if not has_top:
        for i, r in enumerate(runs):
            box(
                f"Benchtop {i} (scene only)",
                (r["min"][0] * MM, (r["min"][1] - 20) * MM, r["max"][2] * MM),
                (r["max"][0] * MM, y1, (r["max"][2] + 30) * MM),
                stone, room,
            )
    strip = emitter("Under cabinet strip", WARM, 12)
    for c in job["cabinets"]:
        if c["kind"] in WALL_KINDS:
            a, b = c["min"], c["max"]
            y = (a[1] + 60) * MM
            box(f'Strip {c["id"]}', ((a[0] + 25) * MM, y - 0.006, a[2] * MM - 0.002), ((b[0] - 25) * MM, y + 0.006, a[2] * MM), strip, room)
            light(
                f'Strip {c["id"]}', "AREA", ((a[0] + b[0]) / 2 * MM, y, a[2] * MM - 0.004), lamps, WARM,
                14 * (b[0] - a[0]) * MM, shape="RECTANGLE", size=(b[0] - a[0] - 50) * MM, size_y=0.012,
            )

    # The room's own glow, so nothing falls to black.
    world = bpy.data.worlds.new("Room")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.9, 0.92, 1.0, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.12
    bpy.context.scene.world = world

    # A little off centre, at eye height: how you'd stand to look at a kitchen.
    data = bpy.data.cameras.new("Camera")
    data.lens = 28
    data.sensor_width = 36
    data.clip_start = 0.05
    cam = bpy.data.objects.new("Camera", data)
    centre = (lo.x + hi.x) / 2
    at = Vector((centre - 0.14 * (hi.x - lo.x), y1 - 0.6 - dist, 1.5))
    cam.location = at
    cam.rotation_euler = (Vector((centre, 0.15, 1.15)) - at).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam


def build(job_path, obj_path, blend_path, cache, render):
    job = json.load(open(job_path))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.wm.obj_import(filepath=obj_path, forward_axis="Y", up_axis="Z", global_scale=MM)
    bodies = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    cabinets = collection("Cabinets")
    parts = {p["name"]: p for p in job["parts"]}
    materials = {}
    unknown = 0
    for i, ob in enumerate(bodies):
        for c in list(ob.users_collection):
            c.objects.unlink(ob)
        cabinets.objects.link(ob)
        part = parts.get(ob.name) or parts.get(ob.name.rsplit(".", 1)[0])
        if part is None:
            # Supplier hardware: keep the colour SolidCraft gave it, as metal, smooth where curved.
            unknown += 1
            for slot in ob.material_slots:
                if slot.material and slot.material.use_nodes:
                    b = slot.material.node_tree.nodes.get("Principled BSDF")
                    if b:
                        b.inputs["Metallic"].default_value = 1.0
                        b.inputs["Roughness"].default_value = 0.32
            ob.data.shade_smooth()
            ob.data.set_sharp_from_angle(angle=math.radians(35))
            continue
        key = part["board"] or ("front" if part["front"] else "carcass")
        if key not in materials:
            if part["board"]:
                materials[key] = board_material(key, job["boards"][key], cache)
            else:
                look = job["default_look"]
                m, b = principled(f"Generic {key}", (0.8, 0.77, 0.7), look["roughness"])
                soften_edges(m, b)
                materials[key] = (m, False)
        material, landscape = materials[key]
        ob.data.materials.clear()
        ob.data.materials.append(material)
        board = job["boards"].get(part["board"]) or {}
        add_uvs(ob, part, board.get("sheet", [2400, 1200]), landscape, i + 1)
        ob.data.shade_flat()
    pts = [o.matrix_world @ Vector(c) for o in bodies for c in o.bound_box]
    lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    build_room(job, lo, hi)

    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    prefs = bpy.context.preferences.addons["cycles"].preferences
    if prefs.compute_device_type == "NONE":
        prefs.compute_device_type = "METAL"
        bpy.ops.wm.save_userpref()
    prefs.get_devices()
    for d in prefs.devices:
        d.use = True
    sc.cycles.device = "GPU"
    sc.cycles.samples = 256
    sc.cycles.use_denoising = True
    sc.cycles.max_bounces = 8
    sc.render.resolution_x = 1920
    sc.render.resolution_y = 1280
    sc.view_settings.view_transform = "AgX"
    sc.view_settings.look = "AgX - Medium High Contrast"
    sc.render.image_settings.file_format = "PNG"
    # The file stands alone, so it can be moved or sent.
    bpy.ops.file.pack_all()
    os.makedirs(os.path.dirname(blend_path), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=blend_path)
    print(f"scene: {len(bodies)} bodies, {len(materials)} materials, {unknown} hardware")
    if render:
        sc.render.filepath = render
        bpy.ops.render.render(write_still=True)


def main():
    args = sys.argv[sys.argv.index("--") + 1:]
    if args[0] == "--prepare":
        cache = args[1]
        for f in sorted(os.listdir(cache)):
            if f.endswith(".jpg"):
                dst = height_path(cache, f[:-4])
                if not os.path.exists(dst):
                    make_height(os.path.join(cache, f), dst)
                    print(f, flush=True)
        return
    render = args[args.index("--render") + 1] if "--render" in args else None
    build(args[0], args[1], args[2], args[3], render)


main()

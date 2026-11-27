"""Checks that need Blender, run by crates/cabinet-ffi/tests/blender.rs."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build_scene import MM, slab_uvs, smooth_routed


def exported_groove():
    """A curved prism as the export writes it: triangles only, the diagonals of one facet the
    opposite way to the next, and a separate copy of the vertices for every other facet."""
    ring = [(0.01 * math.sin(i * 0.3), i * 0.002) for i in range(11)]
    verts, faces = [], []
    for i in range(10):
        a, b = ring[i], ring[i + 1]
        n = len(verts)
        verts += [(a[0], a[1], 0), (b[0], b[1], 0), (b[0], b[1], 0.87), (a[0], a[1], 0.87)]
        faces += [(n, n + 1, n + 2), (n, n + 2, n + 3)] if i % 2 else [(n, n + 1, n + 3), (n + 1, n + 2, n + 3)]
    me = bpy.data.meshes.new("groove")
    me.from_pydata(verts, [], faces)
    me.update()
    ob = bpy.data.objects.new("groove", me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def a_routed_face_shades_the_same_at_both_ends_of_a_groove():
    ob = exported_groove()
    smooth_routed(ob)
    me = ob.data
    assert len(me.vertices) == 22, f"{len(me.vertices)} vertices: facets not welded"
    assert len(me.polygons) == 10, f"{len(me.polygons)} faces: facets not joined"
    ends = {}
    for v in me.vertices:
        ends.setdefault((round(v.co.x, 5), round(v.co.y, 5)), []).append(v)
    for pair in ends.values():
        low, high = pair
        drift = low.normal.angle(high.normal)
        assert drift < math.radians(0.01), f"{math.degrees(drift):.2f} degrees along the groove"


def a_real_edge_on_a_routed_face_stays_sharp():
    me = bpy.data.meshes.new("corner")
    me.from_pydata(
        [(0, 0, 0), (0.01, 0, 0), (0.01, 0, 1), (0, 0, 1), (0.01, 0.01, 0), (0.01, 0.01, 1)],
        [],
        [(0, 1, 2), (0, 2, 3), (1, 4, 5), (1, 5, 2)],
    )
    me.update()
    ob = bpy.data.objects.new("corner", me)
    bpy.context.scene.collection.objects.link(ob)
    smooth_routed(ob)
    assert any(e.use_edge_sharp for e in ob.data.edges), "the corner went soft"


def box(name, lo, hi):
    """A cabinet-frame box (mm), as an object in metres."""
    x0, y0, z0 = (v * MM for v in lo)
    x1, y1, z1 = (v * MM for v in hi)
    verts = [(x, y, z) for z in (z0, z1) for y in (y0, y1) for x in (x0, x1)]
    faces = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    me.update()
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def a_joined_top_starts_a_new_slab_past_its_join():
    """BENCH-19: a 4500 top on 3200 slabs, joined at 2700: the faces split there, and the piece
    past the join starts its slab again."""
    ob = box("top", (0, 0, 870), (4500, 620, 890))
    slab_uvs(ob, {"slab": {"s": ["x", 1, 0], "t": ["y", 1, 0], "breaks": [2700]}}, [3200, 1600], False)
    me, uv = ob.data, ob.data.uv_layers["panel"]
    upper = [p for p in me.polygons if p.normal.z > 0.9]
    assert len(upper) == 2, f"{len(upper)} upper faces: not split at the join"
    for p in upper:
        vs = [uv.data[li].uv[1] for li in p.loop_indices]
        assert min(vs) > -1e-6 and max(vs) <= 2700 / 3200 + 1e-6, f"a piece off its slab: {vs}"


def a_waterfall_meets_the_top_on_the_same_place_on_the_slab():
    """A left waterfall rises into the top: at their shared edge they take the same slab point."""
    top = box("bt", (-20, 0, 870), (2000, 620, 890))
    fall = box("wf", (-20, 0, 0), (0, 620, 890))
    slab_uvs(top, {"slab": {"s": ["x", 1, -20 - 890], "t": ["y", 1, 0], "breaks": []}}, [3200, 1600], False)
    slab_uvs(fall, {"slab": {"s": ["z", 1, 0], "t": ["y", 1, 0], "breaks": []}}, [3200, 1600], False)

    def at(ob, x, z):
        uv = ob.data.uv_layers["panel"]
        for li, loop in enumerate(ob.data.loops):
            co = ob.data.vertices[loop.vertex_index].co / MM
            if abs(co.x - x) < 1e-3 and abs(co.z - z) < 1e-3 and abs(co.y) < 1e-3:
                return tuple(round(c, 5) for c in uv.data[li].uv)

    assert at(top, -20, 890) == at(fall, -20, 890), (at(top, -20, 890), at(fall, -20, 890))


a_routed_face_shades_the_same_at_both_ends_of_a_groove()
a_real_edge_on_a_routed_face_stays_sharp()
a_joined_top_starts_a_new_slab_past_its_join()
a_waterfall_meets_the_top_on_the_same_place_on_the_slab()
print("scene checks passed")

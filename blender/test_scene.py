"""Checks that need Blender, run by crates/cabinet-ffi/tests/blender.rs."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build_scene import smooth_routed


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


a_routed_face_shades_the_same_at_both_ends_of_a_groove()
a_real_edge_on_a_routed_face_stays_sharp()
print("scene checks passed")

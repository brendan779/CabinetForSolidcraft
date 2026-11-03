"""The import script's helpers, without Revit. Run by crates/cabinet-ffi/tests/revit.rs."""
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from import_into_revit import grain_is_upright, material_of, parse_mtl, parse_obj, picture_size

BOARD = {"decor": "Oak", "finish": "Matt", "grained": True, "image": "oak", "pixels": [1500, 3000],
         "sheet": [3600.0, 1800.0], "look": {"roughness": 0.4}, "colour": "#806040"}
BODY = {"name": "B01.Door1", "material": "B01_Door1"}


def an_objs_quads_become_triangles_and_each_body_keeps_its_material():
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "a.obj")
        open(p, "w").write("o A.B\nusemtl A_B\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 3//1 4//1\n")
        verts, bodies = parse_obj(p)
        assert len(verts) == 4
        assert bodies == [{"name": "A.B", "material": "A_B", "triangles": [(0, 1, 2), (0, 2, 3)]}]
        m = os.path.join(d, "a.mtl")
        open(m, "w").write("newmtl A_B\nKd 0.5 0.25 1\n")
        assert parse_mtl(m) == {"A_B": (0.5, 0.25, 1.0)}


def grain_up_a_side_or_door_is_upright_and_along_a_drawer_front_is_not():
    assert grain_is_upright({"grain": "z", "thickness": "y"})
    assert grain_is_upright({"grain": "y", "thickness": "z"})
    assert not grain_is_upright({"grain": "x", "thickness": "y"})
    assert not grain_is_upright({"grain": "y", "thickness": "x"})


def a_picture_takes_its_real_size_from_the_sheet_and_its_own_shape():
    assert picture_size(BOARD) == ((1800.0, 3600.0), True)
    assert picture_size({**BOARD, "pixels": [3000, 1500]}) == ((3600.0, 1800.0), False)
    assert picture_size({**BOARD, "pixels": [960, 960], "sheet": [1300.0, 1300.0]}) == ((1300.0, 1300.0), True)


def a_drawer_front_gets_the_picture_turned_in_a_material_of_its_own():
    door = material_of(BODY, {"board": "oak", "grain": "z", "thickness": "y"}, {"oak": BOARD}, {})
    drawer = material_of(BODY, {"board": "oak", "grain": "x", "thickness": "y"}, {"oak": BOARD}, {})
    assert door["angle"] == 0.0 and drawer["angle"] == 90.0
    assert door["key"] != drawer["key"] and door["name"] != drawer["name"]


def an_ungrained_board_is_never_turned():
    plain = {**BOARD, "grained": False}
    a = material_of(BODY, {"board": "oak", "grain": "x", "thickness": "y"}, {"oak": plain}, {})
    assert a["angle"] == 0.0 and "across" not in a["key"]


def two_thicknesses_of_a_decor_share_a_material():
    part = {"grain": "z", "thickness": "y"}
    thin = material_of(BODY, {**part, "board": "oak.16"}, {"oak.16": BOARD}, {})
    thick = material_of(BODY, {**part, "board": "oak.18"}, {"oak.18": BOARD}, {})
    assert thin["key"] == thick["key"]


def paint_wins_over_the_board_and_hardware_keeps_its_colour():
    painted = material_of(
        BODY, {"board": "oak", "paint": {"name": "Rose", "finish": "Low sheen", "colour": "#AA5566", "roughness": 0.3}},
        {"oak": BOARD}, {},
    )
    assert painted["colour"] == (0xAA, 0x55, 0x66) and "image" not in painted
    pull = material_of({"name": "B01.Pull", "material": "m"}, None, {}, {"m": (1.0, 0.0, 0.0)})
    assert pull["colour"] == (255, 0, 0) and pull["metal"]


an_objs_quads_become_triangles_and_each_body_keeps_its_material()
grain_up_a_side_or_door_is_upright_and_along_a_drawer_front_is_not()
a_picture_takes_its_real_size_from_the_sheet_and_its_own_shape()
a_drawer_front_gets_the_picture_turned_in_a_material_of_its_own()
an_ungrained_board_is_never_turned()
two_thicknesses_of_a_decor_share_a_material()
paint_wins_over_the_board_and_hardware_keeps_its_colour()
print("import checks passed")
